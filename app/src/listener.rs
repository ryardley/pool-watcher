//! Alloy listener: HTTP backfill plus WS subscribe, grouped by block and
//! forwarded to the scanner client.
//!
//! The listener is gated on the scanner being initialized: the web `init`
//! handler calls `Listener::start`, which releases the gate and runs the
//! backfill/subscribe loop. It reads the proxy address and deployment block
//! from the config, backfills the full history with `eth_getLogs`, then keeps
//! a live `eth_subscribe` stream alive. Logs are grouped by block into
//! `BlockLogs` batches, each block's timestamp is fetched for the Beancount
//! date, and the returned events are handed to the ledger writer.

use crate::config::Config;
use crate::ledger::Ledger;
use crate::scanner::ScannerClient;
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::{Filter, Log};
use alloy::transports::http::reqwest;
use alloy::transports::ws::WsConnect;
use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use railgun_scanner_handle::{BlockLogs, RawLog};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::mpsc;

/// How long to wait before reconnecting the WS subscription after it drops.
const RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Cloneable handle to the listener task. `start` releases the gate so the
/// backfill/subscribe loop begins.
#[derive(Clone)]
pub struct Listener {
    start_tx: mpsc::Sender<()>,
}

impl Listener {
    /// Spawn the listener task. It waits for `start` before doing anything.
    pub fn spawn(config: Config, scanner: ScannerClient, ledger: Ledger) -> Self {
        let (start_tx, mut start_rx) = mpsc::channel(1);
        tokio::spawn(async move {
            // The scanner must be initialized before the listener can feed it
            // logs; wait for the web `init` handler to release the gate.
            if start_rx.recv().await.is_none() {
                return;
            }
            run(config, scanner, ledger).await;
        });
        Listener { start_tx }
    }

    /// Release the gate: start the backfill/subscribe loop. Idempotent.
    pub async fn start(&self) {
        let _ = self.start_tx.send(()).await;
    }
}

/// Backfill the full history, then keep the live subscription alive
/// (reconnecting on drops). The ledger is written after every batch so Fava
/// reloads on the rename.
async fn run(config: Config, scanner: ScannerClient, mut ledger: Ledger) {
    println!(
        "listener: started (chain_id={}, proxy={}, deployment_block={})",
        config.chain_id, config.deployment.proxy, config.deployment.deployment_block
    );
    let url: reqwest::Url = match config.rpc_url.parse() {
        Ok(url) => url,
        Err(err) => {
            eprintln!("listener: invalid RPC_URL {}: {err}", config.rpc_url);
            return;
        }
    };
    let http = ProviderBuilder::new().connect_http(url);

    if let Err(err) = backfill(&http, &config, &scanner, &mut ledger).await {
        eprintln!("listener: backfill failed: {err:#}");
    }

    loop {
        match subscribe_loop(&config, &http, &scanner, &mut ledger).await {
            Ok(()) => eprintln!("listener: subscription ended, reconnecting"),
            Err(err) => eprintln!("listener: subscription error: {err:#}"),
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// `eth_getLogs` from `deploymentBlock` to latest, filtered by the proxy, then
/// batch by block and forward to the scanner in one `scan`. Events are added to
/// the ledger keyed by their block's timestamp.
async fn backfill(
    http: &impl Provider,
    config: &Config,
    scanner: &ScannerClient,
    ledger: &mut Ledger,
) -> Result<()> {
    let proxy: Address = config
        .deployment
        .proxy
        .parse()
        .context("invalid proxy address")?;
    let filter = Filter::new()
        .address(proxy)
        .from_block(config.deployment.deployment_block)
        .to_block(BlockNumberOrTag::Latest);
    let logs = http.get_logs(&filter).await.context("eth_getLogs")?;
    let by_block = group_by_block(logs);
    if by_block.is_empty() {
        return Ok(());
    }

    let mut blocks = Vec::new();
    let mut timestamps = BTreeMap::new();
    for (block, logs) in &by_block {
        let timestamp = fetch_timestamp(http, *block).await?;
        timestamps.insert(*block, timestamp);
        blocks.push(BlockLogs {
            chain_id: config.chain_id,
            logs: logs.iter().map(log_to_raw).collect(),
        });
    }

    let events = scanner.scan(blocks).await.context("scan")?;
    for event in &events {
        if let Some(timestamp) = timestamps.get(&event.block_number) {
            ledger.add(event, &date_from_timestamp(*timestamp));
        }
    }
    ledger.write(&config.ledger_path).context("write ledger")?;
    println!(
        "listener: backfill: {} blocks, {} logs, {} events",
        by_block.len(),
        by_block.values().map(Vec::len).sum::<usize>(),
        events.len(),
    );
    Ok(())
}

/// Keep a live `eth_subscribe` logs stream. Logs are grouped by block and each
/// completed block is flushed to the scanner when the next block begins.
async fn subscribe_loop(
    config: &Config,
    http: &impl Provider,
    scanner: &ScannerClient,
    ledger: &mut Ledger,
) -> Result<()> {
    let ws = ProviderBuilder::new()
        .connect_ws(WsConnect::new(&config.ws_url))
        .await
        .with_context(|| format!("cannot connect ws {}", config.ws_url))?;
    let proxy: Address = config
        .deployment
        .proxy
        .parse()
        .context("invalid proxy address")?;
    let sub = ws
        .subscribe_logs(&Filter::new().address(proxy))
        .await
        .context("eth_subscribe logs")?;
    println!("listener: subscribed to new logs on {}", config.ws_url);

    let mut stream = sub.into_stream();
    let mut buffer = BlockBuffer::new();
    while let Some(log) = stream.next().await {
        if log.removed {
            continue;
        }
        if let Some((block, logs)) = buffer.push(log) {
            flush_block(http, scanner, ledger, config, block, &logs).await?;
        }
    }
    if let Some((block, logs)) = buffer.take() {
        flush_block(http, scanner, ledger, config, block, &logs).await?;
    }
    Ok(())
}

/// Send one block's logs to the scanner, fetch the block's timestamp for the
/// ledger date, and rewrite the ledger.
async fn flush_block(
    http: &impl Provider,
    scanner: &ScannerClient,
    ledger: &mut Ledger,
    config: &Config,
    block: u64,
    logs: &[Log],
) -> Result<()> {
    let timestamp = fetch_timestamp(http, block).await?;
    let batch = BlockLogs {
        chain_id: config.chain_id,
        logs: logs.iter().map(log_to_raw).collect(),
    };
    let events = scanner.scan(vec![batch]).await.context("scan")?;
    let date = date_from_timestamp(timestamp);
    for event in &events {
        ledger.add(event, &date);
    }
    ledger.write(&config.ledger_path).context("write ledger")?;
    println!(
        "listener: block {block}: {} logs, {} events",
        logs.len(),
        events.len(),
    );
    Ok(())
}

/// `eth_getBlockByNumber` for the block's Unix timestamp (the Beancount date).
async fn fetch_timestamp<P: Provider>(provider: &P, block: u64) -> Result<u64> {
    let block = provider
        .get_block_by_number(BlockNumberOrTag::Number(block))
        .await
        .context("eth_getBlockByNumber")?
        .ok_or_else(|| anyhow!("block {block} not found"))?;
    Ok(block.header.timestamp)
}

/// Group raw logs by block number, sorted ascending.
fn group_by_block(logs: Vec<Log>) -> BTreeMap<u64, Vec<Log>> {
    let mut by_block: BTreeMap<u64, Vec<Log>> = BTreeMap::new();
    for log in logs {
        by_block.entry(log.block_number.unwrap_or(0)).or_default().push(log);
    }
    by_block
}

/// Convert an alloy RPC log to the scanner's `RawLog` wire shape.
fn log_to_raw(log: &Log) -> RawLog {
    RawLog {
        address: log.address().to_string(),
        topics: log.topics().iter().map(|t| t.to_string()).collect(),
        data: log.data().data.to_string(),
        block_number: log.block_number.map(|n| serde_json::json!(n)),
        log_index: log.log_index.map(|n| serde_json::json!(n)),
        transaction_hash: log.transaction_hash.map(|h| h.to_string()),
    }
}

/// Unix seconds -> Beancount date (YYYY-MM-DD).
fn date_from_timestamp(timestamp: u64) -> String {
    chrono::DateTime::from_timestamp(timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

/// Buffers subscription logs per block and reports a block as complete once a
/// log from a later block arrives.
struct BlockBuffer {
    block: Option<u64>,
    logs: Vec<Log>,
}

impl BlockBuffer {
    fn new() -> Self {
        Self { block: None, logs: Vec::new() }
    }

    /// Feed a log. If it opens a new block, the previous block's logs are
    /// returned (with their block number) so the caller can flush them.
    fn push(&mut self, log: Log) -> Option<(u64, Vec<Log>)> {
        let block = log.block_number.unwrap_or(0);
        let ready = match self.block {
            Some(current) if current != block => Some((current, std::mem::take(&mut self.logs))),
            _ => None,
        };
        self.block = Some(block);
        self.logs.push(log);
        ready
    }

    /// Flush any remaining buffered logs (end of stream).
    fn take(&mut self) -> Option<(u64, Vec<Log>)> {
        let block = self.block?;
        self.block = None;
        Some((block, std::mem::take(&mut self.logs)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner;
    use alloy::primitives::{Address, LogData, B256};
    use alloy::providers::mock::Asserter;
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    const PROXY: &str = "0x99dBE4AEa58E518C50a1c04aE9b48C9F6354612f";

    fn test_config(chain_id: u64) -> Config {
        Config {
            rpc_url: "http://localhost:8545".into(),
            ws_url: "ws://localhost:8545".into(),
            chain_id,
            scanner_socket: temp_socket("listener").into(),
            deployment_path: "/tmp/deployment.json".into(),
            ledger_path: std::env::temp_dir()
                .join(format!("listener-ledger-{}.beancount", std::process::id())),
            bind_addr: "127.0.0.1:0".into(),
            deployment: crate::config::Deployment {
                proxy: PROXY.into(),
                deployment_block: 131,
            },
        }
    }

    fn temp_socket(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("listener-test-{}-{name}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    /// A mock scanner speaking the NDJSON wire contract over a Unix socket.
    /// Each `scan` echoes a canned event so tests can observe the batch.
    struct MockScanner {
        socket: String,
        listener: Option<UnixListener>,
        scans: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl MockScanner {
        fn bind(name: &str) -> Self {
            let socket = temp_socket(name);
            let _ = std::fs::remove_file(&socket);
            let listener = UnixListener::bind(&socket).expect("bind mock socket");
            Self {
                socket,
                listener: Some(listener),
                scans: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }
        }

        fn serve(&self, stop: Arc<AtomicBool>) {
            let listener = self
                .listener
                .as_ref()
                .expect("listener")
                .try_clone()
                .expect("clone listener");
            let scans = self.scans.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => serve_connection(stream, &scans),
                        Err(_) => break,
                    }
                }
            });
        }

        fn scan_count(&self) -> usize {
            self.scans.load(Ordering::SeqCst)
        }
    }

    impl Drop for MockScanner {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.socket);
        }
    }

    fn serve_connection(stream: std::os::unix::net::UnixStream, scans: &AtomicUsize) {
        use std::io::{BufRead, BufReader, Write};
        let mut writer = stream.try_clone().expect("clone stream");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).expect("read request");
            if n == 0 {
                return;
            }
            let request: serde_json::Value = serde_json::from_str(&line).expect("parse request");
            if request["op"] == "scan" {
                let n = scans.fetch_add(1, Ordering::SeqCst);
                let blocks = request["blocks"].as_array().expect("blocks");
                let events: Vec<serde_json::Value> = blocks
                    .iter()
                    .enumerate()
                    .map(|(i, b)| {
                        let chain_id = b["chainId"].as_u64().unwrap_or(0);
                        serde_json::json!({
                            "chainId": chain_id,
                            "walletAddress": "0xWALLET",
                            "address": PROXY,
                            "blockNumber": 215 + i as u64,
                            "txHash": format!("0xtx{n}-{i}"),
                            "logIndex": 0,
                            "kind": "transact",
                            "hash": null,
                            "token": "TST",
                            "value": "10000000000000000000",
                            "fee": null,
                            "outputType": null,
                            "direction": "in",
                            "sender": null,
                            "receiver": null,
                            "memo": "test",
                        })
                    })
                    .collect();
                let response = serde_json::json!({ "op": "events", "events": events });
                writer.write_all(response.to_string().as_bytes()).expect("write");
                writer.write_all(b"\n").expect("newline");
                writer.flush().expect("flush");
            } else {
                let _ = writer.write_all(br#"{"op":"error","message":"unknown op"}"#);
                let _ = writer.write_all(b"\n");
                let _ = writer.flush();
            }
        }
    }

    fn sample_log(block: u64, index: u64) -> Log {
        Log::<LogData> {
            inner: alloy::primitives::Log {
                address: PROXY.parse::<Address>().expect("address"),
                data: LogData::new_unchecked(vec![B256::with_last_byte(0x01)], Default::default()),
            },
            block_hash: None,
            block_number: Some(block),
            block_timestamp: None,
            transaction_hash: Some(B256::with_last_byte(0x02)),
            transaction_index: Some(0),
            log_index: Some(index),
            removed: false,
        }
    }

    /// Minimal block JSON with a `timestamp` (hex quantity), plus every field
    /// the alloy `Block`/`Header` deserialization requires.
    fn block_json(number: u64, timestamp: u64) -> serde_json::Value {
        let hex32 = |byte: u8| format!("0x{}", format!("{byte:02x}").repeat(32));
        let zero = |n: usize| format!("0x{}", "00".repeat(n));
        serde_json::json!({
            "hash": hex32(0x11),
            "parentHash": hex32(0x22),
            "sha3Uncles": hex32(0x33),
            "miner": "0x0000000000000000000000000000000000000000",
            "stateRoot": hex32(0x44),
            "transactionsRoot": hex32(0x55),
            "receiptsRoot": hex32(0x66),
            "logsBloom": zero(256),
            "difficulty": "0x0",
            "number": format!("0x{number:x}"),
            "gasLimit": "0x0",
            "gasUsed": "0x0",
            "timestamp": format!("0x{timestamp:x}"),
            "extraData": "0x",
            "mixHash": hex32(0x77),
            "nonce": "0x0000000000000000",
            "uncles": [],
            "transactions": [],
            "withdrawals": [],
        })
    }

    #[test]
    fn group_by_block_sorts_and_groups() {
        let logs = vec![sample_log(216, 1), sample_log(215, 0), sample_log(215, 1)];
        let by_block = group_by_block(logs);
        let keys: Vec<u64> = by_block.keys().copied().collect();
        assert_eq!(keys, [215, 216]);
        assert_eq!(by_block[&215].len(), 2);
        assert_eq!(by_block[&216].len(), 1);
    }

    #[test]
    fn log_to_raw_matches_wire_shape() {
        let log = sample_log(215, 7);
        let raw = log_to_raw(&log);
        assert_eq!(raw.address, PROXY);
        assert_eq!(raw.topics.len(), 1);
        assert_eq!(raw.data, "0x");
        assert_eq!(raw.block_number, Some(serde_json::json!(215)));
        assert_eq!(raw.log_index, Some(serde_json::json!(7)));
        assert!(raw.transaction_hash.is_some());
    }

    #[test]
    fn date_from_timestamp_formats_utc_date() {
        // 2026-11-06T00:00:00Z
        assert_eq!(date_from_timestamp(1_793_923_200), "2026-11-06");
        // 0 and negatives fall back safely.
        assert_eq!(date_from_timestamp(0), "1970-01-01");
    }

    #[test]
    fn block_buffer_flushes_previous_block_on_boundary() {
        let mut buffer = BlockBuffer::new();
        assert!(buffer.push(sample_log(215, 0)).is_none());
        assert!(buffer.push(sample_log(215, 1)).is_none());
        let (block, logs) = buffer.push(sample_log(216, 0)).expect("flush on boundary");
        assert_eq!(block, 215);
        assert_eq!(logs.len(), 2);
        let (block, logs) = buffer.take().expect("flush remainder");
        assert_eq!(block, 216);
        assert_eq!(logs.len(), 1);
    }

    /// End-to-end backfill: a mocked HTTP provider serves `eth_getLogs` and one
    /// `eth_getBlockByNumber` per block; a mock scanner returns events; the
    /// ledger must be written with the events and their block dates.
    #[tokio::test]
    async fn backfill_forwards_batches_and_writes_ledger() {
        let mock = MockScanner::bind("backfill");
        let mut config = test_config(31337);
        config.scanner_socket = mock.socket.clone().into();
        let _ = std::fs::remove_file(&config.ledger_path);

        let asserter = Asserter::new();
        let logs = vec![
            serde_json::json!({
                "address": PROXY,
                "topics": [B256::with_last_byte(0x01).to_string()],
                "data": "0x",
                "blockNumber": "0xd7", // 215
                "logIndex": "0x0",
                "transactionIndex": "0x0",
                "transactionHash": B256::with_last_byte(0x02).to_string(),
            }),
            serde_json::json!({
                "address": PROXY,
                "topics": [B256::with_last_byte(0x01).to_string()],
                "data": "0x",
                "blockNumber": "0xd8", // 216
                "logIndex": "0x0",
                "transactionIndex": "0x0",
                "transactionHash": B256::with_last_byte(0x03).to_string(),
            }),
        ];
        asserter.push_success(&logs);
        asserter.push_success(&block_json(215, 1_793_923_200));
        asserter.push_success(&block_json(216, 1_793_923_201));
        let http = ProviderBuilder::new().connect_mocked_client(asserter);

        let stop = Arc::new(AtomicBool::new(false));
        mock.serve(stop.clone());
        let client = scanner::spawn_with(config.scanner_socket.clone(), Duration::from_secs(2));

        let mut ledger = Ledger::new();
        backfill(&http, &config, &client, &mut ledger)
            .await
            .expect("backfill");

        assert_eq!(mock.scan_count(), 1, "one scan with all blocks batched");
        ledger.write(&config.ledger_path).expect("write");
        let text = std::fs::read_to_string(&config.ledger_path).expect("ledger file");
        // One event per block; both carry the "test" narration.
        assert_eq!(text.matches("test").count(), 2);
        assert!(text.contains("2026-11-06"), "ledger date from block timestamp");

        stop.store(true, Ordering::Relaxed);
        let _ = std::fs::remove_file(&config.ledger_path);
    }
}

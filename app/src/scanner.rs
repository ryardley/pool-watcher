//! Scanner client: a dedicated thread + mpsc channel so the synchronous
//! `railgun-scanner-handle` can be driven from async actix/alloy code.
//!
//! The thread owns the `Handle` and serializes every request through a single
//! channel, so `scan` calls are single-flight. It waits for the socket file to
//! appear (the scanner container may start after the app) and retries the
//! connect until it succeeds or a timeout elapses.

use crate::config::Config;
use anyhow::{anyhow, Result};
use railgun_scanner_handle::{BlockLogs, Event, Handle, HandleError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

/// Default time to keep retrying the connect before giving up.
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 30;
/// How long to sleep between connect attempts.
const RETRY_INTERVAL: Duration = Duration::from_millis(250);
/// Channel capacity for queued requests.
const CHANNEL_CAPACITY: usize = 16;

/// One request to the scanner thread. The reply is sent back over a oneshot.
// `pubkey`/`init`/`scan`/`ping` are consumed by the web and listener tickets;
// until then they are exercised by the tests below.
#[allow(dead_code)]
enum Request {
    Pubkey {
        reply: oneshot::Sender<Result<String>>,
    },
    Init {
        key_ciphertext: String,
        reply: oneshot::Sender<Result<String>>,
    },
    Scan {
        blocks: Vec<BlockLogs>,
        reply: oneshot::Sender<Result<Vec<Event>>>,
    },
    Ping {
        reply: oneshot::Sender<Result<()>>,
    },
}

impl Request {
    /// Reply with an error, regardless of the request's expected payload.
    fn send_error(self, err: anyhow::Error) {
        match self {
            Request::Pubkey { reply } => {
                let _ = reply.send(Err(err));
            }
            Request::Init { reply, .. } => {
                let _ = reply.send(Err(err));
            }
            Request::Scan { reply, .. } => {
                let _ = reply.send(Err(err));
            }
            Request::Ping { reply } => {
                let _ = reply.send(Err(err));
            }
        }
    }
}

/// Cloneable handle to the scanner client thread. Share it with async code.
#[derive(Clone)]
pub struct ScannerClient {
    tx: mpsc::Sender<Request>,
}

// `pubkey`/`init`/`scan`/`ping` are consumed by the web and listener tickets;
// until then they are exercised by the tests below.
#[allow(dead_code)]
impl ScannerClient {
    /// Fetch the scanner's X25519 public key (base64).
    pub async fn pubkey(&self) -> Result<String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Request::Pubkey { reply })
            .await
            .map_err(|_| anyhow!("scanner client thread is gone"))?;
        rx.await.map_err(|_| anyhow!("scanner client thread is gone"))?
    }

    /// Initialize the scanner with a sealed viewing key; returns the wallet
    /// address.
    pub async fn init(&self, key_ciphertext: String) -> Result<String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Request::Init {
                key_ciphertext,
                reply,
            })
            .await
            .map_err(|_| anyhow!("scanner client thread is gone"))?;
        rx.await.map_err(|_| anyhow!("scanner client thread is gone"))?
    }

    /// Scan a batch of blocks and return the decrypted events.
    pub async fn scan(&self, blocks: Vec<BlockLogs>) -> Result<Vec<Event>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Request::Scan { blocks, reply })
            .await
            .map_err(|_| anyhow!("scanner client thread is gone"))?;
        rx.await.map_err(|_| anyhow!("scanner client thread is gone"))?
    }

    /// Liveness check.
    pub async fn ping(&self) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Request::Ping { reply })
            .await
            .map_err(|_| anyhow!("scanner client thread is gone"))?;
        rx.await.map_err(|_| anyhow!("scanner client thread is gone"))?
    }
}

/// Spawn the scanner client thread and return a handle to it.
pub fn spawn(config: &Config) -> ScannerClient {
    let timeout = connect_timeout();
    spawn_with(config.scanner_socket.clone(), timeout)
}

/// Spawn the client thread with an explicit socket path and connect timeout
/// (used by tests and the web route tests).
pub(crate) fn spawn_with(socket: PathBuf, connect_timeout: Duration) -> ScannerClient {
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    std::thread::spawn(move || worker(socket, connect_timeout, rx));
    ScannerClient { tx }
}

/// Connect timeout from `SCANNER_CONNECT_TIMEOUT_SECS`, or the default.
fn connect_timeout() -> Duration {
    let secs = std::env::var("SCANNER_CONNECT_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CONNECT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// The client thread: owns the `Handle` and serves requests one at a time.
fn worker(socket: PathBuf, connect_timeout: Duration, mut rx: mpsc::Receiver<Request>) {
    let mut handle: Option<Handle> = None;
    while let Some(request) = rx.blocking_recv() {
        let dead = if let Some(h) = handle.as_mut() {
            serve(h, request)
        } else {
            match connect_with_retry(&socket, connect_timeout) {
                Ok(mut h) => {
                    let dead = serve(&mut h, request);
                    handle = Some(h);
                    dead
                }
                Err(err) => {
                    eprintln!("scanner client: {err}");
                    request.send_error(err);
                    false
                }
            }
        };
        // An io error means the connection died; drop it so the next request
        // reconnects.
        if dead {
            handle = None;
        }
    }
}

/// Serve one request on the handle. Returns true if the connection died (an
/// io error) and the handle should be dropped.
fn serve(h: &mut Handle, request: Request) -> bool {
    match request {
        Request::Pubkey { reply } => send_reply(reply, h.pubkey().map_err(anyhow::Error::from)),
        Request::Init {
            key_ciphertext,
            reply,
        } => {
            let result = h
                .init(&key_ciphertext)
                .map(|ready| ready.wallet_address)
                .map_err(anyhow::Error::from);
            send_reply(reply, result)
        }
        Request::Scan { blocks, reply } => {
            let result = h
                .scan(blocks)
                .map(|events| events.events)
                .map_err(anyhow::Error::from);
            send_reply(reply, result)
        }
        Request::Ping { reply } => {
            let result = h.ping().map(|_| ()).map_err(anyhow::Error::from);
            send_reply(reply, result)
        }
    }
}

/// Send a reply and report whether the underlying connection died.
fn send_reply<T>(reply: oneshot::Sender<Result<T>>, result: Result<T>) -> bool {
    let dead = matches!(
        result.as_ref().err().and_then(|e| e.downcast_ref::<HandleError>()),
        Some(HandleError::Io(_))
    );
    let _ = reply.send(result);
    dead
}

/// Retry `Handle::connect` until it succeeds or `timeout` elapses. The scanner
/// container may start after the app, so the socket file may not exist yet.
fn connect_with_retry(socket: &Path, timeout: Duration) -> Result<Handle> {
    let deadline = Instant::now() + timeout;
    loop {
        match Handle::connect(socket) {
            Ok(handle) => return Ok(handle),
            Err(err) => {
                if Instant::now() >= deadline {
                    return Err(anyhow!(
                        "scanner socket {} not reachable within {}s: {err}",
                        socket.display(),
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(RETRY_INTERVAL);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A mock scanner speaking the NDJSON wire contract over a Unix socket.
    /// Accepts connections until the listener is dropped; each connection is
    /// served line-by-line until the client closes it.
    struct MockScanner {
        socket: PathBuf,
        listener: Option<UnixListener>,
        scans: Arc<AtomicUsize>,
    }

    impl MockScanner {
        fn bind(socket: &Path) -> Self {
            let _ = std::fs::remove_file(socket);
            let listener = UnixListener::bind(socket).expect("bind mock socket");
            Self {
                socket: socket.to_path_buf(),
                listener: Some(listener),
                scans: Arc::new(AtomicUsize::new(0)),
            }
        }

        /// Serve connections on a background thread until `stop` is set.
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
            self.scans.load(Ordering::Relaxed)
        }
    }

    impl Drop for MockScanner {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.socket);
        }
    }

    fn serve_connection(stream: UnixStream, scans: &AtomicUsize) {
        let mut writer = stream.try_clone().expect("clone stream");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).expect("read request");
            if n == 0 {
                return;
            }
            let response = mock_response(&line, scans);
            writer
                .write_all(response.as_bytes())
                .expect("write response");
            writer.write_all(b"\n").expect("write newline");
            writer.flush().expect("flush");
        }
    }

    fn mock_response(line: &str, scans: &AtomicUsize) -> String {
        let request: serde_json::Value = serde_json::from_str(line).expect("parse request");
        match request["op"].as_str() {
            Some("pubkey") => r#"{"op":"pubkey","publicKey":"MOCKPUBKEY"}"#.to_string(),
            Some("init") => r#"{"op":"ready","walletAddress":"0xMOCKWALLET"}"#.to_string(),
            Some("scan") => {
                let n = scans.fetch_add(1, Ordering::SeqCst);
                let chain_id = request["blocks"]
                    .as_array()
                    .and_then(|blocks| blocks.first())
                    .and_then(|b| b["chainId"].as_u64())
                    .unwrap_or(0);
                serde_json::to_string(&serde_json::json!({
                    "op": "events",
                    "events": [{
                        "chainId": chain_id,
                        "walletAddress": "0xMOCKWALLET",
                        "address": "0xPROXY",
                        "blockNumber": 1,
                        "txHash": format!("0xtx{n}"),
                        "logIndex": 0,
                        "kind": "transact",
                        "hash": null,
                        "token": "TST",
                        "value": "1",
                        "fee": null,
                        "outputType": null,
                        "direction": "in",
                        "sender": null,
                        "receiver": null,
                        "memo": "test",
                    }]
                }))
                .expect("serialize events")
            }
            Some("ping") => r#"{"op":"pong"}"#.to_string(),
            _ => r#"{"op":"error","message":"unknown op"}"#.to_string(),
        }
    }

    fn temp_socket(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "scanner-client-test-{}-{name}.sock",
            std::process::id()
        ))
    }

    fn block_logs(chain_id: u64) -> Vec<BlockLogs> {
        vec![BlockLogs {
            chain_id,
            logs: vec![],
        }]
    }

    #[tokio::test]
    async fn ops_against_running_scanner() {
        let socket = temp_socket("ops");
        let mock = MockScanner::bind(&socket);
        let stop = Arc::new(AtomicBool::new(false));
        mock.serve(stop.clone());

        let client = spawn_with(socket, Duration::from_secs(2));

        let pubkey = client.pubkey().await.expect("pubkey");
        assert_eq!(pubkey, "MOCKPUBKEY");

        let wallet = client.init("ciphertext".to_string()).await.expect("init");
        assert_eq!(wallet, "0xMOCKWALLET");

        let events = client.scan(block_logs(31337)).await.expect("scan");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].chain_id, 31337);
        assert_eq!(events[0].tx_hash, "0xtx0");

        client.ping().await.expect("ping");

        stop.store(true, Ordering::Relaxed);
    }

    #[tokio::test]
    async fn connects_when_socket_appears_late() {
        let socket = temp_socket("late");
        let client = spawn_with(socket.clone(), Duration::from_secs(5));

        // The socket does not exist yet; the first request must retry until the
        // mock scanner binds it.
        std::thread::sleep(Duration::from_millis(200));
        let mock = MockScanner::bind(&socket);
        let stop = Arc::new(AtomicBool::new(false));
        mock.serve(stop.clone());

        let pubkey = client.pubkey().await.expect("pubkey after retry");
        assert_eq!(pubkey, "MOCKPUBKEY");

        stop.store(true, Ordering::Relaxed);
    }

    #[tokio::test]
    async fn missing_socket_fails_with_clear_error() {
        let socket = temp_socket("missing");
        let _ = std::fs::remove_file(&socket);
        let client = spawn_with(socket.clone(), Duration::from_millis(500));

        let err = client.pubkey().await.expect_err("pubkey must fail");
        let message = format!("{err:#}");
        assert!(
            message.contains("not reachable within"),
            "unexpected error: {message}"
        );
        assert!(message.contains("missing.sock"), "unexpected error: {message}");
    }

    #[tokio::test]
    async fn concurrent_scans_are_serialized() {
        let socket = temp_socket("serial");
        let mock = MockScanner::bind(&socket);
        let stop = Arc::new(AtomicBool::new(false));
        mock.serve(stop.clone());

        let client = spawn_with(socket, Duration::from_secs(2));

        // Fire many scans concurrently with distinct chain ids; each must get
        // back its own payload (no cross-talk), proving the channel serializes.
        let mut tasks = Vec::new();
        for i in 0..16u64 {
            let client = client.clone();
            tasks.push(tokio::spawn(async move {
                let events = client.scan(block_logs(1000 + i)).await.expect("scan");
                assert_eq!(events.len(), 1);
                assert_eq!(events[0].chain_id, 1000 + i);
            }));
        }
        for task in tasks {
            task.await.expect("task");
        }

        assert_eq!(mock.scan_count(), 16, "all scans reached the scanner");

        stop.store(true, Ordering::Relaxed);
    }
}

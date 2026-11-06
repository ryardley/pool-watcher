//! Sync client handle for the stateless Railgun scanner.
//!
//! The request/response structs in this crate are the source of truth for the
//! wire contract. The Node scanner mirrors them. Framing is NDJSON over a Unix
//! domain socket: one JSON object per line, one response line per request.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// A raw RPC log object, as returned by `eth_getLogs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    #[serde(rename = "blockNumber", default, skip_serializing_if = "Option::is_none")]
    pub block_number: Option<serde_json::Value>,
    #[serde(rename = "logIndex", default, skip_serializing_if = "Option::is_none")]
    pub log_index: Option<serde_json::Value>,
    #[serde(
        rename = "transactionHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub transaction_hash: Option<String>,
}

/// Logs for a single block, tagged with the chain (pool) they belong to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockLogs {
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    pub logs: Vec<RawLog>,
}

/// One decrypted note/commitment (or a public unshield) belonging to the wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,
    pub address: String,
    #[serde(rename = "blockNumber")]
    pub block_number: u64,
    #[serde(rename = "txHash")]
    pub tx_hash: String,
    #[serde(rename = "logIndex")]
    pub log_index: u64,
    pub kind: String,
    pub hash: Option<String>,
    pub token: String,
    pub value: String,
    pub fee: Option<String>,
    #[serde(rename = "outputType")]
    pub output_type: Option<u64>,
    pub direction: String,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Init {
    pub op: &'static str,
    #[serde(rename = "keyCiphertext")]
    pub key_ciphertext: String,
}

impl Init {
    pub fn new(key_ciphertext: impl Into<String>) -> Self {
        Self {
            op: "init",
            key_ciphertext: key_ciphertext.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Scan {
    pub op: &'static str,
    pub blocks: Vec<BlockLogs>,
}

impl Scan {
    pub fn new(blocks: Vec<BlockLogs>) -> Self {
        Self { op: "scan", blocks }
    }
}

#[derive(Debug, Serialize)]
pub struct Ping {
    pub op: &'static str,
}

impl Default for Ping {
    fn default() -> Self {
        Self { op: "ping" }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ready {
    pub op: String,
    #[serde(rename = "walletAddress")]
    pub wallet_address: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Events {
    pub op: String,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pong {
    pub op: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Error {
    pub op: String,
    pub message: String,
}

/// Any response line from the scanner.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Response {
    Ready {
        #[serde(rename = "walletAddress")]
        wallet_address: String,
    },
    Events {
        events: Vec<Event>,
    },
    Pong,
    Error {
        message: String,
    },
}

#[derive(Debug)]
pub enum HandleError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Protocol(String),
}

impl std::fmt::Display for HandleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleError::Io(e) => write!(f, "io error: {e}"),
            HandleError::Json(e) => write!(f, "json error: {e}"),
            HandleError::Protocol(m) => write!(f, "protocol error: {m}"),
        }
    }
}

impl std::error::Error for HandleError {}

impl From<std::io::Error> for HandleError {
    fn from(e: std::io::Error) -> Self {
        HandleError::Io(e)
    }
}

impl From<serde_json::Error> for HandleError {
    fn from(e: serde_json::Error) -> Self {
        HandleError::Json(e)
    }
}

/// A synchronous connection to one scanner instance.
pub struct Handle {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl Handle {
    /// Connect to the scanner listening on `path`.
    pub fn connect(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        let writer = stream.try_clone()?;
        let reader = BufReader::new(stream);
        Ok(Self { writer, reader })
    }

    fn send_line(&mut self, line: &str) -> Result<(), HandleError> {
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }

    fn read_line(&mut self) -> Result<String, HandleError> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Err(HandleError::Protocol("connection closed".into()));
        }
        Ok(line.trim_end_matches(['\n', '\r']).to_string())
    }

    /// Send an already-serialized request line and read the raw response line.
    pub fn send_raw(&mut self, line: &str) -> Result<String, HandleError> {
        self.send_line(line)?;
        self.read_line()
    }

    /// Send a request and parse the response.
    pub fn request(&mut self, request: &impl Serialize) -> Result<Response, HandleError> {
        let line = serde_json::to_string(request)?;
        let response = self.send_raw(&line)?;
        Ok(serde_json::from_str(&response)?)
    }

    /// Initialize the scanner with an encrypted viewing key.
    pub fn init(&mut self, key_ciphertext: &str) -> Result<Ready, HandleError> {
        match self.request(&Init::new(key_ciphertext))? {
            Response::Ready { wallet_address } => Ok(Ready {
                op: "ready".into(),
                wallet_address,
            }),
            Response::Error { message } => Err(HandleError::Protocol(message)),
            other => Err(HandleError::Protocol(format!(
                "unexpected response to init: {other:?}"
            ))),
        }
    }

    /// Scan a batch of blocks and return the decrypted events.
    pub fn scan(&mut self, blocks: Vec<BlockLogs>) -> Result<Events, HandleError> {
        match self.request(&Scan::new(blocks))? {
            Response::Events { events } => Ok(Events {
                op: "events".into(),
                events,
            }),
            Response::Error { message } => Err(HandleError::Protocol(message)),
            other => Err(HandleError::Protocol(format!(
                "unexpected response to scan: {other:?}"
            ))),
        }
    }

    /// Liveness check.
    pub fn ping(&mut self) -> Result<Pong, HandleError> {
        match self.request(&Ping::default())? {
            Response::Pong => Ok(Pong { op: "pong".into() }),
            Response::Error { message } => Err(HandleError::Protocol(message)),
            other => Err(HandleError::Protocol(format!(
                "unexpected response to ping: {other:?}"
            ))),
        }
    }
}

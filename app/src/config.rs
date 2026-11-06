//! Configuration for the accounting demo app.
//!
//! Reads the test bed's deployment file (`.state/deployment.json`) and the
//! env vars documented in `docs/app/spec.md`.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};

/// Subset of `.state/deployment.json` the app needs.
#[derive(Debug, Clone, Deserialize)]
pub struct Deployment {
    pub proxy: String,
    #[serde(rename = "deploymentBlock")]
    pub deployment_block: u64,
}

impl Deployment {
    pub fn from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| {
            format!(
                "deployment file {} is missing or unreadable",
                path.display()
            )
        })?;
        serde_json::from_str(&text).with_context(|| {
            format!(
                "deployment file {} is not valid JSON with proxy + deploymentBlock",
                path.display()
            )
        })
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub ws_url: String,
    pub chain_id: u64,
    pub scanner_socket: PathBuf,
    pub deployment_path: PathBuf,
    pub ledger_path: PathBuf,
    pub bind_addr: String,
    pub deployment: Deployment,
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn load() -> Result<Self> {
        let deployment_path =
            PathBuf::from(env_or("DEPLOYMENT_PATH", "/data/.state/deployment.json"));
        let deployment = Deployment::from_path(&deployment_path)?;
        let chain_id = env_or("CHAIN_ID", "31337")
            .parse()
            .context("CHAIN_ID must be a u64")?;
        Ok(Self {
            rpc_url: env_or("RPC_URL", "http://host.docker.internal:8545"),
            ws_url: env_or("WS_URL", "ws://host.docker.internal:8545"),
            chain_id,
            scanner_socket: PathBuf::from(env_or("SCANNER_SOCKET", "/run/scanner/scanner-b.sock")),
            deployment_path,
            ledger_path: PathBuf::from(env_or("LEDGER_PATH", "/ledger/ledger.beancount")),
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8080"),
            deployment,
        })
    }
}

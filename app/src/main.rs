//! Entry point: parse config, spawn the scanner client, start the actix-web
//! server and the alloy listener.

mod config;
mod ledger;
mod listener;
mod scanner;
mod web;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::Config::load()?;
    println!(
        "config: rpc={} ws={} chain_id={} scanner={} deployment={} ledger={} bind={} proxy={} deployment_block={}",
        config.rpc_url,
        config.ws_url,
        config.chain_id,
        config.scanner_socket.display(),
        config.deployment_path.display(),
        config.ledger_path.display(),
        config.bind_addr,
        config.deployment.proxy,
        config.deployment.deployment_block,
    );
    scanner::spawn(&config);
    let ledger = ledger::Ledger::new();
    ledger.write(&config.ledger_path)?;
    listener::spawn(&config, ledger);
    web::serve(config).await
}

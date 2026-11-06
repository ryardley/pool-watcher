//! Alloy listener: HTTP backfill plus WS subscribe, grouped by block and
//! forwarded to the scanner client.
//!
//! The listener is gated on the scanner being initialized: the web `init`
//! handler calls `Listener::start`, which releases the gate and runs the
//! backfill/subscribe loop. The alloy logic itself is a later ticket; this
//! module owns the start signal and the task lifecycle.

use crate::config::Config;
use crate::ledger::Ledger;
use tokio::sync::mpsc;

/// Cloneable handle to the listener task. `start` releases the gate so the
/// backfill/subscribe loop begins.
#[derive(Clone)]
pub struct Listener {
    start_tx: mpsc::Sender<()>,
}

impl Listener {
    /// Spawn the listener task. It waits for `start` before doing anything.
    pub fn spawn(config: Config, ledger: Ledger) -> Self {
        let (start_tx, mut start_rx) = mpsc::channel(1);
        tokio::spawn(async move {
            // The scanner must be initialized before the listener can feed it
            // logs; wait for the web `init` handler to release the gate.
            if start_rx.recv().await.is_none() {
                return;
            }
            run(config, ledger).await;
        });
        Listener { start_tx }
    }

    /// Release the gate: start the backfill/subscribe loop. Idempotent.
    pub async fn start(&self) {
        let _ = self.start_tx.send(()).await;
    }
}

/// The backfill + subscribe loop. The alloy implementation lands with the
/// listener ticket; until then the task stays alive so the wiring is real.
async fn run(config: Config, _ledger: Ledger) {
    println!(
        "listener: started (chain_id={}, proxy={}, deployment_block={})",
        config.chain_id, config.deployment.proxy, config.deployment.deployment_block
    );
    std::future::pending::<()>().await
}

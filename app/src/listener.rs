//! Alloy listener: HTTP backfill plus WS subscribe, grouped by block and
//! forwarded to the scanner client.

use crate::config::Config;
use crate::ledger::Ledger;

pub fn spawn(_config: &Config, _ledger: Ledger) {}

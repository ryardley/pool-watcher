//! Scanner client: a dedicated thread + mpsc channel so the synchronous
//! `railgun-scanner-handle` can be driven from async actix/alloy code.

use crate::config::Config;

pub fn spawn(_config: &Config) {}

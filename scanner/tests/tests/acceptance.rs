//! End-to-end acceptance: consume logs fetched for real from the running chain
//! (regenerated into `out/logs.json` by `scripts/acceptance.sh`), drive the
//! scanner via the Rust handle, and assert the events reconstruct
//! `out/steps.jsonl` and the balances arithmetic.

use std::fs;

use railgun_scanner_handle::{Event, Handle};
use scanner_protocol_tests::{
    key_ciphertext, load_fixture, load_keys, repo_root, ScannerProcess,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Step {
    kind: String,
    #[serde(rename = "txHash")]
    tx_hash: String,
    block: u64,
    from: String,
    to: String,
    amount: String,
    fee: Option<String>,
    memo: Option<String>,
}

fn load_steps() -> Vec<Step> {
    let path = repo_root().join("out/steps.jsonl");
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    data.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("parse step"))
        .collect()
}

fn scan(name: &str) -> (String, Vec<Event>) {
    let keys = load_keys();
    let fixture = load_fixture();
    let entry = &keys[name];
    let scanner = ScannerProcess::spawn();
    let mut handle = Handle::connect(scanner.socket()).expect("connect");
    let ready = handle
        .init(&key_ciphertext(&entry.shareable_viewing_key))
        .expect("init");
    assert_eq!(ready.wallet_address, entry.railgun_address);
    let events = handle.scan(fixture.blocks).expect("scan").events;
    (entry.railgun_address.clone(), events)
}

fn balance(events: &[Event]) -> i128 {
    let mut total: i128 = 0;
    for event in events {
        let value: i128 = event.value.parse().expect("integer value");
        total += if event.direction == "in" { value } else { -value };
    }
    total
}

fn find<'a>(events: &'a [Event], tx_hash: &str, kind: &str, direction: &str) -> &'a Event {
    events
        .iter()
        .find(|e| e.tx_hash == tx_hash && e.kind == kind && e.direction == direction)
        .unwrap_or_else(|| panic!("no {kind}/{direction} event for tx {tx_hash}"))
}

fn find_transfer<'a>(
    events: &'a [Event],
    tx_hash: &str,
    direction: &str,
    value: &str,
) -> &'a Event {
    events
        .iter()
        .find(|e| {
            e.tx_hash == tx_hash
                && e.kind == "transact"
                && e.direction == direction
                && e.value == value
        })
        .unwrap_or_else(|| {
            panic!("no transact/{direction} event for tx {tx_hash} value {value}")
        })
}

#[test]
fn acceptance_reconstructs_steps_and_balances() {
    let steps = load_steps();
    let keys = load_keys();
    let a_addr = keys["A"].railgun_address.clone();
    let b_addr = keys["B"].railgun_address.clone();

    let (a_wallet, a_events) = scan("A");
    assert_eq!(a_wallet, a_addr);
    let (b_wallet, b_events) = scan("B");
    assert_eq!(b_wallet, b_addr);

    let mut a_expected: i128 = 0;
    let mut b_expected: i128 = 0;

    for step in &steps {
        let amount: i128 = step.amount.parse().expect("amount");
        let fee: i128 = step.fee.as_deref().unwrap_or("0").parse().expect("fee");
        match step.kind.as_str() {
            "shield" => {
                assert_eq!(step.to, a_addr, "shield is to A");
                let event = find(&a_events, &step.tx_hash, "shield", "in");
                assert_eq!(event.block_number, step.block);
                assert_eq!(event.receiver.as_deref(), Some(a_addr.as_str()));
                // The note value is the shield amount minus the contract fee.
                assert_eq!(event.value, (amount - fee).to_string());
                a_expected += amount - fee;
            }
            "transfer" => {
                if step.from == a_addr {
                    let event = find_transfer(&a_events, &step.tx_hash, "out", &step.amount);
                    assert_eq!(event.block_number, step.block);
                    assert_eq!(event.memo, step.memo);
                    assert_eq!(event.sender.as_deref(), Some(a_addr.as_str()));
                    assert_eq!(event.receiver.as_deref(), Some(step.to.as_str()));
                    a_expected -= amount;
                }
                if step.to == b_addr {
                    let event = find_transfer(&b_events, &step.tx_hash, "in", &step.amount);
                    assert_eq!(event.block_number, step.block);
                    assert_eq!(event.memo, step.memo);
                    assert_eq!(event.receiver.as_deref(), Some(b_addr.as_str()));
                    b_expected += amount;
                }
            }
            "unshield" => {
                assert_eq!(step.from, b_addr, "unshield is from B");
                let event = find(&b_events, &step.tx_hash, "unshield", "out");
                assert_eq!(event.block_number, step.block);
                assert_eq!(event.receiver.as_deref(), Some(step.to.as_str()));
                // The note value leaving the private balance is the gross amount.
                assert_eq!(event.value, step.amount);
                assert_eq!(event.fee, step.fee);
                b_expected -= amount;
            }
            other => panic!("unknown step kind: {other}"),
        }
    }

    // The events' in/out arithmetic matches the scenario balances.
    assert_eq!(balance(&a_events), a_expected);
    assert_eq!(balance(&b_events), b_expected);
    assert_eq!(a_expected, 762_500_000_000_000_000_000);
    assert_eq!(b_expected, 185_000_000_000_000_000_000);

    // Sanity: the fixture really came from the chain, not a stale hand-crafted file.
    let fixture = load_fixture();
    let log_count: usize = fixture.blocks.iter().map(|b| b.logs.len()).sum();
    assert!(log_count >= 8, "expected at least 8 railgun logs, got {log_count}");
}

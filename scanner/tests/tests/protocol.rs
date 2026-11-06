//! Cross-language protocol tests: spawn the real Node scanner and drive it with
//! the Rust handle. These exercise the wire contract, not Node agreeing with Node.

use railgun_scanner_handle::{Event, Handle, HandleError};
use scanner_protocol_tests::{
    key_ciphertext, load_fixture, load_keys, ScannerProcess,
};

const EOA2: &str = "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC";

fn balance(events: &[Event]) -> i128 {
    let mut total: i128 = 0;
    for event in events {
        let value: i128 = event.value.parse().expect("value is an integer");
        if event.direction == "in" {
            total += value;
        } else {
            total -= value;
        }
    }
    total
}

fn transfers<'a>(events: &'a [Event], direction: &str) -> Vec<&'a Event> {
    events
        .iter()
        .filter(|e| {
            e.kind == "transact" && e.direction == direction && e.memo.is_some()
        })
        .collect()
}

#[test]
fn init_scan_ping_for_wallet_a() {
    let keys = load_keys();
    let fixture = load_fixture();
    let a = &keys["A"];

    let scanner = ScannerProcess::spawn();
    let mut handle = Handle::connect(scanner.socket()).expect("connect");

    let ready = handle
        .init(&key_ciphertext(&a.shareable_viewing_key))
        .expect("init");
    assert_eq!(ready.op, "ready");
    assert_eq!(ready.wallet_address, a.railgun_address);

    let events = handle.scan(fixture.blocks.clone()).expect("scan").events;

    // Linkage is echoed into every event.
    for event in &events {
        assert_eq!(event.chain_id, 31337);
        assert_eq!(event.wallet_address, a.railgun_address);
        assert_eq!(event.address, "0x99dBE4AEa58E518C50a1c04aE9b48C9F6354612f");
    }

    // Shield of 1000 (minus 2.5 fee) to A.
    let shield = events
        .iter()
        .find(|e| e.kind == "shield")
        .expect("shield event");
    assert_eq!(shield.direction, "in");
    assert_eq!(shield.value, "997500000000000000000");
    assert!(shield.fee.is_none(), "shield fee must be null");
    assert_eq!(shield.receiver.as_deref(), Some(a.railgun_address.as_str()));

    // Five A->B transfers, with memos and sender visibility.
    let outs = transfers(&events, "out");
    let values: Vec<&str> = outs.iter().map(|e| e.value.as_str()).collect();
    assert_eq!(
        values,
        vec![
            "10000000000000000000",
            "25000000000000000000",
            "40000000000000000000",
            "60000000000000000000",
            "100000000000000000000",
        ]
    );
    let memos: Vec<&str> = outs.iter().map(|e| e.memo.as_deref().unwrap_or("")).collect();
    assert_eq!(
        memos,
        vec!["transfer #0", "transfer #1", "transfer #2", "transfer #3", "transfer #4"]
    );
    for event in &outs {
        assert_eq!(event.sender.as_deref(), Some(a.railgun_address.as_str()));
        assert_eq!(event.receiver.as_deref(), Some(keys["B"].railgun_address.as_str()));
        assert!(event.fee.is_none(), "transact fee must be null");
    }

    // B's unshield is not A's: it is attributed to the wallet that can decrypt
    // the change note in the same transaction.
    assert!(
        events.iter().all(|e| e.kind != "unshield"),
        "A must not see B's unshield"
    );

    // Balance arithmetic: shield minus the five transfers.
    assert_eq!(balance(&events), 762_500_000_000_000_000_000);

    let pong = handle.ping().expect("ping");
    assert_eq!(pong.op, "pong");
}

#[test]
fn scan_for_wallet_b() {
    let keys = load_keys();
    let fixture = load_fixture();
    let b = &keys["B"];

    let scanner = ScannerProcess::spawn();
    let mut handle = Handle::connect(scanner.socket()).expect("connect");
    let ready = handle
        .init(&key_ciphertext(&b.shareable_viewing_key))
        .expect("init");
    assert_eq!(ready.wallet_address, b.railgun_address);

    let events = handle.scan(fixture.blocks.clone()).expect("scan").events;

    // Five received transfers with memos. Sender is visible only for the
    // transfers that opted in (indices 0, 2, 4).
    let ins = transfers(&events, "in");
    let values: Vec<&str> = ins.iter().map(|e| e.value.as_str()).collect();
    assert_eq!(
        values,
        vec![
            "10000000000000000000",
            "25000000000000000000",
            "40000000000000000000",
            "60000000000000000000",
            "100000000000000000000",
        ]
    );
    let memos: Vec<&str> = ins.iter().map(|e| e.memo.as_deref().unwrap_or("")).collect();
    assert_eq!(
        memos,
        vec!["transfer #0", "transfer #1", "transfer #2", "transfer #3", "transfer #4"]
    );
    let senders: Vec<Option<&str>> = ins.iter().map(|e| e.sender.as_deref()).collect();
    assert_eq!(
        senders,
        vec![
            Some(keys["A"].railgun_address.as_str()),
            None,
            Some(keys["A"].railgun_address.as_str()),
            None,
            Some(keys["A"].railgun_address.as_str()),
        ]
    );
    assert!(ins.iter().all(|e| e.fee.is_none()), "transact fee must be null");

    let unshield = events
        .iter()
        .find(|e| e.kind == "unshield")
        .expect("unshield event");
    assert_eq!(unshield.direction, "out");
    assert_eq!(unshield.value, "50000000000000000000");
    assert_eq!(unshield.fee.as_deref(), Some("125000000000000000"));
    assert_eq!(unshield.receiver.as_deref(), Some(EOA2));

    // Balance arithmetic: received minus unshielded.
    assert_eq!(balance(&events), 185_000_000_000_000_000_000);
}

#[test]
fn scan_before_init_errors() {
    let fixture = load_fixture();
    let scanner = ScannerProcess::spawn();
    let mut handle = Handle::connect(scanner.socket()).expect("connect");
    let err = handle.scan(fixture.blocks).expect_err("scan before init");
    match err {
        HandleError::Protocol(message) => assert!(message.contains("not initialized")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn init_twice_errors() {
    let keys = load_keys();
    let scanner = ScannerProcess::spawn();
    let mut handle = Handle::connect(scanner.socket()).expect("connect");
    handle
        .init(&key_ciphertext(&keys["A"].shareable_viewing_key))
        .expect("first init");
    let err = handle
        .init(&key_ciphertext(&keys["A"].shareable_viewing_key))
        .expect_err("second init");
    match err {
        HandleError::Protocol(message) => assert!(message.contains("already initialized")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn malformed_json_errors() {
    let scanner = ScannerProcess::spawn();
    let mut handle = Handle::connect(scanner.socket()).expect("connect");
    let response = handle.send_raw("{not json").expect("send raw");
    assert!(response.contains("\"op\":\"error\""), "response: {response}");
}

#[test]
fn unknown_op_errors() {
    let scanner = ScannerProcess::spawn();
    let mut handle = Handle::connect(scanner.socket()).expect("connect");
    let response = handle
        .send_raw(r#"{"op":"frobnicate"}"#)
        .expect("send raw");
    assert!(response.contains("\"op\":\"error\""), "response: {response}");
}

#[test]
fn single_flight_queues_second_client() {
    let keys = load_keys();
    let scanner = ScannerProcess::spawn();

    let mut first = Handle::connect(scanner.socket()).expect("connect first");
    first
        .init(&key_ciphertext(&keys["A"].shareable_viewing_key))
        .expect("init");

    // Second client connects while the first is still active; it is queued.
    let mut second = Handle::connect(scanner.socket()).expect("connect second");

    // The first client is still served.
    assert_eq!(first.ping().expect("first ping").op, "pong");

    // Closing the first client lets the queued second client be served.
    drop(first);
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(second.ping().expect("second ping").op, "pong");
}

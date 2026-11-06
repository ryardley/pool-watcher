//! Beancount ledger writer: events -> Beancount text, deduped, atomic rewrite.
//!
//! The merchant model for wallet B: received notes with a product memo are
//! sales, the unshield is a withdrawal to the bank (net of fee) with the fee
//! posted to expenses, and change notes / sent notes are excluded.

use bigdecimal::{BigDecimal, Zero};
use railgun_scanner_handle::Event;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const COMMODITY: &str = "TST";
const DECIMALS: u32 = 18;

const ACCOUNT_RAILGUN: &str = "Assets:Railgun:B";
const ACCOUNT_SALES: &str = "Income:Sales";
const ACCOUNT_BANK: &str = "Assets:Bank:EOA2";
const ACCOUNT_FEES: &str = "Expenses:Fees";

const ACCOUNTS: [&str; 4] = [
    ACCOUNT_RAILGUN,
    ACCOUNT_SALES,
    ACCOUNT_BANK,
    ACCOUNT_FEES,
];

/// One Beancount transaction.
struct Entry {
    date: String,
    narration: String,
    metadata: Vec<(String, String)>,
    postings: Vec<(String, String)>,
}

/// In-memory ledger: deduped events, rendered on demand.
pub struct Ledger {
    entries: Vec<Entry>,
    seen: HashSet<String>,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

// `add` and the mapping helpers are consumed by the listener (a later ticket);
// until then they are exercised by the tests below.
#[allow(dead_code)]
impl Ledger {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            seen: HashSet::new(),
        }
    }

    /// Add an event, deduped by `txHash:logIndex:hash:direction` (unshield:
    /// `txHash:logIndex:unshield`). `date` is the Beancount date (YYYY-MM-DD)
    /// for the event's block, fetched by the listener.
    pub fn add(&mut self, event: &Event, date: &str) {
        let key = dedup_key(event);
        if !self.seen.insert(key) {
            return;
        }
        if let Some(entry) = entry_from_event(event, date) {
            self.entries.push(entry);
        }
    }

    /// Render the full Beancount document.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("option \"operating_currency\" \"TST\"\n\n");

        if let Some(date) = self.entries.iter().map(|e| e.date.as_str()).min() {
            for account in ACCOUNTS {
                let _ = writeln!(out, "{date} open {account} {COMMODITY}");
            }
            out.push('\n');
        }

        for entry in &self.entries {
            let _ = writeln!(out, "{} * \"{}\"", entry.date, escape(entry.narration.as_str()));
            for (key, value) in &entry.metadata {
                let _ = writeln!(out, "  {key}: {value}");
            }
            for (account, amount) in &entry.postings {
                let _ = writeln!(out, "  {account}  {amount}");
            }
            out.push('\n');
        }
        out
    }

    /// Atomically write the ledger: write `<path>.tmp`, then rename over
    /// `path` so Fava reloads on the rename.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut tmp = path.as_os_str().to_os_string();
        tmp.push(".tmp");
        let tmp_path = PathBuf::from(tmp);
        std::fs::write(&tmp_path, self.render())?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }
}

/// Dedup key per the design: `txHash:logIndex:hash:direction`, and
/// `txHash:logIndex:unshield` for unshields.
#[allow(dead_code)]
fn dedup_key(event: &Event) -> String {
    if event.kind == "unshield" {
        format!("{}:{}:unshield", event.tx_hash, event.log_index)
    } else {
        format!(
            "{}:{}:{}:{}",
            event.tx_hash,
            event.log_index,
            event.hash.as_deref().unwrap_or(""),
            event.direction
        )
    }
}

/// Map a scanner event to a Beancount entry, or `None` if it is excluded
/// (change notes, sent notes, shields).
#[allow(dead_code)]
fn entry_from_event(event: &Event, date: &str) -> Option<Entry> {
    match event.kind.as_str() {
        "transact" => {
            // Change notes (outputType == 2) and sent notes are excluded.
            if event.output_type == Some(2) || event.direction != "in" {
                return None;
            }
            // Sales carry a product memo; the change note's received twin has
            // none, so the memo also separates it from the sales.
            let memo = event.memo.as_deref()?;
            let value = to_tst(&event.value);
            Some(Entry {
                date: date.to_string(),
                narration: memo.to_string(),
                metadata: metadata(event),
                postings: vec![
                    (ACCOUNT_RAILGUN.to_string(), format!("{value} {COMMODITY}")),
                    (ACCOUNT_SALES.to_string(), format!("-{value} {COMMODITY}")),
                ],
            })
        }
        "unshield" => {
            let value = to_tst(&event.value);
            let fee = event
                .fee
                .as_deref()
                .map(to_tst)
                .unwrap_or_else(zero_tst);
            let net = to_tst(&net_value(event));
            Some(Entry {
                date: date.to_string(),
                narration: "Withdrawal".to_string(),
                metadata: metadata(event),
                postings: vec![
                    (ACCOUNT_RAILGUN.to_string(), format!("-{value} {COMMODITY}")),
                    (ACCOUNT_BANK.to_string(), format!("{net} {COMMODITY}")),
                    (ACCOUNT_FEES.to_string(), format!("{fee} {COMMODITY}")),
                ],
            })
        }
        _ => None,
    }
}

/// Metadata per entry: `txhash`, `block`, `logindex`, `wallet`, `token`.
#[allow(dead_code)]
fn metadata(event: &Event) -> Vec<(String, String)> {
    vec![
        ("txhash".to_string(), format!("\"{}\"", event.tx_hash)),
        ("block".to_string(), event.block_number.to_string()),
        ("logindex".to_string(), event.log_index.to_string()),
        ("wallet".to_string(), format!("\"{}\"", event.wallet_address)),
        ("token".to_string(), format!("\"{}\"", event.token)),
    ]
}

/// Gross value minus fee, in base units (the net the EOA receives).
#[allow(dead_code)]
fn net_value(event: &Event) -> String {
    let value = BigDecimal::from_str(&event.value).unwrap_or_default();
    let fee = event
        .fee
        .as_deref()
        .and_then(|f| BigDecimal::from_str(f).ok())
        .unwrap_or_default();
    (value - fee).to_string()
}

/// Base units ÷ 10¹⁸, formatted to 18 decimal places.
#[allow(dead_code)]
fn to_tst(base_units: &str) -> String {
    let base = BigDecimal::from_str(base_units).unwrap_or_default();
    let tst = base / BigDecimal::from(10u64.pow(DECIMALS));
    if tst.is_zero() {
        return zero_tst();
    }
    tst.with_scale(DECIMALS as i64).to_string()
}

#[allow(dead_code)]
fn zero_tst() -> String {
    format!("0.{}", "0".repeat(DECIMALS as usize))
}

/// Escape a narration for a Beancount string literal.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/events-b.json"
    );

    fn fixture_events() -> Vec<Event> {
        let text = fs::read_to_string(FIXTURE).expect("fixture events file");
        serde_json::from_str(&text).expect("fixture events parse")
    }

    /// Beancount date for a block: 2026-11-01 + (block - 215) days.
    fn date_for_block(block: u64) -> String {
        let day = 1 + (block as i64 - 215);
        format!("2026-11-{day:02}")
    }

    fn feed(ledger: &mut Ledger, events: &[Event]) {
        for event in events {
            ledger.add(event, &date_for_block(event.block_number));
        }
    }

    /// Parse the rendered Beancount into (narration, postings) per entry.
    fn parse_entries(text: &str) -> Vec<(String, Vec<(String, String)>)> {
        let mut entries = Vec::new();
        let mut current: Option<(String, Vec<(String, String)>)> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("20") {
                if let Some(quote) = rest.find('"') {
                    if let Some(entry) = current.take() {
                        entries.push(entry);
                    }
                    let narration = rest[quote + 1..]
                        .split('"')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    current = Some((narration, Vec::new()));
                    continue;
                }
            }
            if let Some((_, postings)) = &mut current {
                if let Some((account, amount)) = parse_posting(line) {
                    postings.push((account, amount));
                }
            }
        }
        if let Some(entry) = current {
            entries.push(entry);
        }
        entries
    }

    fn parse_posting(line: &str) -> Option<(String, String)> {
        let line = line.trim_start();
        if !line.starts_with("Assets:") && !line.starts_with("Income:") && !line.starts_with("Expenses:") {
            return None;
        }
        let mut parts = line.split_whitespace();
        let account = parts.next()?.to_string();
        let amount = parts.next()?.to_string();
        Some((account, amount))
    }

    fn tst(s: &str) -> f64 {
        s.parse().unwrap()
    }

    #[test]
    fn fixture_produces_balanced_ledger() {
        let events = fixture_events();
        let mut ledger = Ledger::new();
        feed(&mut ledger, &events);
        let text = ledger.render();

        let entries = parse_entries(&text);
        assert_eq!(entries.len(), 6, "5 sales + 1 withdrawal");

        let mut balances: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for (_, postings) in &entries {
            let mut sum = 0.0;
            for (account, amount) in postings {
                let value = tst(amount);
                sum += value;
                *balances.entry(account.clone()).or_insert(0.0) += value;
            }
            assert!(
                (sum - 0.0).abs() < 1e-9,
                "entry not balanced: {postings:?}"
            );
        }

        assert!((balances["Assets:Railgun:B"] - 185.0).abs() < 1e-9);
        assert!((balances["Income:Sales"] - -235.0).abs() < 1e-9);
        assert!((balances["Assets:Bank:EOA2"] - 49.875).abs() < 1e-9);
        assert!((balances["Expenses:Fees"] - 0.125).abs() < 1e-9);
    }

    #[test]
    fn sales_have_product_memo_narrations() {
        let events = fixture_events();
        let mut ledger = Ledger::new();
        feed(&mut ledger, &events);
        let entries = parse_entries(&ledger.render());

        let narrations: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            narrations,
            [
                "transfer #0",
                "transfer #1",
                "transfer #2",
                "transfer #3",
                "transfer #4",
                "Withdrawal",
            ]
        );
    }

    #[test]
    fn withdrawal_splits_fee() {
        let events = fixture_events();
        let unshield = events
            .iter()
            .find(|e| e.kind == "unshield")
            .expect("unshield event");
        let mut ledger = Ledger::new();
        ledger.add(unshield, "2026-11-06");
        let entries = parse_entries(&ledger.render());
        assert_eq!(entries.len(), 1);
        let (narration, postings) = &entries[0];
        assert_eq!(narration, "Withdrawal");
        assert_eq!(postings.len(), 3);
        assert_eq!(postings[0], ("Assets:Railgun:B".to_string(), "-50.000000000000000000".to_string()));
        assert_eq!(postings[1], ("Assets:Bank:EOA2".to_string(), "49.875000000000000000".to_string()));
        assert_eq!(postings[2], ("Expenses:Fees".to_string(), "0.125000000000000000".to_string()));
    }

    #[test]
    fn change_notes_are_absent() {
        let events = fixture_events();
        let mut ledger = Ledger::new();
        feed(&mut ledger, &events);
        let text = ledger.render();

        // The change note (block 220, logIndex 4) must not appear.
        assert!(!text.contains("202454251d6d1f2d5b46ab950cc69604bb33c457e6816475f3233d4d8aac5712"));
        // No entry carries outputType 2.
        assert!(!text.contains("outputType"));
    }

    #[test]
    fn rerun_produces_no_duplicates() {
        let events = fixture_events();
        let mut ledger = Ledger::new();
        feed(&mut ledger, &events);
        feed(&mut ledger, &events);
        let entries = parse_entries(&ledger.render());
        assert_eq!(entries.len(), 6);
    }

    #[test]
    fn write_is_atomic_and_idempotent() {
        let events = fixture_events();
        let mut ledger = Ledger::new();
        feed(&mut ledger, &events);

        let dir = std::env::temp_dir().join(format!("ledger-test-{}", std::process::id()));
        let path = dir.join("ledger.beancount");
        ledger.write(&path).expect("write ledger");
        assert!(!path.with_extension("beancount.tmp").exists(), "tmp file cleaned up");

        let first = fs::read_to_string(&path).expect("read ledger");
        ledger.write(&path).expect("rewrite ledger");
        let second = fs::read_to_string(&path).expect("read ledger again");
        assert_eq!(first, second, "rewrite is idempotent");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn amount_formatting() {
        assert_eq!(to_tst("10000000000000000000"), "10.000000000000000000");
        assert_eq!(to_tst("125000000000000000"), "0.125000000000000000");
        assert_eq!(to_tst("49875000000000000000"), "49.875000000000000000");
        assert_eq!(to_tst("0"), "0.000000000000000000");
    }
}

---
id: pw-4hz9
status: open
deps: []
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 1
parent: pw-mn26
tags: [scanner]
---
# scanner: add fee to unshield events

## Description

The scanner's unshield event currently exposes only `value` (= `base + fee`).
The merchant ledger needs the split so it can post the net to
`Assets:Bank:EOA2` and the fee to `Expenses:Fees`. The contract emits
`Unshield(to, token, base, fee)` where `base` is already net of `fee`; the
scanner reports `value = base + fee` (the gross leaving the private balance).
Add a `fee` field to the event so consumers can compute `net = value - fee`.

## Design

The Rust handle is the source of truth for the wire contract; the Node scanner
mirrors it (contract-first).

1. `scanner/crates/handle/src/lib.rs` — add `pub fee: Option<String>` to the
   `Event` struct (serde field `fee`).
2. `scanner/src/wire.ts` — add `fee: z.string().nullable()` to `EventSchema`.
3. `scanner/src/decrypt.ts`:
   - `shieldEvents` / `tryDecryptTransact`: `fee: null`.
   - `unshieldEvents`: `fee: log.args.fee.toString()` (keep
     `value = log.args.amount + log.args.fee`).
4. Update tests that assert the event shape:
   - `scanner/src/decrypt.test.ts` (assert `fee` on the unshield event).
   - `scanner/tests/tests/protocol.rs` and `scanner/tests/tests/acceptance.rs`
     (the Rust `Event` struct now has `fee`).
5. Update `docs/scanner/spec.md` event shape (already drafted).

## Acceptance Criteria

- `Event` in the Rust handle and `EventSchema` in `wire.ts` both carry `fee`.
- Unshield events have `fee` = the contract's fee; shield/transact events have
  `fee: null`.
- `value` is unchanged (`base + fee`), so `net = value - fee` reconstructs the
  contract's `base`.
- `npm test` (scanner) and `cargo test` (scanner workspace) pass.

## References

- `docs/scanner/spec.md` — event shape, `fee` semantics
- `scanner/src/decrypt.ts:213` — `unshieldEvents` (current `value` computation)
- `scanner/src/abi.ts:59` — `UnshieldArgsSchema` (`amount`, `fee`)


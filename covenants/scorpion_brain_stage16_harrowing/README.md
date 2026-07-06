# Stage 16 — "On The Third Day" (the tomb's minimum)

> "Destroy this temple, and in three days I will raise it up." — John 2:19

Not a new chromosome, not a new byte of DNA — a new **global invariant** on the
existing IGNITE transition (14, Stage 14/FLAME-PHOENIX). As originally built,
IGNITE let the owner rise from DUST the instant after crumbling — a status
flip with extra steps, not a resurrection. Stage 16 closes that gap: the tomb
must hold for a real minimum duration before the flame is legal, keyed off
`dusted_at_daa` (already present since Stage 13, no schema change needed).

```rust
const THREE_DAYS_DAA: u64 = 259_200; // this project's established 86_400-DAA/day convention

assert!(current_daa >= old_dna.dusted_at_daa + THREE_DAYS_DAA,
    "the tomb has not yet held three days -- no resurrection before its time");
```

This is a **minimum, not a deadline** — the mirror image of Chromosome 11's
`redemption_deadline` (which *expires* if you wait too long). The tomb never
expires; you simply cannot rise early. No amount of owner_secret possession
lets you cheat the calendar — the calendar itself is now part of the covenant.

## Proven this session (2026-07-06), real local RISC0/STARK proofs

1. **NEGATIVE:** ignite attempted exactly **one DAA short** of three days
   (`current_daa = dusted_at_daa + 259_199`) → guest panics with
   `"the tomb has not yet held three days -- no resurrection before its time"`,
   proving correctly REJECTED.
2. **POSITIVE:** ignite attempted at **exactly** three days
   (`current_daa = dusted_at_daa + 259_200`, the `>=` boundary itself) →
   `receipt.verify()` PASSED. gen20 DUST → gen21 STEM, fee_reserve 0 → 60,000,000.

Both re-checked with the independent from-scratch verifier
(`risc0gate/verifier`, no risc0-zkvm crate — hand-rolled STARK/Merkle
verification matching Kaspa's `OpZkPrecompile` tag 0x21):
```
Step 1: verify_integrity() -> PASSED
Step 2: compute_assert_claim() -> PASSED
```
image_id in the receipt (`28a87a09...`) matches a from-source rebuild of the
current guest byte-for-byte — not a stale cached ELF.

## Real bug hit and fixed mid-build (build-cache staleness, not guest logic)

First run of the negative test *unexpectedly succeeded* — looked like a
security bug in the new assert. Root cause: risc0's guest build
(`methods/build.rs` → `risc0_build::embed_methods()`) didn't re-trigger after
editing `methods/guest/src/main.rs` — `cargo build` only recompiled the
`host` crate, reusing an ELF binary that was hours stale (confirmed via file
mtimes: guest ELF at 03:29, source edit at 15:08). Fix: `rm -rf
target/riscv-guest` + clear the `methods` fingerprint dir to force a true
guest rebuild; confirmed via `print_image_id` that the image_id actually
changed before re-running proofs. **Lesson: after any guest `main.rs` edit,
verify the image_id changed (or the ELF mtime is newer than the source edit)
before trusting a "PASSED"/"REJECTED" result — a silently stale build can
make a real logic change look verified when it never ran.**

## Regression-checked against existing proven suite

Stage 14's original test (`ignite_phoenix.rs`) and Stage 15's full 7-proof
lifecycle chain (`ignite_full_cycle.rs`) both hardcoded DAA gaps smaller than
three days for their IGNITE step (50,000 DAA) — both updated to
`dusted_at_daa + 259_200` and **re-run end-to-end successfully** under the
new rule, confirming Stage 16 doesn't break any previously-proven mechanism.

Code: `harrowing.rs` (dedicated boundary test). Local-proof-only, same status
as every stage since 11 — nothing beyond Stage 10's single mainnet hatch
(genesis + one heartbeat) is live on Kaspa mainnet yet.

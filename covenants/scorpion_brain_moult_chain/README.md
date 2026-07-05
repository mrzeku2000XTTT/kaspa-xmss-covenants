# Scorpion Brain — Moult Chain (the Spirit)

`hatch.rs` proves ONLY the literal one-time genesis transition (STEM gen0 -> VAULT gen1).
`moult.rs` generalizes this: given ANY prior receipt (which stores its own full
`new_dna_hex`), it parses the 721-byte DNA at exact byte offsets, advances
generation by 1, decrements `fee_reserve` by a heartbeat cost, correctly handles
the Chromosome 5 (metabolism) rate-window reset-or-advance rule
(`window_start_daa`/`window_spend_count`), and produces a fresh real RISC0/STARK
proof on top — locally verified every time.

This is what turns the organism from a single hatch event into something that
can keep living indefinitely: run `moult` again on its own output, forever
(as long as `fee_reserve` doesn't run out).

## Local proof chain (this session, 2026-07-05)

Ran 3 consecutive real STARK proofs, each one built directly on the previous
receipt's `new_dna_hex`, all `receipt.verify()` succeeded locally:

1. `hatch` — STEM gen0 -> VAULT gen1 (fee_reserve 60,000,000 -> 50,000,000)
2. `moult` — VAULT gen1 -> VAULT gen2 (fee_reserve 50,000,000 -> 45,000,000)
3. `moult` — VAULT gen2 -> VAULT gen3 (fee_reserve 45,000,000 -> 40,000,000)

Real bug caught and fixed mid-session: the first `moult` run panicked in-guest
with `expired rate window must reset to current_daa` — Chromosome 5's rate-limit
logic requires `window_start_daa` to reset to `current_daa` (and
`window_spend_count` to reset to 1) whenever `current_daa - window_start_daa >
rate_window`, otherwise persist `window_start_daa` and increment
`window_spend_count` by exactly 1. `moult.rs` now implements this correctly.

## Note on identity

This local prototype uses a FRESH owner_secret/covenant_id (regenerated after a
sandbox reset wiped the previous local key material) — it is NOT the same
identity as the real mainnet-hatched covenant (`299322b5...` / heartbeat
`6c0ad732...`, see entry 667 / PROVEN_MILESTONES.md). Nothing here touches
mainnet funds; this is purely local proof-chaining validation before wiring
`moult` into an actual on-chain hop-chain spend.

## Next step

Wrap each `moult` call in an actual mainnet spend (old covenant UTXO -> newly
built covenant address for the new DNA state), the same hop-chain pattern
already proven for Sentinel/Based App, so the chain becomes real on-chain
generations, not just local proofs.

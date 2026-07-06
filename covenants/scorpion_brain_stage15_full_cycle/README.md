# Stage 15 — IGNITING ("the rising of ashes")

Not a new chromosome. This is the proof that the whole 14-chromosome lifecycle
is a genuine, repeatable CYCLE — not 14 isolated tricks that happen to compile.
One unbroken chain of 7 real RISC0/STARK proofs, each one's receipt feeding
directly into the next proof's `old_dna`, one covenant_id and one owner_secret
the entire way through:

| # | Transition | From -> To | fee_reserve |
|---|-----------|-----------|-------------|
| 1 | HATCH | STEM gen0 -> VAULT gen1 | 60M -> 50M |
| 2 | MOULT | VAULT gen1 -> VAULT gen2 | 50M -> 45M |
| 3 | FALL | VAULT gen2 (starved) -> FALLEN gen3 | 15M -> 10M |
| 4 | FOSSILIZE | FALLEN gen3 (redemption missed) -> FOSSIL gen4 | 10M -> 5M |
| 5 | CRUMBLE | FOSSIL gen4 -> DUST gen5 | 5M -> 0 |
| 6 | **IGNITE** | **DUST gen5 -> STEM gen6** | **0 -> 60M (resupplied)** |
| 7 | MOULT | STEM gen6 -> VAULT gen7 | 60M -> 50M |

Step 6 is the headline: the organism crossed all the way to zero (true, provable
dust — no value, no further transitions possible under normal rules) and then
rose again as the SAME organism. Same `covenant_id` (chitin) from generation 0
through generation 7. Same `owner_commitment`. Same `creation_daa` /
`creation_txid` (the birth certificate, immutable since Chromosome 7 was built
in Stage 7 — enforced automatically here with zero extra code). The only thing
that changed across the flame is everything Stage 14 says should reset: XMSS
state, game state, escrow state, hunt state, the entire fall/redemption/dust
record.

## All 7 proofs verified locally, real STARK, this session (2026-07-06)

```
[1/7] HATCH      gen0 STEM  -> gen1 VAULT   fee 60M -> 50M   VERIFIED
[2/7] MOULT      gen1 VAULT -> gen2 VAULT   fee 50M -> 45M   VERIFIED
[3/7] FALL       gen2 VAULT -> gen3 FALLEN  fee 15M -> 10M   VERIFIED
[4/7] FOSSILIZE  gen3 FALLEN-> gen4 FOSSIL  fee 10M -> 5M    VERIFIED
[5/7] CRUMBLE    gen4 FOSSIL-> gen5 DUST    fee 5M  -> 0     VERIFIED
[6/7] IGNITE     gen5 DUST  -> gen6 STEM    fee 0   -> 60M   VERIFIED  <- PHOENIX
[7/7] MOULT      gen6 STEM  -> gen7 VAULT   fee 60M -> 50M   VERIFIED
```

Two real bugs caught and fixed while building this chain (both in the test
harness's hand-constructed DNA byte patches, not in guest logic — same lesson
as entry 680/691's fall/fossilize tests):

1. **Metabolism rate-window reset bug:** whenever `current_daa - old.window_start_daa
   > rate_window`, the guest requires the new state's `window_spend_count` to reset
   to exactly `1`, not `old.window_spend_count + 1`. Every DAA jump in this chain is
   far larger than `rate_window` (1000), so *every single transition* resets the
   window — got this wrong once (used `+1` instead of a hard reset to `1`) on the
   very first MOULT.
2. **Missing FALL-reason witness:** transition 7 (FALL) reads one extra witness byte
   (`reason: u8`) that none of the other transitions need — the test harness's
   generic `run_proof()` didn't support extra per-transition witness data, so added
   `run_proof_ex()` with an optional extra byte specifically for this case.

Code: `ignite_full_cycle.rs` (host binary, chains all 7 proofs in one process).
Final receipt (`final_ignite_receipt.json`) is the Stage 6 IGNITE proof from the
chain — the phoenix moment.

**Still local-proof-only.** Stage 10's single mainnet hatch (genesis + one
heartbeat) remains the only piece of this organism actually living on Kaspa
mainnet. This stage proves the mechanism end-to-end so that a future full
mainnet hop-chain deployment (each transition = a real P2SH successor spend)
has a verified reference implementation to follow.

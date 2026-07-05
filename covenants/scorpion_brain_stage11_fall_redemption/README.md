# Stage 11 — Fall & Redemption (the afterlife)

Chromosome 11, added on top of the complete 10-chromosome scorpion_brain
(identity, ownership, XMSS, privacy, game, routing, reproduction, memory,
escrow, metabolism — entries 624-655). DNA grows from 721 -> 741 bytes.

## Concept

The organism is not just alive/dead. It has a third state: **fallen**. Any
active organism (STEM, VAULT, SENTINEL) can be pushed into `MODE_FALLEN`
permissionlessly, by anyone, if they can prove one of three true failures:

1. **Starvation** -- `fee_reserve < min_output` (it let its own reserve run dry)
2. **Cancer** -- `window_spend_count >= rate_max_spend` (spent faster than its own policy allows)
3. **Abandonment** -- a Sentinel-armed organism's check-in deadline + grace period has lapsed

No secret is required to trigger a fall -- the evidence is public and checkable
on-chain, so nobody should need the owner's permission to record that the
organism has genuinely failed.

Once fallen, a fixed, protocol-level (not owner-tunable) redemption window
opens: `redemption_deadline = fallen_at_daa + 100_000` DAA, and the owner
needs `redemption_threshold = 5` honest heartbeats (each proving control of
`owner_secret`) within that window to earn **ascension** back to `MODE_VAULT`.
Ascension is the one deliberate exception to "fee_reserve must strictly
decrease" in the entire organism -- the owner must actually resupply the
reserve, a real sacrifice, not just a status flip.

If the deadline lapses without enough redemption heartbeats, anyone may
permissionlessly **fossilize** the organism into `MODE_FOSSIL` -- the true
terminal state. No further transitions exist out of `MODE_FOSSIL`. The
chitin (covenant_id) persists, immutable, forever, but the organism is gone.

## Transitions added

| # | Name | From -> To | Authorization |
|---|------|-----------|----------------|
| 7 | FALL | STEM/VAULT/SENTINEL -> FALLEN | permissionless, proof of genuine failure |
| 8 | REDEEM | FALLEN -> FALLEN | owner_secret, redemption_progress += 1 |
| 9 | ASCEND | FALLEN -> VAULT | owner_secret + redemption_progress >= threshold + fee_reserve resupply |
| 10 | FOSSILIZE | FALLEN -> FOSSIL | permissionless, deadline lapsed + threshold not met |

## Proof status (this session, 2026-07-05)

All three local RISC0/STARK proofs generated and verified (`receipt.verify()` passed):

- **FALL**: genuine starvation (`fee_reserve=15M < min_output=20M`) -> `MODE_FALLEN`. Negative test (false starvation claim on a healthy organism) correctly rejected.
- **REDEEM**: one honest heartbeat, `redemption_progress` 0->1, proof of `owner_secret`. Negative test (wrong owner_secret) correctly rejected.
- **FOSSILIZE**: redemption window lapsed with only 2/5 progress -> `MODE_FOSSIL`. Two negative tests: fossilizing before the deadline lapses (rejected), and fossilizing an organism that already earned enough redemption (rejected -- should ascend, not die).

**Not yet proven locally:** ASCEND (requires 5 chained real redemption heartbeats to reach threshold -- long-running, queued as next step). Nothing in this stage has been deployed to mainnet yet -- local proof-of-mechanism only, same status as Stages 1-9 before their eventual P2SH wrap (Stage 10 precedent: `covenants/scorpion_brain_stage10/`).

Files: `fall.rs`, `redeem.rs`, `fossilize.rs` (host binaries), `*_receipt.json` (proof artifacts -- public data only, no secrets). Guest logic lives in `god_particle/scorpion_brain/methods/guest/src/main.rs` (synced, now 741-byte DNA).

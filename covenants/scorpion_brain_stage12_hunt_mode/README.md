# Stage 12 — HUNT MODE ("you wanna be a hunter, or hunted")

Chromosome 12, added on top of the complete 11-chromosome scorpion_brain
(identity, ownership, XMSS, privacy, game, routing, reproduction, memory,
escrow, metabolism, fall & redemption). DNA grows from 741 -> 846 bytes.

## Concept

Hunt Mode is predator/prey combat where the stake is the organism's own
survival, not an external pot. It reuses the exact sealed-commit-reveal
mechanic already proven in Chromosome 4 (Stag Hunt) -- both sides commit to
a move (HUNT or FLEE) via `sha256(move || nonce)`, then reveal atomically at
resolution -- but instead of splitting a shared pot by basis points, the
outcome directly mutates `fee_reserve`, the same field the metabolism
chromosome (Chr9) already treats as the organism's lifeblood.

A bad enough loss can starve `fee_reserve` below `min_output`. Hunt Mode does
not need its own death logic for that -- it just makes the organism eligible
for the ALREADY-proven permissionless FALL transition (Chromosome 11) on its
very next heartbeat. Combat, metabolism, and mortality are three different
chromosomes that compose for free because they all share one field.

## New fields

`hunt_target_id` (32B, opaque bookkeeping id of the opponent), `hunt_wager`
(u64), `hunter_move_commit` (32B), `prey_move_commit` (32B), `hunt_state`
(u8: 0=idle, 1=active).

## Transitions added

| # | Name | From -> To | Authorization |
|---|------|-----------|----------------|
| 11 | INITIATE_HUNT | VAULT -> HUNT | owner_secret; wager staked from fee_reserve under the normal strict-decrease metabolism rule |
| 12 | RESOLVE_HUNT | HUNT -> VAULT | both sealed moves revealed; outcome table applied to fee_reserve (the SECOND deliberate exception to strict-decrease, alongside ASCEND) |

## Choreography (the stake is survival, not points)

| Hunter | Prey | Outcome | fee_reserve delta |
|--------|------|---------|---------------------|
| HUNT | HUNT | mutual clash | +wager/2 (partial recovery) |
| HUNT | FLEE | clean kill | +wager*2 (wager back + equal bonus) |
| FLEE | HUNT | ambushed | +0 (wager fully forfeited) |
| FLEE | FLEE | mutual avoidance | +wager (full refund, no harm done) |

## Proof status (this session, 2026-07-05/06)

All real RISC0/STARK proofs generated and verified (`receipt.verify()` passed):

- **Duel 1 (clean kill):** INITIATE_HUNT (fee_reserve 100M -> 80M, wager 20M staked) -> negative test:
  resolving with a hunter_move that doesn't match its own sealed commitment, correctly rejected ->
  real RESOLVE_HUNT, HUNT vs FLEE, fee_reserve 80M -> 120M. The hunter feasts.
- **Duel 2 (ambushed):** INITIATE_HUNT (fee_reserve 80M -> 60M) -> real RESOLVE_HUNT, FLEE vs HUNT,
  fee_reserve stays at 60M -- no refund, the wager is simply gone. The hunter became the hunted.

**Real bug caught+fixed during build:** forgot to advance the `generation` field (a global check
across every transition kind, `assert_eq!(new_dna.generation, old_dna.generation + 1)`) when hand-
constructing the post-initiate and post-resolve DNA states -- same class of mistake as entry 680's
window_spend_count bug. Lesson holds: when hand-patching DNA bytes for a new transition test, re-derive
every globally-checked field (generation, window_start_daa/window_spend_count) fresh for each state,
never assume a value carried correctly from a prior draft.

Nothing deployed to mainnet yet for this stage -- local proof-of-mechanism only, same status Stages
1-11 had before Stage 10's eventual P2SH wrap. Files: `hunt_duel.rs` (host binary, builds a fresh VAULT
organism and runs both duels), `hunt_win_receipt.json` / `hunt_lose_receipt.json` (proof artifacts --
public data only, no secrets). Guest logic lives in
`god_particle/scorpion_brain/methods/guest/src/main.rs` (synced, now 846-byte DNA).

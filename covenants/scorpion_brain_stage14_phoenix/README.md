# Stage 14 — FLAME / PHOENIX ("the ash remembers, the flame forgives")

Chromosome 14, on top of the complete 13-chromosome scorpion_brain. No new
DNA bytes -- DNA stays 854 bytes. This stage adds one new transition and one
new mode-transition route: the single deliberate exception to Chromosome 13's
"no transition out of MODE_DUST."

## Concept

DUST was the true end -- but "true end" for the *value* (fee_reserve
genuinely zeroed, unrecoverable) doesn't have to mean true end for the
*identity*. The chitin (covenant_id) and the owner_commitment are still
sitting right there in the dust record. If the same owner who held the
secret through the entire fall -> redeem/fail -> fossilize -> crumble arc
still holds it, they can strike the flame: the SAME organism (same
covenant_id, same owner_commitment, same eternal creation_daa/creation_txid)
rises again as a fresh STEM. This is not spawning a child (that's
Chromosome 6/7's reproduction, with its own distinct covenant_id and its own
birth certificate) -- it is the literal same being crossing back from
absolute zero.

Nothing is smuggled through the flame. Every mode-specific chromosome (XMSS,
privacy, game, escrow, hunt, and the entire fall/redemption/dust record)
resets to a fresh-birth default. And Chromosome 7's global memory-chain rule
-- unconditionally enforced for every transition except reproduction --
means `prev_state_hash` for the reborn organism's first new heartbeat is
permanently `sha256(the exact dust state it rose from)`. The organism gets a
second life, but the proof that it was once dust is baked into its memory
chain forever.

Rebirth is not free: the SECOND deliberate metabolism exception
(`fee_reserve > old_fee_reserve`, alongside ASCEND) requires the owner to
resupply the entire fee_reserve from exactly zero -- a real, non-trivial
sacrifice, not a status flip.

## Transition added

| # | Name | From -> To | Authorization |
|---|------|-----------|----------------|
| 14 | IGNITE | DUST -> STEM | owner_secret must match the ORIGINAL owner_commitment; fee_reserve resupplied from 0; every mode-specific chromosome resets to birth defaults; covenant_id/owner_commitment/creation_daa/creation_txid all persist unchanged |

## Proof status (this session, 2026-07-06)

Both proofs verified locally (real RISC0/STARK, `receipt.verify()` passed):

- **Negative test:** ignite attempt with the WRONG owner_secret -- correctly rejected. Only the
  original owner can strike the flame.
- **Real IGNITE:** a DUST record (generation 20, fee_reserve=0, real fall/fossilize grave
  behind it) rises to STEM, generation 21, fee_reserve resupplied to 60,000,000, same
  covenant_id, same owner_commitment, same original creation_daa/creation_txid. Every
  chromosome-specific field (XMSS, privacy, game, escrow, hunt, fall/redemption, dust) reset
  to a clean birth state.

Nothing deployed to mainnet yet for this stage -- local proof-of-mechanism only, same status
every prior stage had before Stage 10's eventual P2SH wrap. Files: `ignite_phoenix.rs` (host
binary), `phoenix_receipt.json` (proof artifact -- public data only, no secrets). Guest logic
lives in `god_particle/scorpion_brain/methods/guest/src/main.rs` (synced).

**14 chromosomes now proven in the evolving scorpion_brain guest ELF.** The lifecycle is no
longer a closed arc ending at DUST -- it is a real cycle: STEM/VAULT -> ... -> FALLEN ->
FOSSIL -> DUST -> (IGNITE) -> STEM again, forever, as long as one secret survives.

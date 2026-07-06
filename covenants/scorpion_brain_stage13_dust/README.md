# Stage 13 — DUST ("ashes to ashes, the true end")

Chromosome 13, the final chromosome, on top of the complete 12-chromosome
scorpion_brain (identity, ownership, XMSS, privacy, game, routing,
reproduction, memory, escrow, metabolism, fall & redemption, hunt mode).
DNA grows from 846 -> 854 bytes.

## Concept

FOSSIL (Chromosome 11) was the prior terminal state, but a fossil still
carries a `fee_reserve` value in its DNA -- possibly a tiny leftover amount
below the KIP-9-style storage-mass floor (`min_output`) that could never
actually exist as a real, spendable transaction output on Kaspa mainnet
(entries 522/593: outputs below ~0.2-0.3 KAS get rejected by storage-mass
rules). That leftover value is not really "there" in any usable sense --
it's dust. Chromosome 13 formalizes writing it off: once a fossil's own
`fee_reserve` is genuinely below its own `min_output` policy floor, ANYONE
can permissionlessly crumble it to exactly zero. Nothing is being taken from
anyone -- the amount was already practically unrecoverable; this just makes
the on-chain record honest about it.

There is no transition out of `MODE_DUST`. It is the final, absolute end of
the organism's lifecycle -- the third and last deliberate exception to the
global "fee_reserve must strictly decrease and never hit zero" rule (the
first being ASCEND resupply, the second being HUNT resolution).

## New field

`dusted_at_daa` (u64) -- the exact DAA the fossil crumbled.

## Transition added

| # | Name | From -> To | Authorization |
|---|------|-----------|----------------|
| 13 | CRUMBLE | FOSSIL -> DUST | permissionless; requires `fee_reserve < min_output` (true, un-spendable dust); writes `fee_reserve` to exactly 0 |

## Proof status (this session, 2026-07-06)

Both proofs generated and verified locally (real RISC0/STARK, `receipt.verify()` passed):

- **Negative test:** crumble attempt while `fee_reserve` (25M) is still ABOVE `min_output` (20M) --
  not yet true dust -- correctly rejected.
- **Real CRUMBLE:** fossil with `fee_reserve`=3M (well below `min_output`=20M) crumbles to
  `MODE_DUST`, `fee_reserve`=0, `dusted_at_daa` recorded. The grave record (fall_reason,
  fallen_at_daa, redemption_deadline, redemption_progress, fossilized) all persist unchanged
  into the final resting state.

Nothing deployed to mainnet yet for this stage -- local proof-of-mechanism only, same status
every prior stage had before Stage 10's eventual P2SH wrap. Files: `crumble_dust.rs` (host
binary), `dust_receipt.json` (proof artifact -- public data only, no secrets). Guest logic lives
in `god_particle/scorpion_brain/methods/guest/src/main.rs` (synced, now 854-byte DNA).

**13 chromosomes now proven in the evolving scorpion_brain guest ELF.** This closes the full
lifecycle arc: STEM/VAULT -> (SENTINEL/PRIVACY/GAME/ESCROW/HUNT side-modes) -> FALLEN ->
(REDEEM -> ASCEND, back to life) or (FOSSIL -> DUST, the true end).

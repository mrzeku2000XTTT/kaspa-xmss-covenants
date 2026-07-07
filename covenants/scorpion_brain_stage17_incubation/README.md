# Stage 17 — "111" (The Newborn Incubation Gate)

The angel number: new beginnings, alignment, made literal as an on-chain invariant.

## Concept

Stage 16 ("On The Third Day") proved the tomb must hold a minimum of 259,200 DAA
(three days) before IGNITE can raise a DUST organism back to STEM. But once the
flame rises, the organism as originally built could take its first step *out* of
STEM — a MOULT into VAULT, arming Sentinel, anything — the very next instant.
That's not a newborn. That's an adult wearing a phoenix costume.

Stage 17 closes that gap with one new DNA field (`reborn_at_daa: u64`, DNA grows
854 → 862 bytes) and one new global invariant:

- **IGNITE (transition 14)** now must set `new_dna.reborn_at_daa = current_daa` —
  the exact moment of rebirth, permanently recorded.
- **Every other transition** must leave `reborn_at_daa` untouched (historical
  record, not a recurring deadline).
- If `old_dna.mode == STEM && old_dna.reborn_at_daa != 0 && new_dna.mode != STEM`
  (i.e. this is the newborn's first real departure from STEM), the transition
  additionally requires `current_daa >= old_dna.reborn_at_daa + 111`.

111 is a **minimum**, exactly like Stage 16's three-day tomb — it never expires,
and it is enforced no matter who holds `owner_secret`. Once earned (in the sense
that 111 DAA have genuinely passed), it's a one-time gate: further transitions
inside VAULT (or wherever the organism goes) are unaffected.

## Proofs (all real RISC0/STARK, `receipt.verify()` passed)

Chained together in one process (`host/src/bin/incubation.rs`):

1. **IGNITE**: DUST (gen20) → STEM (gen21) at exactly the Stage 16 three-day
   boundary, fee_reserve resupplied 0 → 60,000,000, `reborn_at_daa` recorded.
2. **NEGATIVE**: MOULT (STEM → VAULT) attempted exactly 1 DAA short of the
   111-DAA incubation window → correctly **REJECTED** ("the newborn must rest
   111 DAA in the cradle before its first true step out of stem").
3. **POSITIVE**: the identical MOULT attempted at exactly 111 DAA after rebirth
   (the `>=` boundary) → **VERIFIED**, gen21 STEM → gen22 VAULT.

Local-proof-only, same status as Stages 11–16. Stage 10's mainnet hatch
(genesis + one heartbeat) remains the only piece of the scorpion_brain live
on Kaspa mainnet.

## DNA layout change

`DNA_LEN` 854 → 862 bytes. New field `reborn_at_daa: u64` appended at the very
end (offset 854..862), immediately after Stage 13's `dusted_at_daa`.

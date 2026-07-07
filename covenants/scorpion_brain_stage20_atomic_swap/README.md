# Stage 20 — "Crosschain Atomic Transfer" (Chromosome 20, transition 17)

A real HTLC (hash + time locked contract), built directly into the scorpion_brain
organism's own state machine. This is what lets one scorpion covenant atomically swap
value with anything else — another chain, another protocol, or (proven here) **another
independent scorpion instance**.

## DNA growth

DNA blob: 914 → 955 bytes (+41).

| offset | size | field |
|---|---|---|
| 914 | 32 | `swap_hash` — sha256(secret), committed at INIT |
| 946 | 8  | `swap_timeout_daa` — absolute DAA deadline for refund |
| 954 | 1  | `swap_active` — 0 = no pending swap, 1 = pending |

New guest image ID (confirmed changed from Stage 19's `f063b2a9...`):
`014004cf2621fc40ab2700597fe10a2004ce6a30128c23f01e2038b81f822d21`

## Mechanics — transition 17, action byte selects sub-behavior

- **action 0 — INIT**: commit to a fresh `swap_hash` + `swap_timeout_daa`. Requires no
  swap already pending. Timeout must be at least `MIN_SWAP_WINDOW_DAA` (100) in the
  future — no instant-deadline griefing.
- **action 1 — CLAIM**: reveal the preimage. Deliberately **not owner-gated** — anyone
  who knows the secret can unlock, because knowledge of the secret IS the
  authorization (it's the same secret the counterparty revealed on the other leg of the
  swap). The revealed secret is committed **in the clear, publicly, permanently** to the
  STARK receipt's journal (appended after the usual 66-byte `old_hash‖new_hash‖old_mode‖
  new_mode` header) — so any observer, on any chain, can read it straight off the
  accepted proof and complete their own leg with the exact same secret. That public
  reveal is the entire mechanism that makes this atomic instead of just a timelock.
- **action 2 — REFUND**: if the timeout has passed and nobody claimed, permissionless
  cancel — no signature required, mirrors Chromosome 3's sentinel dead-man's-switch.

All three require `mode == VAULT` (only a living, mature scorpion can swap) and don't
change the scorpion's own mode. Outside transition 17, all three swap fields are frozen
(Phase 1h global invariant) — no unrelated heartbeat can tamper with a pending swap.

## Proofs — Part 1: single-scorpion mechanics (5 proofs, all real local STARKs)

0. HATCH: STEM(gen0) → VAULT(gen1).
1. SWAP_INIT: commits a hash + 200-DAA-future timeout.
2. SWAP_CLAIM: reveals the correct secret — unlocks, and the secret is confirmed
   present, in the clear, in the receipt's public journal.
3. A second SWAP_INIT + SWAP_REFUND: nobody claims, timeout passes, permissionless
   refund succeeds with `swap_active` cleared and the secret never revealed.
4. NEGATIVE: CLAIM attempted with the **wrong** secret — correctly rejected
   (`sha256(secret) != swap_hash`).

## Proofs — Part 2: the actual point — scorpion ↔ scorpion atomic swap (6 proofs)

Two **independent** scorpion covenant instances (different `covenant_id`, different
owner) swap with each other:

1. Hatch Scorpion A (Alice) and Scorpion B (Bob) — two separate organisms.
2. Bob picks a secret, gives Alice only `sha256(secret)`.
3. **Alice INITs first** on her own covenant, locking for Bob, with the **longer**
   timeout (she's taking on first-mover risk).
4. **Bob mirrors second** on his own covenant, locking for Alice, with a **shorter**
   timeout — standard atomic-swap safety margin: if anything goes wrong, Bob's window
   closes first, so Alice is never left exposed longer than Bob.
5. **Bob CLAIMs Alice's lock** (on Scorpion A) — this makes the secret publicly
   readable in Scorpion A's accepted STARK journal.
6. **Alice extracts the secret straight off that public journal** (no side channel to
   Bob needed) and **CLAIMs Bob's lock** (on Scorpion B) with it.

Result: both scorpions end with `swap_active = 0`. Neither party could take the other's
locked value without also revealing the exact key that lets the other claim theirs.
That's atomicity — enforced by STARK proofs and a shared hash, not by trust.

## Status

Local-proof only, same as every stage since 11 — nothing new spent/deployed on mainnet.
Chromosome count now 18: identity, ownership, XMSS, privacy, game theory, routing,
reproduction, memory, escrow, metabolism, fall/redemption, hunt mode, dust, flame/phoenix,
newborn incubation, world genesis, ARMA, **crosschain atomic transfer (new)**.

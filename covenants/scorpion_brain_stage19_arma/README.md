# Stage 19: "ARMA" (Chromosome 19 — assembly/consent separation)

Named for the real academic BFT design (Arma: Byzantine Fault Tolerant Consensus with
Horizontal Scalability) that splits a pipeline into many parallel ASSEMBLERS — who batch
and chain data, scaling with raw bandwidth/CPU — and one thin CONSENTER, who only
orders/finalizes batch commitments, never touching individual items one at a time.

## What it does

An off-chain assembler pre-computes an entire internal chain of ordinary VAULT→VAULT
heartbeats (each hop obeying the exact same rules as a normal moult: generation+1,
transition_count+1, fee_reserve strictly decreasing, memory-linked via sha256). ONE
on-chain proof — the consenter — verifies the whole internal chain in a single shot
(transition 16) and finalizes only the LAST state. `generation` and `transition_count`
advance by `batch_size` instead of a fixed 1: the organism's own throughput scales
horizontally with how much its assembler prepared, while the on-chain footprint stays
exactly one transition, same as every other kind.

## DNA changes

Grew 902 → 914 bytes:
- `last_batch_size: u32` (902..906) — records the batch_size just consented to.
- `total_batched_transitions: u64` (906..914) — cumulative count across all ARMA batches ever consented to, append-only.

## New helper: `assert_hop_valid`

Verifies one internal hop (`prev` DNA → `curr` DNA): same covenant_id, same birth
certificate, generation/transition_count advance by exactly 1, memory chain linked via
sha256, fee_reserve strictly decreasing and never hitting zero mid-batch. Every OTHER
byte is frozen EXCEPT: `prev_mode` (legitimately flips STEM→VAULT on the very first
hop only), the metabolism rate-window bookkeeping, and Chromosome 19's own
`last_batch_size`/`total_batched_transitions` fields — all three of those are only
meaningful once, for the single on-chain footprint the WHOLE BATCH produces (checked
separately, unmodified, by the existing Phase 1b/1c logic on the outer old_dna/new_dna
pair — exactly as for any ordinary heartbeat).

## Proofs (3 real local RISC0/STARK proofs, all `receipt.verify()` passing)

1. **HATCH** (transition 0): STEM(gen0) → VAULT(gen1), ordinary birth.
2. **ARMA_CONSENT** (transition 16): assemble 5 ordinary heartbeats off-chain,
   consent to the WHOLE batch in ONE proof. VAULT(gen1) → VAULT(gen6) in a single
   transition. `transition_count` +5, `total_batched_transitions` 0→5,
   `last_batch_size` set to 5.
3. **NEGATIVE**: an assembled batch where one inner hop's `fee_reserve` does NOT
   strictly decrease relative to the true previous hop — correctly **REJECTED**:
   `"fee_reserve must strictly decrease per hop -- every assembled heartbeat still
   costs something"`.

Guest image ID (fresh, confirmed changed from Stage 18's
`148b87564d87afc500d5a906f93bb2e4c0af57168ee9fabed7c7b47105059029` per standing
rule): **`f063b2a93c7adb5fb0b37d89c52898027224ab1ee2204a5bca893ebabdaa5f50`**

## Status

Local-proof only, same as Stages 11-18 — nothing new spent/deployed on mainnet.

Chromosome count now 17: identity, ownership, XMSS, privacy, game theory, routing,
reproduction, memory, escrow, metabolism, fall/redemption, hunt mode, dust,
flame/phoenix, newborn incubation (17), world genesis (18), **ARMA / assembly-consent
separation (19, new)**.

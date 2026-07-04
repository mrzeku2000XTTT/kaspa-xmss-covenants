# Proven Milestones — Kaspa Toccata / KCC20 Covenant R&D

**This file is the single source of truth for what's already been proven on mainnet.**
Check this FIRST before spending real KAS re-proving something. If it's here, it's done —
build on top of it instead of repeating it. Update this file every time a new mainnet
proof lands, regardless of which conversation/session did the work.

Format: `TX <txid>` = mainnet transaction, confirmed `is_accepted:true` on api.kaspa.org unless noted.

---

## 1. XMSS Post-Quantum Signatures (replacing ECDSA)
- Reference impl cross-validated byte-for-byte vs official `xmss-reference` C code.
- 2-layer on-chain-linked hypertree scaled to **h=16/layer = 2^32 addressable signatures**,
  script size only grows O(height) not O(leaves) (~133KB regardless of tree depth).
  Deploy `2b5e0446...eaed4`, claim/spend `87dbf376...d05037`.
- Applications proven: 2-of-2 multisig, recurring payments, time-capsule (XMSS+CLTV),
  minimal "Based App" 3-hop workflow covenant, Stag Hunt game-theory covenant (branching
  payouts via `OpTxOutputAmount`/`OpTxOutputSpk`).
- Code: `covenants/hypertree_2layer/`, `covenants/multisig_2of2/`, `covenants/recurring_payments/`,
  `covenants/time_capsule/`, `covenants/based_app_workflow/`, `covenants/stag_hunt/`.

## 2. Sentinel — Quantum-Safe Dead-Man's-Switch (XMSS + resettable CLTV)
- IF-branch (check-in/renewal) and ELSE-branch (permissionless CLTV release) both proven
  end-to-end on mainnet with a fully generalized, arbitrary-customer-key pipeline (not
  hardcoded demo material) — `sentinel_keygen.py` runs 100% locally/air-gapped.
- Multi-hop chain proof: deploy `9fe4bbea...53a61` -> check-in `433b9d95...27c10` -> check-in
  `7b54945c...bd6d7`, both is_accepted:true.
- Repurposed as **sentinel-x402**: Kaspa-native x402 payment scheme — "check-in" = pay for
  next period, timeout = automatic permissionless refund. Deploy `305b3f52...a99e`, pay
  `7167ce82...e72a`, refund `737cd410...631cd0`.
- Wired into SuperZK app as `covenant_type="sentinel"`.
- Code: `covenants/sentinel/`, `x402-kaspa/sentinel-x402/`.

## 3. ZKGate — Groth16 (OpZkPrecompile tag 0x20)
- Cracked the exact compressed G1/G2 proof+VK byte encoding Kaspa's precompile expects
  (LE coords + sign-flag bit, no libsnark dependency needed).
- Fully generalized — works with ANY fresh secret, not one memorized hardcoded proof.
- Self-serve unlock proven: deploy -> reveal proof kit -> customer requests unlock -> agent
  executes ZK-proof-branch spend using ONLY public redeem_script_hex (zero private keys,
  zero custody risk) since `OpTxOutputSpk` enforces payout destination in-script.
- Deploy `ef1374fc...02dcd`, spend `43cbe373...41537` (fresh secret=999999, not the original
  hardcoded one). Self-serve unlock: deploy `67ba65f4...`, unlock `e8ca919a...5b27b42`.
- Code: `covenants/zkgate_groth16/` (circuit, verification key, generic deploy/unlock scripts).

## 4. ZKGate — RISC Zero / STARK (OpZkPrecompile tag 0x21)
- First-ever mainnet verification of Kaspa's RISC0/STARK precompile.
- **Earlier proof (community-visible, "Fable 5"):** deploy+spend confirmed 2026-07-03
  ~03:17 UTC, spend TX `a2fe7dcf...f11d0`, mass 479838. (Discovered after the fact —
  this file exists specifically so this kind of rediscovery doesn't cost real KAS again.)
- **Second independent proof (2026-07-04):** installed full RISC0 toolchain from scratch,
  generated a genuine succinct STARK receipt (poseidon2) in <2min on 1 CPU core, verified
  locally against a standalone Rust verifier crate, then deployed (`65344544...d0eb`) and
  spent (`0418ce59...ceed8`, mass 494838) via the OP_ZK/tag-0x21 branch. Both spends
  is_accepted:true.
- **Known gotcha:** compute mass scales ~linearly with `compute_budget` (~175 mass/unit),
  a SEPARATE constraint from the SU/fee formula (`allowed_SU = budget*10000+9999`). Tune
  compute_budget near the 500k mass ceiling, not just for SU sufficiency — budget=2700
  worked for a 223KB-script/25M-SU receipt.
- **Significance:** hash-only, no trusted setup, post-quantum — the one that actually beats
  current Zcash tech per `research/PRIVACY_ADVANCED_RESEARCH.md`.
- Code: `covenants/zkgate_risc0/` (guest/host RISC0 project + standalone verifier crate).

## 5. Privacy Roadmap (research, not yet built)
- Full phased plan for a Tornado/Zcash-style shielded pool adapted to Kaspa's per-UTXO
  P2SH state-covenant model, built on the now-proven RISC0/STARK precompile.
- See `research/PRIVACY_ROADMAP.md` and `research/PRIVACY_ADVANCED_RESEARCH.md`.
- **Next actual milestone to build:** Phase 1 shielded pool proof-of-mechanism (small tree,
  1 fixed denomination, manual nullifier tracking) — not yet started as of 2026-07-04.

## 6. x402-Kaspa
- One-shot `exact` scheme (signed-but-unbroadcast payment + facilitator verify/settle):
  TX `96ac9a37...5a550`.
- Recurring/subscription scheme: see Sentinel-x402 above.
- Code: `x402-kaspa/`.

---

### House rules for adding to this file
- Every new mainnet proof gets a one-paragraph entry here with real TX ids, BEFORE moving
  on to the next thing — not just in per-conversation memory.
- If you're about to spend real KAS proving something, search this file for the concept
  first (Ctrl-F the covenant type / precompile tag / opcode name).
- Keep it factual and current — update in place if a "proof" later turns out flawed
  (see e.g. Sentinel's leaf-index-0 debugging saga, kept in per-conversation memory not here
  since it's implementation-detail-level, not milestone-level).

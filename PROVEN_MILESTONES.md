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

## 5. Privacy Roadmap
- Full phased plan for a Tornado/Zcash-style shielded pool adapted to Kaspa's per-UTXO
  P2SH state-covenant model, built on the now-proven RISC0/STARK precompile.
- See `research/PRIVACY_ROADMAP.md` and `research/PRIVACY_ADVANCED_RESEARCH.md`.

## 6. Shielded Privacy Pool — FIRST mainnet-proven Groth16 withdrawal (2026-07-04)
- **What:** Tornado Cash/Zcash-style shielded pool on Kaspa. 4-leaf Poseidon Merkle tree;
  deposit = commitment in tree; withdrawal = Groth16 ZK proof of membership + revealed
  nullifier hash (prevents double-spend without revealing which deposit). Observer sees
  "someone proved membership and revealed nullifier X" — NOT which of the 4 deposits was
  spent. Withdrawal goes to a fresh, unlinkable address.
- **Circuit:** circom `withdraw.circom` — Merkle membership (4-leaf Poseidon tree) +
  nullifier hash. 2 public inputs: [root, nullifierHash]. Trusted setup via snarkjs
  (Groth16). Proof verified locally with snarkjs before on-chain attempt.
- **Key breakthrough — public input push order:** rusty-kaspa's OpZkPrecompile (0xa6, tag
  0x20) pops public inputs from the TOP of the stack (LIFO). So `inputs[0]` (root) must be
  pushed LAST (on top), `inputs[1]` (nullifier) pushed FIRST (deeper). Correct push order:
  `pd(NULLIFIER), pd(ROOT), pi(2), pd(PROOF), pd(VK), pd(0x20), OP_ZK`. Getting this wrong
  produces "Groth16 verification failed" — confirmed by reading the precompile source at
  `crypto/txscript/src/zk_precompiles/groth16/mod.rs`.
- **Mainnet TXs:**
  - Deploy (0.5 KAS locked): `da609f48fcdb22e50747ce120281959356cb454019810228914c184604c1e3a8`
  - **Withdrawal (ZK proof verified on-chain, 0.28 KAS to fresh address):** `c6d3606659485f16f5b5560cce172e587f24832b62e8ea00949466b2e41a7f9c` — `is_accepted: true`
- **Fee economics:** Groth16 verification with 2 public inputs needs ~20.1M sompi
  (compute mass 201,132) in required fees. Covenant must be funded with enough to cover
  fee + leave output above KIP-9 storage-mass floor (~0.2 KAS). 0.5 KAS deposit worked
  cleanly (0.28 KAS output after 0.22 KAS fee).
- **Known loss:** 0.35 KAS permanently stuck in an earlier wrong-pubkey + wrong-order
  deploy (TX `fa7d1032...`) — both branches unspendable. 0.2 KAS in another attempt (TX
  `9195b7a1...`) reclaimable via ELSE/owner-sig but pending sighash format debugging.
- Code: `covenants/privacy_pool/` (circuit, keys, deploy/spend scripts, encoding).
- **Next:** Phase 2 — on-chain nullifier-registry covenant (currently manual tracking),
  deeper tree (O(height) scaling confirmed from XMSS work), dynamic denominations.

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

## 7. God Particle — Scorpion Brain (RISC0/STARK unified covenant organism)
- **Thesis:** move ALL covenant logic (identity, ownership, XMSS immune system, mortality,
  privacy, game theory) off-chain into a single RISC0 guest program. The on-chain script
  stays a constant size (~223KB, one STARK verify) no matter how complex the organism's
  internal state machine gets. Full architecture documented in `god_particle/GOD_PARTICLE_COMPLETE.md`.
- **Stage 1 (Hatching) — local proof:** guest enforces Chromosome 1 (identity/chitin) +
  simplified Chromosome 2 (ownership) for stem(0)->vault(1) transitions: covenant_id
  immutable, generation+1, prev_mode continuity, owner-secret hash commitment check.
  DNA blob 163 bytes. **PROOF VERIFIED LOCALLY** — image_id `363bc3b0...`.
- **Stage 2 (The Stinger) — local proof:** extended the SAME guest ELF with a second
  transition path that runs the full proven XMSS/WOTS+ verification (PRF, F-chain,
  Hl combine, L-tree reduction, Merkle authpath walk — previously a ~130KB unrolled
  on-chain script, see Sentinel entries above) natively in Rust INSIDE the STARK circuit.
  Verifies: WOTS+ signature validity, leaf-index-must-equal-next-leaf (no antibody reuse),
  deadline_daa advances by exactly checkin_interval, heartbeat_count+1.
  **PROOF VERIFIED LOCALLY** — image_id `ee147ac7...`, seal 222,668 bytes (real SHA256
  work across 51 chains + Merkle walk actually executing in the zkVM, not just asserted).
- **Bug found & fixed:** first Stage 2 attempt failed XMSS verification — root cause was
  chain-index-to-digit mapping order (checksum digits go on chains 0-2, message digits on
  chains 3-50, not message-first). Matched the proven Python signer's exact convention.
- Code: `god_particle/scorpion_brain/` (guest = single ELF, `host/src/main.rs` = Stage 1
  proof, `host/src/bin/checkin.rs` = Stage 2 proof), `god_particle/scorpion_brain_stage2_keygen/`
  (witness generator, reuses proven `x402-kaspa`/`core` XMSS primitives).
- **Not yet deployed to mainnet** — both proofs are local-only so far. Next: Stage 3
  (privacy carapace — Merkle+nullifier+Groth16-or-STARK inside the same guest), then wrap
  in a P2SH covenant and put a real heartbeat on mainnet.

## 7b. God Particle — Stage 3 "The Carapace" (privacy pool logic, natively inside the STARK)
- Extended the SAME guest ELF again (now handles 3 transition kinds: 0=owner-secret,
  1=XMSS check-in, 2=privacy withdrawal). DNA grew to 232 bytes with Chromosome 4
  (merkle_root, nullifier_hash, nullifier_count, tree_depth).
- **Real logic, no external circuit:** guest reimplements a sha256 Merkle-membership
  proof (leaf = sha256(secret||nonce), walked up an arbitrary-depth auth path) plus
  nullifier derivation (sha256("NULL"||secret)) natively in Rust. The STARK itself
  is the ZK proof of "I know a valid deposit and haven't revealed which one" — no
  nested Groth16/circom circuit needed, unlike the earlier standalone privacy_pool build.
- **PROOF VERIFIED LOCALLY on the first attempt** (no bugs this time) — image_id
  `2c30176a...` (this is the new combined-guest image id, superseding Stage 1/2's
  earlier ids since all three transitions now live in one evolving ELF), seal 222,668
  bytes. Test: 4-leaf tree (h=2), withdrew leaf 0, proved membership + nullifier reveal.
- Code: same `god_particle/scorpion_brain/` tree — `host/src/bin/withdraw.rs` is the
  new Stage 3 proof binary. Witness gen: `god_particle/scorpion_brain_stage2_keygen/gen_stage3_witness.py`.
- **Not yet deployed to mainnet.** Three brains (stem<->vault, sentinel check-in, privacy
  withdrawal) now live as one organism, selected by a transition byte. Next: Stage 4
  (claws — escrow/game theory) per the build order, or move to wrapping the whole thing
  in a real P2SH covenant and putting a heartbeat on mainnet.

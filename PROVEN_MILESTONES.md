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

## 7c. God Particle — Stage 4 "The Claws" (game theory, natively inside the STARK)
- Extended the SAME guest ELF a fourth time (transition=3): courtship dance resolution
  (Stag Hunt) — both players' sealed moves (sha256(move||nonce) commitments) verified
  against revealed moves, payout split computed from a hardcoded choreography table
  (basis points of pot: SS=4500/4500/1000, SH=1000/8000/1000, HS=8000/1000/1000,
  HH=4000/4000/2000, A/B/house), all natively in Rust — no on-chain OpTxOutputAmount/
  OpTxOutputSpk branching script needed (unlike the original Stag Hunt covenant).
  DNA grew to 329 bytes (Chromosome 9: player commits, pot_amount, game_state, 3 payouts).
- **PROOF VERIFIED LOCALLY on the FIRST attempt** — image_id changes again (new
  combined-guest id, now 4 transition kinds: 0=owner-secret, 1=XMSS check-in,
  2=privacy withdrawal, 3=game resolve). Test: both players played STAG (SS outcome),
  pot 1.5 KAS equivalent, payouts A=0.675/B=0.675/house=0.15 computed and verified inside
  the STARK.
- Code: `god_particle/scorpion_brain/host/src/bin/resolve_game.rs`, witness gen
  `god_particle/scorpion_brain_stage2_keygen/gen_stage4_witness.py`.
- **Four brains, one organism**, all local-proof-only so far: stem<->vault, sentinel
  check-in, privacy withdrawal, game resolve — selected by a transition byte in one
  evolving RISC0 guest ELF. Not yet deployed to mainnet.

## 8. Rollup practice — batching N chained state transitions into ONE STARK proof
- Separate small project (`god_particle/scorpion_rollup_practice/`), not part of the
  scorpion brain. Practices the core rollup/Based-App pattern (KIP-21 lanes, Igra-style
  L2 settlement): instead of one proof per state transition, a SINGLE guest execution
  loops through N chained transitions (here: N=4 stem<->vault moults using the same
  owner-secret-authorized logic as Stage 1), enforcing that each link's old_dna equals
  the previous link's new_dna byte-for-byte, and only commits the BATCH's start hash,
  end hash, and length to the journal — no intermediate state ever needs verifying
  on L1. This is the same principle a real rollup uses to settle thousands of L2 txs
  with one L1-verified proof.
- **PROOF VERIFIED LOCALLY on the FIRST attempt.** Chain: generation 100->104,
  stem->vault->stem->vault->stem. journal = batch_start_hash(32) + batch_end_hash(32)
  + batch_size(4, u32 LE).
- Motivation: with Igra Labs building a full ZK-verified EVM L2 on Kaspa and KIP-21
  lanes designed for exactly this batching pattern, this was a deliberate practice
  exercise before attempting anything at that scale — confirms the "N transitions,
  1 proof" mental model works end-to-end with our own tooling.

## 7d. God Particle — Stage 5 "The Nervous System" (data-driven route table, natively inside the STARK)
- Extended the SAME guest ELF a fifth time (transition=4): mode transitions are now
  ALSO authorizable via Merkle-inclusion in `route_root` (Chromosome 10), not just the
  hardcoded `mode_transition_allowed()` match table. The owner can wire in new legal
  moults just by publishing a new `route_root` — no guest redeploy needed. DNA grew to
  458 bytes (route_root, self_template, peer_count, 2 peer_ids).
- **Deliberately proved a route NOT in the hardcoded allow-list**: (mode 4 GAME -> mode 0
  STEM) is illegal under the old hardcoded table, but legal here purely because it's
  leaf 3 of a 4-leaf test route tree. **PROOF VERIFIED LOCALLY, first attempt.**
- **Negative test:** corrupted the Merkle auth-path direction bits and reran — guest
  correctly panicked ("route not found in route_root"), confirming the check isn't
  vacuous.
- Code: `god_particle/scorpion_brain/host/src/bin/route_moult.rs`, witness gen
  `god_particle/scorpion_brain_stage2_keygen/gen_stage5_witness.py`.
- **Five brains, one organism**, all local-proof-only so far: stem<->vault/owner-secret,
  XMSS check-in, privacy withdrawal, game resolve, and now route-validated moult —
  selected by a transition byte in one evolving RISC0 guest ELF. Not yet deployed to
  mainnet.

## 7e. God Particle — Stage 6 "Reproduction" (the gonads, parthenogenesis fanout, natively inside the STARK)
- Extended the SAME guest ELF a sixth time (transition=5): the parent covenant spawns
  N fresh child covenants in ONE proof (true fanout, not one tx per child). Each
  child gets its own chitin (`covenant_id = sha256(parent_covenant_id || index)`,
  deterministically derived and verified in-circuit), is born at generation 0 in
  STEM (resting) mode, inherits the parent's owner_commitment (asexual/
  parthenogenesis), and has `max_generations` reduced by exactly 1 from the parent's
  (telomere shortening -- prevents runaway/cancerous reproduction; once
  max_generations hits 0 a lineage is sterile). DNA grew to 493 bytes (Chromosome 6:
  max_children, child_template, max_generations, children_born).
- **PROOF VERIFIED LOCALLY, first attempt.** Test: parent (gen 5, 3 generations of
  fertility left, litter cap 2) spawns 2 children, parent's children_born goes 0->2,
  each child correctly derived/verified.
- **Negative test:** deliberately set a child's max_generations to the parent's value
  instead of parent-1 (skipping the telomere decrement) and reran -- guest correctly
  panicked ("child's reproductive depth must be exactly the parent's minus 1"),
  confirming the check isn't vacuous. Restored the correct file after.
- Code: `god_particle/scorpion_brain/host/src/bin/spawn_children.rs`.
- **Six brains, one organism**, all local-proof-only so far: stem<->vault/owner-secret,
  XMSS check-in, privacy withdrawal, game resolve, route-validated moult, and now
  reproduction/fanout -- selected by a transition byte in one evolving RISC0 guest ELF.
  Not yet deployed to mainnet.

## 7f. God Particle — Stage 7 "Memory" (the ventral ganglia, the incarnation clause)
- Chromosome 7 added: prev_state_hash(32) + transition_count(4) + creation_daa(8) +
  creation_txid(32) = 76 bytes. DNA grew from 493 -> 569 bytes.
- GLOBAL rule now enforced across ALL 6 transition kinds: every new_dna must
  hash-link backward to the exact old_dna it came from
  (`new.prev_state_hash == sha256(old_dna_bytes)`), transition_count advances by
  exactly 1, and creation_daa/creation_txid (the birth certificate) are immutable
  once set -- verified by re-proving Stage 1 (owner-secret moult) end-to-end with
  the new global check active, no regression.
- The deliberate exception, and the actual point of this stage: a child spawned
  during reproduction (transition=5) does NOT continue the parent's memory chain.
  It starts a wholly new one -- transition_count=0, its own distinct creation_daa/
  creation_txid (a fresh birth certificate, provably not a copy of the parent's).
  But its one unerasable, unforgeable first memory is `sha256(parent_dna_at_the_
  moment_of_spawning)` -- an exact, checked link to the precise state it came from.
- **PROOF VERIFIED LOCALLY**, reproving the Stage 6 reproduction case with full
  Chromosome 7 semantics active (parent continues its own chain normally; both
  children get old_hash as their first memory + distinct fresh birth certificates).
- **Negative test:** deliberately fed a child a fake first memory (the parent's
  own creation_txid, instead of the true old_dna hash) -- guest correctly
  panicked ("child's first memory must be the parent's exact state at the moment
  of spawning"). A false claim of origin does not verify. Restored the correct
  file after.
- Code: `append_memory()` + `sha256_bytes()`/`genesis_txid()` helpers added to
  all 6 host binaries; guest global memory-chain assertions + child-specific
  incarnation checks in `methods/guest/src/main.rs`.
- Seven chromosomes of the God Particle genome now proven inside one evolving
  RISC0 guest ELF: identity, ownership, XMSS check-in, privacy withdrawal, game
  resolve, route-validated moult, reproduction+memory. All local-proof-only so
  far -- nothing deployed to mainnet yet.
- Companion document: `god_particle/GOSPEL_OF_THE_SCORPION.md` -- a plain-
  language teaching mapping each proven stage to what it actually means, written
  for anyone (not just engineers) to read, alongside the verifiable source.

## 7g. God Particle — Stage 8 "The Pedipalps" (Chromosome 8, ESCROW / the arbiter, "go find help")
- Chromosome 8 added: arbiter_commit(32) + buyer_commit(32) + seller_commit(32) +
  dispute_deadline(8) + escrow_amount(8) + winner(1) + dispute_state(1) = 114 bytes.
  DNA grew from 569 -> 683 bytes. New MODE_ESCROW=5, wired into mode_transition_allowed
  (STEM->ESCROW to open, ESCROW->ESCROW to resolve).
- New transition kind (transition=6): the claws grip disputed value between two parties
  who can't resolve it alone. Two ways out, both proven:
  - **Help arrives:** a trusted third party (the arbiter) proves knowledge of their own
    secret (hash-preimage of arbiter_commit) and hands down a decision (buyer or seller
    wins). This is the one place in the whole organism where a party OUTSIDE
    owner/buyer/seller has any say at all.
  - **Help never comes:** the dispute_deadline lapses, and the buyer is protected by a
    safe, permissionless, zero-signature default -- no arbiter needed.
- **PROOF VERIFIED LOCALLY, both paths, first try:** arbiter-resolved case (seller
  awarded, current_daa < deadline) and timeout case (buyer refunded by default,
  current_daa > deadline) both pass independently.
- **Negative test:** fed an impostor's fake secret instead of the real arbiter's --
  guest correctly panicked ("arbiter_secret does not match the arbiter_commit on
  record"). A false helper cannot resolve the dispute. Restored the correct file after.
- Followed the mandatory key-backup rule: arbiter/buyer/seller secrets generated via
  os.urandom and saved to `kaspa_wrpc/keys/scorpion_stage8_escrow_secrets.json` BEFORE
  any proving script touched them.
- Code: `host/src/bin/resolve_escrow.rs` (new host binary, `--mode=arbiter|timeout`),
  guest transition=6 branch in `methods/guest/src/main.rs`.
- Eight chromosomes of the God Particle genome now proven inside one evolving RISC0
  guest ELF: identity, ownership, XMSS check-in, privacy withdrawal, game resolve,
  route-validated moult, reproduction+memory, escrow/arbiter. All local-proof-only so
  far -- nothing deployed to mainnet yet.

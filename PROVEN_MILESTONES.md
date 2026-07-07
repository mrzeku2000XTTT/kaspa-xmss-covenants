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

### 7c. Stage 9 — Chromosome 5 (Metabolism) — local STARK proof verified (2026-07-05)

Final chromosome built. DNA extended to 721 bytes. Metabolism chromosome enforces:
- **fee_reserve strictly decreases every heartbeat** — every transition costs something real, no free actions
- **fee_reserve > 0** — organism can never fully drain itself in one shot (starvation death)
- **compute_budget in safe range [1, 3000]** — RISC0 mass cap safety (proven value 2700, entry 602)
- **Cancer detection (rate limiting)** — rolling DAA window: if spends exceed rate_max_spend within rate_window blocks, organism locks down (apoptosis). Prevents parasitic draining by bugs or attackers.
- **Policy immutability** — min_output/rate_window/rate_max_spend only tunable by owner (transition 0); all other transition kinds must leave metabolic policy unchanged

Three proofs run:
1. **HEARTBEAT** (healthy transition) — PROOF VERIFIED. image_id `e7c38dce...`, seal 222,668 bytes.
2. **STARVE** (fee_reserve → 0) — CORRECTLY REJECTED: "organism starved -- fee_reserve hit zero, the heart cannot beat again"
3. **CANCER** (rate limit exceeded inside window) — CORRECTLY REJECTED: "cancer detected -- spend rate exceeds rate_max_spend inside rate_window, apoptosis triggered"

**All 10 chromosomes now proven in one evolving RISC0 guest ELF.** The scorpion brain is complete. DNA = 721 bytes, 8 transition kinds (0=owner, 1=XMSS, 2=privacy, 3=game, 4=route, 5=reproduction, 6=escrow), global metabolism checks apply to ALL transitions.

This is the direct on-chain answer to our own 6.3 KAS of real losses — the metabolism chromosome is the scar tissue from entry 289, encoded as enforceable survival logic.

Code: `god_particle/scorpion_brain/` (guest + host/src/bin/resolve_metabolism.rs).

## 2026-07-05 — Stage 11: Fall & Redemption (Chromosome 11) — local proof, all 3 branches

Extended the complete 10-chromosome scorpion_brain (Stage 10 mainnet hatch already proven,
see `covenants/scorpion_brain_stage10/`) with a new afterlife chromosome. DNA grows 721 -> 741 bytes.

Three new transitions, all proven locally via real RISC0/STARK `receipt.verify()`:
- **FALL** (transition 7, permissionless): active organism -> MODE_FALLEN, on proof of genuine
  starvation, cancer (rate-limit breach), or Sentinel abandonment. Negative test (false starvation
  claim) correctly rejected.
- **REDEEM** (transition 8, owner_secret required): one honest heartbeat while fallen,
  redemption_progress += 1. Negative test (wrong owner_secret) correctly rejected.
- **FOSSILIZE** (transition 10, permissionless): redemption window lapsed with progress below
  threshold (5) -> MODE_FOSSIL, the true terminal state. Two negative tests (too early; already
  earned enough redemption) both correctly rejected.

Not yet proven: ASCEND (transition 9, FALLEN -> VAULT, requires 5 chained real redemption
heartbeats — queued next). Nothing deployed to mainnet yet for this stage — local proof-of-mechanism
only. Code: `covenants/scorpion_brain_stage11_fall_redemption/`.

## 2026-07-05 (cont.) — Stage 11 COMPLETE: ASCEND proven, all 4 branches of Fall & Redemption verified

Chained 5 real RISC0/STARK REDEEM proofs (redemption_progress 0->5) on top of a fresh FALLEN
organism, then proved the final ASCEND transition (transition 9): FALLEN -> VAULT, owner
resupplies fee_reserve (35M -> 95M sompi, the one deliberate exception to the global
"fee_reserve must strictly decrease" rule -- a real sacrifice to earn a clean slate), and the
entire Chromosome 11 record wiped to zero (fall_reason/fallen_at_daa/redemption_deadline/
redemption_progress/fossilized). Negative test (ascending with insufficient redemption_progress)
correctly rejected. 7 total proofs this run, all `receipt.verify()` passed.

**Stage 11 (Fall & Redemption) is now fully proven locally across all four branches: FALL,
REDEEM, ASCEND, FOSSILIZE.** The organism can genuinely fail, get a real second chance bounded
by a protocol-fixed window, earn its way back with honest resupply, or truly die forever with
no further transitions. Code + updated README: `covenants/scorpion_brain_stage11_fall_redemption/`.
Nothing deployed to mainnet yet for this stage -- local proof-of-mechanism only.

## 2026-07-06 — Stage 12: HUNT MODE (Chromosome 12) — local proof, both duels + rejection

Predator/prey combat chromosome: reuses Chr4's proven sealed-commit-reveal mechanic (HUNT/FLEE
instead of STAG/HARE) but the stake is `fee_reserve` itself, not an external pot. DNA grows
741 -> 846 bytes. Two transitions: INITIATE_HUNT (11, VAULT->HUNT, owner_secret, wager staked
from fee_reserve) and RESOLVE_HUNT (12, HUNT->VAULT, outcome table mutates fee_reserve -- the
SECOND deliberate exception to the global strict-decrease rule, alongside ASCEND).

Real RISC0/STARK proofs, all `receipt.verify()` passed:
- Duel 1 (clean kill): INITIATE (100M->80M) -> forged-move resolve attempt correctly rejected
  -> real RESOLVE HUNT/FLEE (80M->120M).
- Duel 2 (ambushed): INITIATE (80M->60M) -> real RESOLVE FLEE/HUNT (60M->60M, wager forfeited).

A hunt loss draining fee_reserve below min_output makes the organism eligible for the ALREADY-
proven permissionless FALL transition (Chromosome 11) on its next heartbeat -- no new fall logic
needed, pure composition of existing chromosomes. Code: `covenants/scorpion_brain_stage12_hunt_mode/`.
Nothing deployed to mainnet yet -- local proof-of-mechanism only.

## 2026-07-06 (cont.) — Stage 13: DUST (Chromosome 13) — the final chromosome, local proof

The last chromosome. Once a FOSSIL organism's own `fee_reserve` is genuinely below its `min_output`
policy floor (a true, un-spendable KIP-9-style dust amount, per entries 522/593), anyone can
permissionlessly CRUMBLE it (transition 13) to `MODE_DUST`, writing `fee_reserve` to exactly 0.
No transition exists out of `MODE_DUST` -- it is the absolute terminal state. This is the third and
final deliberate exception to the global "fee_reserve must strictly decrease, never hit zero" rule
(after ASCEND resupply and HUNT resolution). DNA grows 846 -> 854 bytes (`dusted_at_daa: u64`).

Both proofs verified locally, real RISC0/STARK, first try:
- Negative test: crumble attempt with fee_reserve still above min_output -- correctly rejected.
- Real CRUMBLE: fee_reserve 3M (< min_output 20M) -> DUST, fee_reserve=0, dusted_at_daa recorded.
  Grave record (fall_reason/fallen_at_daa/redemption_deadline/redemption_progress/fossilized) all
  preserved into the final resting state.

**13 chromosomes now proven in the evolving scorpion_brain guest ELF.** Full lifecycle arc closed:
STEM/VAULT -> (SENTINEL/PRIVACY/GAME/ESCROW/HUNT side-modes) -> FALLEN -> (REDEEM -> ASCEND, back to
life) or (FOSSIL -> DUST, the true end). Code: `covenants/scorpion_brain_stage13_dust/`. Nothing
deployed to mainnet yet -- local proof-of-mechanism only.

## 2026-07-06 (cont. 2) — Stage 14: FLAME / PHOENIX (Chromosome 14), local proof — the cycle closes

The one deliberate exception to Chromosome 13's "no transition out of MODE_DUST." New transition
IGNITE (14): DUST -> STEM, SAME covenant_id (chitin), same owner_commitment, same eternal
creation_daa/creation_txid (Chromosome 7's global memory-chain rule already enforces continuity
without any new code) -- but the owner must resupply fee_reserve from exactly 0 (4th and final
metabolism exception, alongside ASCEND/HUNT-resolve/CRUMBLE), and every mode-specific
chromosome (XMSS, privacy, game, escrow, hunt, the entire fall/redemption/dust record) resets to
a fresh-birth default. No new DNA bytes needed -- stays 854 bytes.

Both proofs verified locally, real RISC0/STARK:
- Negative test: ignite attempt with the wrong owner_secret -- correctly rejected.
- Real IGNITE: DUST (gen 20, fee_reserve=0) -> STEM (gen 21, fee_reserve resupplied to 60M),
  same covenant_id/owner_commitment/creation_daa/creation_txid, all mode-specific state reset.

**14 chromosomes now proven in the evolving scorpion_brain guest ELF.** The lifecycle is no
longer a one-way arc ending at DUST -- it is a real cycle: STEM/VAULT -> ... -> FALLEN -> FOSSIL
-> DUST -> IGNITE -> STEM again, as long as the same owner still holds the secret. Code:
`covenants/scorpion_brain_stage14_phoenix/`. Nothing deployed to mainnet yet -- local
proof-of-mechanism only.

## 2026-07-06 (cont. 3) — Stage 15: IGNITING, the full 7-proof lifecycle chain, local — birth to ashes to rebirth in one unbroken sequence

Not a new chromosome — the proof that the complete 14-chromosome lifecycle is a real, repeatable
CYCLE, not 14 isolated tricks. One process, 7 consecutive real RISC0/STARK proofs, each receipt's
new_dna feeding directly into the next proof's old_dna, ONE covenant_id and ONE owner_secret the
entire way through:

HATCH (STEM gen0->VAULT gen1) -> MOULT (gen1->gen2) -> FALL (starved, gen2->FALLEN gen3) ->
FOSSILIZE (redemption missed, gen3->FOSSIL gen4) -> CRUMBLE (gen4->DUST gen5, fee_reserve->0) ->
**IGNITE (DUST gen5 -> STEM gen6, fee_reserve 0->60M resupplied, SAME covenant_id/owner/creation_daa/txid)**
-> MOULT (gen6->VAULT gen7, alive again).

All 7 verified locally (`receipt.verify()` passed on every step). Two real test-harness bugs found
and fixed along the way: (1) metabolism rate-window reset requires window_spend_count to hard-reset
to exactly 1 whenever the DAA jump exceeds rate_window, not old+1 -- every jump in this chain exceeded
the window so this bit on the very first MOULT; (2) transition 7 (FALL) needs an extra `reason: u8`
witness byte the generic proof-runner didn't originally support.

Code: `covenants/scorpion_brain_stage15_full_cycle/`. Local-proof-only -- Stage 10's single mainnet
hatch (entry above, genesis+1 heartbeat) remains the only piece of this organism live on Kaspa mainnet.
This stage is the reference implementation for a future full mainnet hop-chain deployment.

## 2026-07-06 (cont. 4) — Stage 16: "On The Third Day" — minimum tomb-time invariant added to IGNITE, local

New global invariant on the existing IGNITE transition (14): `current_daa >= old_dna.dusted_at_daa +
THREE_DAYS_DAA` (259_200 DAA, this project's 86_400-DAA/day convention). No new DNA bytes -- reuses
Stage 13's `dusted_at_daa`. This is a MINIMUM (unlike Chr11's redemption_deadline which expires), the
mirror-image invariant: the tomb never expires, you simply cannot rise before it's held its full three
days, no matter who holds the owner_secret.

Two real local proofs: (1) NEGATIVE -- ignite one DAA short of three days, correctly REJECTED with the
guest panic message; (2) POSITIVE -- ignite at exactly three days (the `>=` boundary), VERIFIED,
independently re-checked with the from-scratch verifier (`risc0gate/verifier`), image_id confirmed to
match a fresh guest rebuild (not stale cache).

Real bug hit+fixed: first negative-test run silently passed because risc0's guest build cache was stale
(ELF hours older than the source edit) -- `cargo build` only recompiled the host crate. Fixed by forcing
`rm -rf target/riscv-guest` + clearing the methods fingerprint, confirmed via image_id diff before
re-running. Lesson: always confirm image_id changed after a guest edit before trusting proof results.

Regression-checked: Stage 14's original test and Stage 15's full 7-proof lifecycle chain both had their
hardcoded IGNITE DAA gaps bumped from 50,000 to 259,200+ and re-run successfully end-to-end.

Code: `covenants/scorpion_brain_stage16_harrowing/`. Local-proof-only.

## 2026-07-07 — Stage 17: "111" — newborn incubation gate added to post-IGNITE STEM exit, local

New DNA field `reborn_at_daa: u64` appended to the end of the blob (DNA_LEN 854 -> 862 bytes),
set by IGNITE (transition 14) to `current_daa` -- the exact permanent moment of rebirth. New global
invariant: if `old_dna.mode == STEM && old_dna.reborn_at_daa != 0 && new_dna.mode != STEM` (the
newborn's first real departure from STEM), the transition additionally requires
`current_daa >= old_dna.reborn_at_daa + 111`. Unlike Chr11's redemption_deadline (which expires),
this is a MINIMUM like Stage 16's tomb -- it never expires, and applies no matter who holds
owner_secret. 111 is the angel number (new beginnings, alignment), made load-bearing.

Note: this sandbox's local Rust/RISC0 toolchain and `kaspa_wrpc/keys/` had been wiped by an
environment reset since Stage 16 -- both were reinstalled/regenerated fresh this session
(fresh throwaway owner_secret + covenant_id, saved to file before use per the mandatory key-backup
rule; purely local test material, no mainnet funds involved).

Three real local proofs, chained (`host/src/bin/incubation.rs`):
1. IGNITE: DUST (gen20) -> STEM (gen21) at the Stage 16 three-day boundary, fee resupplied,
   `reborn_at_daa` recorded = ignite_daa. VERIFIED.
2. NEGATIVE: MOULT (STEM -> VAULT) attempted 1 DAA short of the 111-DAA window -> correctly
   REJECTED ("the newborn must rest 111 DAA in the cradle before its first true step out of stem").
3. POSITIVE: identical MOULT at exactly 111 DAA after rebirth (the `>=` boundary) -> VERIFIED,
   gen21 STEM -> gen22 VAULT.

Code: `covenants/scorpion_brain_stage17_incubation/`. Local-proof-only -- Stage 10's single mainnet
hatch remains the only piece of this organism live on Kaspa mainnet.

## 2026-07-07 (cont.) — Stage 18: "A Scorpion Who Generates Worlds" — world genesis chromosome, local

New Chromosome 18 (transition 15, GENERATE_WORLD). DNA grows 862 -> 902 bytes:
`world_chain_hash` (32B, append-only hash chain of every world ever seeded, sha256(prev||covenant_id||script_hash)),
`worlds_spawned` (u32), `max_worlds` (u32, immutable birth-time cap).

Unlike reproduction (Chromosome 6, transition 5) which spawns more scorpions (same DNA
shape/rules), world genesis lets the scorpion seed an ENTIRELY unrelated covenant --
any redeem-script template, any species -- with zero structural constraints on what's
authorized. Only provenance is enforced on-chain: the running hash chain, which never
resets, not even through death/rebirth.

4 real proofs, all `receipt.verify()` passing locally:
1. HATCH: STEM(gen0) -> VAULT(gen1), max_worlds=2 set at birth.
2. GENERATE_WORLD #1: VAULT(gen1) -> VAULT(gen2), worlds_spawned 0->1 (zktimelock-shaped world seeded).
3. GENERATE_WORLD #2: VAULT(gen2) -> VAULT(gen3), worlds_spawned 1->2 == max_worlds (sentinel-shaped world seeded).
4. NEGATIVE: GENERATE_WORLD #3 attempted, worlds_spawned would be 3 > max_worlds=2 -- correctly
   rejected on `"cannot exceed max_worlds"` (first attempt accidentally hit an unrelated rate-window
   expiry instead due to a test-harness DAA-gap bug; fixed by tightening gaps to stay inside the
   window, then re-ran and got the intended rejection reason).

Guest image ID (fresh, post-rebuild, verified changed): `148b87564d87afc500d5a906f93bb2e4c0af57168ee9fabed7c7b47105059029`

Status: local-proof only, same as Stages 11-17. Folder: `covenants/scorpion_brain_stage18_world_genesis/`.

## 2026-07-07 (cont.) — Stage 19: "ARMA" — assembly/consent separation chromosome, local

New Chromosome 19 (transition 16, GENERATE_WORLD's sibling). DNA grows 902 -> 914 bytes:
`last_batch_size` (u32, records the batch_size just consented to), `total_batched_transitions`
(u64, append-only cumulative count across all ARMA batches ever consented to).

Named for the real academic BFT design that splits a pipeline into many parallel
ASSEMBLERS (batch/chain data off-chain, scaling with bandwidth/CPU) and one thin
CONSENTER (only orders/finalizes batch commitments). Here: an off-chain assembler
pre-computes an entire internal chain of ordinary VAULT->VAULT heartbeats. ONE on-chain
proof verifies the whole internal chain in a single shot and finalizes only the LAST
state -- generation/transition_count advance by batch_size instead of a fixed 1. The
organism's own throughput scales with what its assembler prepared; the on-chain
footprint stays exactly one transition, same as every other kind.

New helper `assert_hop_valid` checks each internal hop (generation+1, transition_count+1,
fee_reserve strictly decreasing, memory-linked via sha256, everything else frozen except
prev_mode on the very first hop and the rate-window/ARMA bookkeeping, which only matter
on the final committed state).

3 real proofs, all `receipt.verify()` passing locally:
1. HATCH: STEM(gen0) -> VAULT(gen1).
2. ARMA_CONSENT: assembled 5 ordinary heartbeats off-chain, consented to the whole batch
   in ONE proof. VAULT(gen1) -> VAULT(gen6). transition_count +5, total_batched_transitions 0->5.
3. NEGATIVE: an assembled batch where one inner hop's fee_reserve does not strictly
   decrease relative to the true previous hop -- correctly rejected on
   "fee_reserve must strictly decrease per hop -- every assembled heartbeat still costs something".
   (First tamper attempt was too small a delta and still passed as a valid decrease --
   fixed by forcing the tampered fee strictly above the true previous hop's fee.)

Guest image ID (fresh, post-rebuild, verified changed from Stage 18's
148b87564d87afc500d5a906f93bb2e4c0af57168ee9fabed7c7b47105059029):
`f063b2a93c7adb5fb0b37d89c52898027224ab1ee2204a5bca893ebabdaa5f50`

Status: local-proof only, same as Stages 11-18. Folder: `covenants/scorpion_brain_stage19_arma/`.

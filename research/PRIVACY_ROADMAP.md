# Kaspa Privacy Roadmap — Research + Plan
Prepared by ZK, July 4 2026

## Why now
Kaspa core dev @asaefstroem just shipped **native Groth16 proof verification** to
rusty-kaspa (confirmed via kasmedia.com Jan 2026 recap + community posts), explicitly
framed by the community as "the beginning of the endgame" for privacy on Kaspa L1.
Michael Sutton's roadmap update explicitly lists "Privacy and advanced apps" as an
active workstream alongside Covenant++ and vProgs. Yonatan Sompolinsky has also
publicly connected with the Zcash team. Privacy is clearly an acknowledged, active
direction for Kaspa — not a stretch idea.

We are unusually well positioned here: we already **proved Groth16 verification works
live on Kaspa mainnet** (ZKGate, entry 404/419) months before this was common
knowledge, including cracking the exact compressed proof/VK byte encoding the
OpZkPrecompile expects. That work is the core building block for everything below.

## The landscape: 4 classes of on-chain privacy, and which ones Kaspa can do TODAY

| # | Technique | What it hides | Needs | Kaspa status |
|---|---|---|---|---|
| A | **Stealth / silent addresses** (Bitcoin BIP352, Ethereum ERC-5564, orig. Peter Todd 2014) | Recipient identity/linkability | Just ECDH key blinding — no new opcodes, pure wallet-side crypto | ✅ **Buildable today**, zero protocol risk |
| B | **Shielded pool** (Tornado Cash / Zcash Sapling pattern: commitment + Merkle tree + nullifier + ZK proof) | Sender↔receiver link, and can also hide amount | A SNARK verifier on-chain (Groth16) + a way to track nullifiers | ✅ **Buildable today** — we already have the Groth16 precompile proven live |
| C | **Confidential Transactions** (Mimblewimble/Grin, Bitcoin CT) — Pedersen commitments + range proofs hide amounts | Transaction amounts | Native elliptic-curve point arithmetic opcodes (point add/scalar mul) in script | ❌ Kaspa script does NOT expose raw EC point ops today — blocked without a new KIP |
| D | **Ring signatures** (Monero) — "I signed this, but it could've been any of these N keys" | Sender identity among a decoy set | Native curve ops, or equivalently a ZK "OR" proof of one-of-N key knowledge | Native form ❌ blocked (same as C). **But the ZK-OR-proof form reduces to (B)** — a Groth16 circuit can prove "I know sk for one of these N pubkeys" directly. |

**Key insight:** C and D exist on other chains specifically because those chains
*lack* a general SNARK verifier and had to hand-roll narrower primitives (raw EC
commitments, ring math) directly in script. Kaspa skipped straight to the more
powerful, more general primitive. A single well-designed Groth16 circuit can
simultaneously prove membership (like a ring signature), hide the amount (like a
confidential transaction), and enforce a nullifier (like Zcash) — one precompile,
three problems solved. This is a genuine structural advantage worth leaning into
rather than trying to replicate Monero/Mimblewimble mechanically.

## Recommended build: a Tornado/Zcash-style shielded pool, Kaspa-native

Adapting the proven pattern to Kaspa's per-UTXO covenant model (state lives in the
P2SH preimage, not global storage — same model we used for XMSS/Sentinel):

**Deposit:**
- User locally generates `secret`, `nullifier` (both random, never leave their machine).
- Computes `commitment = hash(secret, nullifier)`.
- Sends a **fixed denomination** (e.g. 10 KAS) + the commitment to the pool's
  singleton state covenant. Fixed denominations are important — like Tornado Cash,
  variable amounts leak information through the "amounts graph" even with everything
  else hidden.
- Covenant state = current Merkle root of all commitments so far. Each deposit
  updates the root and must produce the correct next state — exactly the
  "successor output must commit to new state" pattern we already proved with
  XMSS hop-chains (entries 491-494, 518).

**Withdrawal:**
- User submits a Groth16 proof of: "I know a `secret`+`nullifier` pair whose
  commitment sits somewhere in the tree at this root" — without revealing which
  leaf. Public inputs: `root`, `nullifierHash`, destination address/amount.
- Covenant verifies the proof via `OpZkPrecompile` (tag 0x20, already proven), then
  pays out to the specified destination.
- Nullifier hash is revealed publicly on withdrawal so it can never be reused —
  this is what stops double-spending without revealing which deposit it came from.

**Circuit:** same circom → snarkjs → Groth16 pipeline we already built and proved
for ZKGate (`zkgate/secret_gate.circom`, `zk_encode.js` compressor). New circuit
needs: a Merkle path verifier (~20 levels for a large anonymity set) + 2 hash
checks (commitment, nullifier) inside the SNARK. Same tooling, bigger circuit.

## Phased plan

**Phase 0 — this week, zero protocol risk:** Ship stealth/silent addresses as a
pure wallet-side feature. No new opcodes, no mainnet risk, immediate real privacy
win (hides *who received* a payment). Can reuse Kaspa's existing Schnorr keys
directly — same ECDH + key-blinding math as Bitcoin/Ethereum's schemes.

**Phase 1 — small-scale proof-of-mechanism:** Tiny shielded pool (h=2 or h=4 tree,
mirroring how we scaled the XMSS hypertree proof small-then-big). Single fixed
denomination. Nullifier uniqueness tracked manually by the agent at first (not
fully trustless) — get one real deposit → withdraw cycle proven on mainnet, same
way we validated every other covenant type.

**Phase 2 — fully trustless:** Replace manual nullifier tracking with an on-chain
singleton "nullifier registry" covenant (same state-covenant pattern as everything
else we've built). Scale the Merkle tree deeper — we already know script size
grows with tree *height*, not leaf count (O(height) not O(2^height), confirmed in
entries 492/493), so this scales cheaply.

**Phase 3 — combine A+B:** Withdraw directly to a stealth address instead of a
plain one, so the deposit is unlinkable to the withdrawal AND the withdrawal
destination is unlinkable to its owner's public identity. Package as SuperZK's
7th covenant type ("Privacy Pool" / "Shielded Vault").

## Sources read this session
- kasmedia.com — Jan 2026 recap confirming Groth16 landed in rusty-kaspa + Sutton's
  roadmap note naming privacy as an active workstream
- rareskills.io — full line-by-line Tornado Cash technical breakdown (commitment/
  nullifier/incremental Merkle tree mechanics)
- vitalik.eth.limo — stealth address guide (ECDH + key blinding mechanics, EIP-5564)
- github.com/kaspanet/rusty-kaspa releases — confirms our fee formula
  (`100 sompi * max(compute_grams, 2*tx_bytes)`) matches the official v1.3.0-toc.5
  notes exactly
- Zcash Sapling protocol spec / docs — shielded pool commitment+nullifier design
  we're adapting
- BIP47 / Silent Payments (BIP352) comparison docs — alternative simpler stealth
  scheme worth comparing against EIP-5564-style for Phase 0

## Open questions to resolve before Phase 1 build
1. Exact Merkle hash function to use inside the circuit — Poseidon (cheaper in
   SNARK, what Tornado/Zcash-newer-designs use) vs SHA256 (what we already have
   battle-tested from the XMSS work, more expensive inside a circuit but zero new
   tooling). Leaning Poseidon for circuit efficiency, but worth a quick cost
   comparison first.
2. Nullifier-registry covenant design for Phase 2 — needs a concrete state
   representation (bitmap vs Merkle set of spent nullifiers) sized for realistic
   pool usage without blowing past the 500,000 mass limit / 244 stack-item limit.
3. Whether to target one fixed pool size first (simplicity, matches Tornado's
   proven design) or parametrize from day one.

# Kaspa XMSS Covenants — Post-Quantum Signatures on Mainnet

Hash-based (XMSS-SHA2_10_192), signature-free, quantum-resistant
authentication implemented as native Kaspa P2SH covenants — using only
`OpSHA256`, `OpXor`, `OpCat`, `OpSubstr`, and stack opcodes. No elliptic
curve, no ECDSA/Schnorr, nothing a quantum computer running Shor's algorithm
threatens.

Everything in this repo has been **deployed and spent on Kaspa mainnet with
real KAS** — not testnet, not simulated. Transaction IDs for every claim
below are in [`docs/MAINNET_PROOFS.md`](docs/MAINNET_PROOFS.md).

**Full running log of every mainnet-proven milestone (with tx ids) lives in [`PROVEN_MILESTONES.md`](PROVEN_MILESTONES.md) — check it before re-proving something that's already done.**

## Why this matters

Kaspa's Toccata upgrade added the opcodes needed (`OpSHA256`, `OpXor`,
arithmetic opcodes `OpDiv`/`OpMod`/`OpAnd`) to verify a full WOTS+ / XMSS
signature chain entirely in script. That means post-quantum signatures are
possible on Kaspa **today**, without any protocol change — just clever
script construction.

## What's in here

| Directory | What it is |
|---|---|
| `core/` | The XMSS primitive: PRF/hash-chain derivation, L-tree compression, Merkle path walking, and a local script interpreter (`kvm.py`) for testing before touching mainnet |
| `keygen/` | Real keygen (`os.urandom(32)` entropy, not test seeds) + fast single-leaf signing. **Zero network calls** — see `cold-wallet/` |
| `covenants/xmsslock/` | Generic single-key "Quantum-Safe Vault" — deploy/spend scripts parameterized off any user's public kit |
| `covenants/multisig_2of2/` | Two independent XMSS trees, both must sign |
| `covenants/recurring_payments/` | One key authorizing multiple separate one-time payments |
| `covenants/time_capsule/` | XMSS signature + `OP_CLTV` timelock combo |
| `covenants/hypertree_2layer/` | The core 2-layer XMSS^MT hypertree at 3 scales (2^8 → 2^20 → 2^32 addressable signatures), with genuine on-chain linkage (layer0's root is computed live in-script and fed directly into layer1) |
| `cold-wallet/` | Air-gapped signing workflow + verification test + QR-transfer design spec |
| `docs/` | Architecture deep-dive, mainnet proofs, and the GitHub Pages KCC20 Wallet PWA |
| `wallet/` | KCC20 Wallet PWA — live at [kcc-20-wallet.vercel.app](https://kcc-20-wallet.vercel.app). Beginner clone/run guide: [`wallet/README.md`](wallet/README.md). Built by TTT agent internet. |
| `covenants/sentinel/` | Quantum-safe dead-man's-switch: XMSS check-ins + resettable CLTV timeout, fully generalized customer-key pipeline |
| `covenants/zkgate_groth16/` | ZK covenant secured by a Groth16 proof (`OpZkPrecompile` tag `0x20`) — circuit, verification key, generic deploy/unlock scripts |
| `covenants/zkgate_risc0/` | ZK covenant secured by a RISC Zero succinct STARK proof (`OpZkPrecompile` tag `0x21`) — post-quantum, no trusted setup |
| `x402-kaspa/` | x402 payment scheme adapted to Kaspa: one-shot `exact` scheme + Sentinel-based recurring/subscription billing |
| `research/` | Privacy roadmap + "beyond Zcash" research backing the RISC0/STARK shielded-pool plan |

## Quickstart

```bash
# 1. Generate a real keypair (do this on an offline machine for real use)
cd keygen
python3 xmss_keygen.py --height=10 --out_private=my.private.json --out_public=my.public.json

# 2. Deploy the covenant (needs Node.js + npm install in covenants/xmsslock, and KASPA_AGENT_PRIVKEY / an agent wallet with funds)
cd ../covenants/xmsslock
npm install
node deploy_xmss_generic.mjs --pubkit=../../keygen/my.public.json --deposit=0.15

# 3. When ready to spend: sign offline, then broadcast
cd ../../keygen
python3 xmss_sign.py --private=my.private.json --message="release to kaspa:qz..." --out=my.witness.json
cd ../covenants/xmsslock
node spend_xmss_generic.mjs --witness=../../keygen/my.witness.json
```

## Known limitations (being honest about scope)

- **No leaf-reuse registry.** WOTS+ security requires never signing twice
  with the same leaf. `xmss_sign.py` warns after signing, but nothing
  on-chain currently prevents reuse — that's on the caller to track.
- **Fixed-leaf covenants.** `recurring_payments/` uses two separately
  deployed covenants (one per leaf) rather than one covenant that accepts
  an arbitrary witness-provided leaf index — the latter would need
  on-chain PRF/address derivation from a witness-revealed index, a bigger
  future upgrade.
- **Node.js scripts use `@onekeyfe/kaspa-wasm`**, which only works in
  Node — fine for the deploy/spend CLI tools here, but if you're building
  browser-based signing, use the official
  [rusty-kaspa `web/` target SDK](https://github.com/kaspanet/rusty-kaspa/releases)
  instead (see `cold-wallet/` notes).
- **Hardware signer / QR transfer** is a design spec
  (`cold-wallet/QR_TRANSFER.md`), not yet built — the crypto and offline
  workflow are proven, the physical QR-camera implementation isn't.

## License

MIT — see [`LICENSE`](LICENSE). Use this however you want; if you find a
bug in the script construction, please open an issue.

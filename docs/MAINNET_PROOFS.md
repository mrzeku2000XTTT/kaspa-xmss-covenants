# Mainnet Proofs

Every transaction below was submitted to and accepted by Kaspa mainnet
consensus (`is_accepted: true`) — verify any of them at
`https://kaspa.stream/transactions/<txid>`.

## Core XMSS-SHA2_10_192 single-chain proof of mechanism

- Deploy/spend: `dff62b8fcbc55f413a15f681731b7c1ae774dd738f0826ca35c0d5edace8bc9f`
  — first signature-free, hash-only, post-quantum unlock on Kaspa mainnet.
  Mass: 3,428.

## 2-layer XMSS^MT hypertree (genuine on-chain linkage)

Scaled in three stages — script size barely changes between them, only
off-chain tree-build time does:

| Scale | Deploy TX | Spend TX | Mass |
|---|---|---|---|
| height=4/layer (2^8 addressable) | `2a53d72191d0fe50c17e7c534a7986eb74623034fcd5b63524b60b33c525ea09` | `2fdd40d05a52c5754831de172769fb9ed7baa6068a2d43d7c43b8b09a776ef2b` | 274,820 |
| height=10/layer (2^20 addressable) | `f8fc66bb53e80806a8ed1928d4b54b147b22b9e71be694207b2f19d2ec303f9e` | `2aff3a7539793c5243ceacd8c5e29b540fcacedd08e3196ff4f8d9d8796b5930` | 276,221 |
| height=16/layer (2^32 addressable, final scale) | `2b5e04463cef8735c167edcc91e7a8835b35f5eb998f3d8436588078860eaed4` | **`87dbf3763a619936b08ce608bf36150b60d4c9c9dbbeab72c78c04ccdbd05037`** | 277,621 |

The height=16 spend is the headline proof — 2^32 addressable signatures,
genuinely linked (layer0 root computed live in-script feeds layer1's
message directly, not a hardcoded intermediate).

## Generalized keygen/sign (real random keys, not test seeds)

- Deploy: `e5198d91c7b14773f1e68e8579e96f442f62a18fb6ddcf02eaba4377629c5f50`
- Spend: `7d233cb827d7ecaf0fd198113fa962b048f55017b42fddf8a24c05d641fc023e`
  — height=10/layer, real `os.urandom(32)` secret generated cold, 12.3s
  keygen, 0.28s sign (proving sign time doesn't scale with tree height).
  Mass: 276,218.

## 2-of-2 multisig

- Deploy: `92e384f3134c18a7ca1958e4cdfd42c5af89908ac4325f72a3b546890ac9f413`
- Spend: `7df072ba98ef6d8fb00f808a9e23b526c9b6752287b4ea5d72825127169f10e8`
  — two independent XMSS trees (height=8 each), both signatures required.
  Mass: 275,804.

## Recurring payments (one key, two independent authorizations)

- Payment 1 — Deploy: `eff1db93c2b9b61d6f7a8208b794d10474b75b524367b3cbc0d26a0a61008f66` /
  Spend: `3cbef359d13f45e7e9b422cab7caf5e893fdf051685c4cd1c43149571af9c5f4` (mass 208,278)
- Payment 2 — Deploy: `ffbbdc0c99f6d038af41d583a713f62c1a1ecf8cacc94695f645bf2d8f8e7cfa` /
  Spend: `b1873c9d07bc80d6458af3d3f24d13b353ec904e2daab5026f6310b1c7922201` (mass 208,276)

Cross-leaf witness reuse was tested and correctly rejected before either
deployment touched mainnet.

## Time capsule (XMSS signature + OP_CLTV timelock)

- Deploy: `e9bbc95a370406e3a504fa9109b3cab74238ae028faec3345a69583bfbcab946`
- Spend: `b2a4dc928ce565294cba8a1d75a68ea613cf1ff3c7f5badbe88b5748bc7b61bf`
  (submitted once chain tip passed the embedded unlock DAA score). Mass: 208,516.

## Non-XMSS mainnet work from the same research effort

Not XMSS, but proven on the same consensus engine as part of the broader
"what can Kaspa script verify" research this repo grew out of:

- **RISC0-Succinct (STARK) ZK proof verification** via `OpZkPrecompile`
  (tag `0x21`): deploy `6dfe7e4fa23efe733e522fab68dc523e90cb37578b0ac50405e12962e4669bb4`,
  spend `a2fe7dcfa05bb5fca3895726e7dedf3c8f53884ffdb835284c1d233fb13f11d0`
  (script-unit cost 25,446,371 SU, mass 479,838 — right up against the
  500,000 mass cap). Independently re-proven from a fresh toolchain install
  and a brand new receipt: deploy `65344544fdf576514b8ac91b6e2122a2f80fad25717db93e1a99d10a971dd0eb`,
  spend `0418ce59a80a423a9b93fd33a1966aa9c9bd980fb8bab8aa77614c3b5cfceed8`
  (mass 494,838). Full guest/host project + standalone Rust verifier in
  `covenants/zkgate_risc0/`.
- **Groth16 ZK proof verification** (tag `0x20`) with a fully generalized
  compressed-proof encoder (not tied to one hardcoded proof) — deploy
  `ef1374fc8af6049147262b5d1c1538b0bc543cd199565a3ff49e406968602dcd`, spend
  `43cbe373f978893d90f1a2028c7ab177f7616d7855d58286e464714688041537`.

## Based App workflow (minimal on-chain state machine)

Three chained P2SH covenants, each hop enforcing its successor via
`OpTxOutputSpk`, secured by XMSS (not ECDSA):

- Deploy: `a6a87b71...8fb8c`
- hop0→hop1: `fa666289...7526a`
- hop1→hop2: `acc66b58...db61f`
- hop2→payout: `55bc59cb...8755`

## Stag Hunt (game-theory covenant, branching payouts)

Two players' XMSS-signed moves ("STAG"/"HARE") resolved atomically;
payout split enforced on-chain via `OpTxOutputAmount`+`OpTxOutputSpk`
(no trust in the submitter):

- Deploy: `fe7e421b517f3ff806d084361bd45260853120449b31dba6fc4bbadbdeb1edea`
- Resolve (outcome=SS): `e073e9a12dcd1236f42f2473aee969c957b001fdff53ce0730fd6f83923964ec`

## Sentinel — quantum-safe dead-man's-switch

Generalized pipeline, arbitrary customer keys (`sentinel_keygen.py`,
`covenants/sentinel/`):

- Deploy: `9fe4bbeaeb1e5161e6e580b7ce77e51b77e0e978fe7a0b8d5f40c7a249d53a61`
- Check-in 1 (hop0→hop1): `433b9d9552e79affb84c308928ff5df3badd0f37968fff85c926c8d2e5727c10`
- Check-in 2 (hop1→hop2): `7b54945c632a672ff4e993f45a70e9c96671a4c048c888ef37823eeb6edbd6d7`

Permissionless CLTV/ELSE timeout-release branch (zero signature) also
separately proven: deploy `46df3786eca13d826d19dcd5841ca6418d7c844553924904252a4dfd5bd87ac4`,
check-in `9e0d395e264a4bc52cfc08bc52d16ff0126733e6ff851052180b83a1b55278b7`.

## x402-Kaspa

- One-shot `exact` scheme (signed-but-unbroadcast payment + facilitator
  verify/settle): `96ac9a37c97c887442012ef5a1f4ea503c469e6ab099b3815f02c32fd8a5a550`.
- **Sentinel-x402** — recurring/subscription billing via the Sentinel
  hop-chain repurposed as pay-per-period-with-automatic-refund-on-cancel:
  deploy `305b3f525da2916855b1a37f7f4f0e805f8c7dbc147b690304f39778dd8fa99e`,
  check-in/pay `7167ce8251430058d50f549927f4244d12525aae60c1f096b753df4e1b73e72a`
  (0.3 KAS released to provider, remainder relocked), timeout/refund
  `737cd4100ab2019d2253fb61fc558f56c12fae915ac012db5fdccec9ea631cd0`
  (permissionless, zero signature).

These aren't included in this repo (different project scope — ZK-SNARK
proof verification rather than hash-based signatures) but are mentioned
here as they were validated using the same mainnet, same consensus
constraints (mass cap, stack limit), and same testing discipline.

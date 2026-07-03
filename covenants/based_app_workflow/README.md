# XMSS-Secured Workflow Covenant — a minimal "Based App"

A genuine multi-step, on-chain **state machine**, secured entirely by
quantum-safe (XMSS) signatures instead of ECDSA/Schnorr — a small,
concrete instance of the pattern Kaspa's "Based Apps" roadmap describes
(off-chain-defined state transitions, settled and enforced on L1).

## What it demonstrates

Three chained P2SH covenants (`hop0 -> hop1 -> hop2 -> payout`), where:

- Each hop requires a **different WOTS+/XMSS leaf signature** from the same
  shared Merkle tree — one leaf per state transition, matching the one-time-
  signature security model exactly (a leaf can never be reused, so each hop
  can only ever be spent once, in order).
- Each hop's script **enforces its own successor** via `OpTxOutputSpk` — the
  spender doesn't choose where funds go next; the covenant does. This is the
  actual mechanism that makes it a *state machine* rather than just three
  independent locks: control flow is on-chain, not trusted off-chain.
- The final hop forces payout to a fixed destination address, closing the
  workflow.

This is deliberately the **simplest possible version** of a Based App: no
off-chain executor, no ZK proof of an off-chain VM trace (see "Scope" below)
— every transition is proven directly on L1 by revealing that step's XMSS
witness. It's the settlement/authorization primitive a bigger Based App
would use, in isolation.

## Live mainnet proof

All three transitions are confirmed accepted by Kaspa mainnet consensus:

| Step | TX | Enforced destination |
|---|---|---|
| Deploy (fund hop0) | `a6a87b71814959f850702a76eb71f5f9194b6ccd172e9f57336c178225a8fb8c` | — |
| Hop 0 → Hop 1 | `fa6662891eab4c3e7503f8781a60f7d375500e9613fbbf2619837b7345b7526a` | hop1 covenant address (script-enforced) |
| Hop 1 → Hop 2 | `acc66b58439161c6944f1dbef8621131ace585577dc72e6e580517f717bdb61f` | hop2 covenant address (script-enforced) |
| Hop 2 → Payout | `55bc59cbfc530ab7afa10f5015f55d739799cc9930a31473f4fc742bb6a78755` | final wallet address (script-enforced) |

Verify any of these at `https://kaspa.stream/transactions/<txid>`.

## How the pieces fit together

```
build_workflow.py     Builds the shared 4-leaf XMSS tree (height=2), signs each
                       of the 3 hop messages, and writes workflow_build.json
                       (a real random keypair via os.urandom(32) — see cold-wallet/
                       for how to do this offline).

build_scripts.mjs      Assembles the 3 redeem scripts IN REVERSE ORDER (hop2 first,
                        since each hop must hardcode the NEXT hop's P2SH address,
                        which requires hashing that script first). Writes
                        scripts_out.json with all 3 addresses + script hex.

deploy_workflow.mjs     Funds hop0's covenant address from a wallet (starts the workflow).

spend_hop.mjs           node spend_hop.mjs --step=0|1|2 --txid=... --idx=... --amt=...
                        Reveals that step's XMSS witness, and forwards value to
                        whatever the script enforces (hop N+1, or final payout on step 2).
```

## Extending this pattern

To build a real Based App on top of this:
1. Replace the fixed 3-hop chain with an arbitrary-length one (same trick —
   pre-compute all hop scripts from one shared tree before deploying hop0).
2. Attach real application data to each hop's revealed message (currently
   just a demo string per step — the message-to-digit pipeline already
   supports arbitrary revealed messages, see `docs/ARCHITECTURE.md` in the
   repo root).
3. For genuinely large/complex state machines, this L1-only chaining
   approach doesn't scale — that's exactly the problem
   [KIP-21](https://github.com/kaspanet/kips/blob/master/kip-0021.md) and
   the [vprogs](https://github.com/kaspanet/vprogs) framework solve (an
   off-chain executor + ZK proof + lane-anchored settlement). `vprogs` is
   still early/prototype-stage infrastructure as of this writing — this
   repo's contribution is the quantum-safe authorization primitive that
   such a system could use in place of ECDSA at its settlement layer, not
   a replacement for that infrastructure.

## Note on the example JSON files

`example_workflow_build.public.json` and `example_scripts_out.json` are the
actual artifacts from the mainnet run above, with the private seed
(`sec_seed_hex`) removed — the witnesses shown are already spent/public
(that's expected: XMSS witnesses are one-time-use and safe to reveal once
used). Run `build_workflow.py` yourself to generate a fresh, real keypair
for your own deployment — never reuse the seed from someone else's example.

# Covenants++ — Metered AI Chat (Sentinel-x402, per-customer chains)

An AI chat where **every reply bundle is prepaid on Kaspa mainnet** through a
per-customer sentinel-x402 covenant chain — no subscriptions, no custodian,
no signature-based refunds.

Built from the mainnet-proven primitives in
[`x402-kaspa/sentinel-x402/`](../x402-kaspa/sentinel-x402/) (TX 6e90833e…,
e073e9a1…), generalized from a static 2-hop demo to N-hop per-customer chains.

## The model

| Concept | Implementation |
|---|---|
| One customer | One XMSS leaf set + one covenant chain (sandbox-generated) |
| 1 epoch = 1 bundle | `repliesPerEpoch` AI replies, e.g. 10 |
| Bundle payment | Worker check-in: XMSS-signed spend paying `increment` (0.3 KAS) to treasury, relocking remainder into next hop |
| Final bundle | Last hop pays treasury and returns remainder to the **guest** (no burn) |
| Abandonment | CLTV (7 days/epoch) → permissionless refund to guest, split 2 outputs, zero signatures |
| Guest's key | Browser-only (localStorage), signs only the funding tx — non-custodial |
| AI metering | `InvokeLLM` per HTTP reply; credit decremented only on successful replies |

Session states: `awaiting_covenant → awaiting_fund → live → (exhausted | refunded)`.

## Pieces

- **Guest PWA** — `docs/metered-chat/index.html` (GitHub Pages):
  keygen/import, plan picker, in-browser PSKT-style funding (UTXO fetch →
  createTransactions → createInputSignature → broadcast), chat with live
  credit meter. No server-side key material.
- **Backend API** — `meteredChat` function (Base44): `init` / `status` /
  `funded` / `reply`. The only LLM spender; enforces the meter before invoke.
- **Worker ops** — `meteredChatWorker` function (secret-guarded): `register` /
  `settle` / `abort`. Never callable from the browser.
- **Sandbox worker** — `worker/` (the only Kaspa signer):
  - `build_chain.py` — per-customer XMSS worker seed (**saved to `keys/` FIRST**),
    one layer per session, leaf-per-epoch verify blocks + witnesses (~65 KB each)
  - `build_chain.mjs` — builds N hop scripts bottom-up, derives P2SH addresses,
    registers the chain with the backend (`--register`), **no auto-deploy** —
    the guest funds hop 0
  - `mark_funded.mjs` — verifies the guest funding tx on L1, records the outpoint
  - `checkin.mjs` — XMSS check-in: pay treasury, relock/return remainder,
    advance `current_hop`, settle the backend meter
  - `refund.mjs` — CLTV timeout refund (anyone can run it; zero signatures)

## Cost math (proven values)

```
V(i)   = increment + fee_checkin + V(i+1)     # each check-in relocks V(i+1)
V(last)= increment + fee_checkin + remainder→guest (≥0.3 KAS)
deposit(3 bundles) = 3×(0.3 + 0.4) + 0.3 = 2.4 KAS
refund splits: every output ≥ 0.3 KAS (fee-margin rule)
```

Fees are 0.4 KAS/check-in because each hop script is ~65 KB (XMSS verify
block) — proportional to mass, as proven on mainnet.

## Runbook (operator)

```bash
cd metered-chat/worker
npm install                                    # once

# 1. Guest hits the PWA → init. Operator builds their chain:
export METERED_WORKER_SECRET=$(cat /app/kaspa_wrpc/keys/meteredchat_worker_secret.hex)
node build_chain.mjs --session=mc_xxx --customer=kaspa:qy… --epochs=3 --register

# 2. Guest funds hop0 in-browser (or manually). Operator confirms:
node mark_funded.mjs --session=mc_xxx --txid=<funding txid>
# (guest PWA also self-reports via `funded`)

# 3. Bundle exhausted (402 needsCheckin) → operator check-in:
node checkin.mjs --session=mc_xxx

# 4. Abandoned session → after CLTV, anyone:
node refund.mjs --session=mc_xxx
```

## Security notes

- Worker seed: `keys/meteredchat_<session>_workerseed.hex` — sandbox-only,
  never committed (gitignored), saved **before** any other build step
  (mandatory key-backup rule).
- `chain_build_*.json` holds one-time XMSS witnesses — sandbox-only (gitignored).
- The backend holds **no key material at all**; the entity stores only a slim
  chain summary (addresses, values, deadlines) — full scripts stay in the sandbox.
- Refunds are trustless: the ELSE branch needs no signature, so the guest can
  always claw back their unspent value after the deadline, even if the
  operator disappears.

## Status

Prototype E2E-tested against mainnet-derived addresses with a real backend
session (build → register → funded → metered reply → credit decrement).
Mainnet funding/check-in/refund reuse the sentinel-x402-proven transaction
shapes unchanged.

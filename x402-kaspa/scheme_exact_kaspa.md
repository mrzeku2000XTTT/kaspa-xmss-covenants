# Exact Payment Scheme for Kaspa (`exact`)

This document specifies the `exact` payment scheme for the x402 protocol on Kaspa mainnet (post-Toccata).

Status: **prototype, mainnet-verified**. Not yet an official x402-foundation submission.

## Scheme Name
`exact`

## Network
`kaspa:mainnet` (Toccata-activated, DAA ≥ 474,165,565)

## Why Kaspa is a natural fit for x402

Every existing `exact` scheme implementation (EVM, Solana, Starknet) has to solve a problem Kaspa
doesn't have: a plain on-chain transfer on those chains either costs gas the buyer must pre-fund,
or requires a separate meta-transaction / authorization standard (EIP-3009, Permit2, paymaster
sponsorship) so the seller/facilitator can submit it without the buyer broadcasting first.

Kaspa's UTXO model sidesteps this entirely:

- A normal, fully-signed Kaspa transaction **is** a valid one-time payment authorization the moment
  it's signed — it doesn't need to be broadcast to exist.
- Because it spends a specific UTXO, it is natively replay-proof: that UTXO can be consumed by
  this exact transaction and no other, ever. No nonce bookkeeping required.
- Kaspa's BPS post-Crescendo means settlement confirms in ~1-10 seconds — faster than most L1/L2
  combinations used by existing x402 schemes.

So the Kaspa `exact` scheme needs **no new authorization standard** — the payload IS a standard
signed Kaspa transaction, just not yet submitted to the network.

## Protocol Flow

1. **Client** sends an HTTP request to a **Resource Server**.
2. **Resource Server** responds `402 Payment Required` with a `PAYMENT-REQUIRED` header:
   ```json
   {
     "scheme": "exact",
     "network": "kaspa:mainnet",
     "payTo": "kaspa:qq...",
     "amount": "30000000",
     "asset": "KAS",
     "extra": { "minConfirmations": 1 }
   }
   ```
3. **Client** builds a real Kaspa transaction spending one of its own UTXOs, with an output paying
   `payTo` exactly `amount` sompi, and signs it locally with its private key. It does **not**
   broadcast this transaction.
4. **Client** serializes the signed transaction (`serializeToSafeJSON()` — contains signatures,
   never the private key) and resubmits the original HTTP request with a `PAYMENT-SIGNATURE`
   header carrying that payload.
5. **Resource Server** forwards the payload to a **Facilitator**'s `/verify` endpoint.
6. **Facilitator** parses the transaction and checks:
   - an output exists paying `payTo` exactly `amount`
   - the referenced input UTXO(s) are real and currently unspent (via node/indexer query)
   - the transaction is well-formed and its signature is structurally valid
7. If verification passes, **Resource Server** calls the **Facilitator**'s `/settle` endpoint, which
   deserializes the transaction (`Transaction.deserializeFromSafeJSON`) and submits it directly to
   a Kaspa node via `rpc.submitTransaction()`.
8. **Facilitator** waits for the transaction to be accepted (Kaspa confirms fast) and returns the
   real `txId` to the **Resource Server**.
9. **Resource Server** returns `200 OK` with the resource and a `PAYMENT-RESPONSE` header containing
   the settlement `txId` (independently verifiable by anyone on a Kaspa explorer).

## Facilitator never holds funds or keys

The facilitator only ever handles a transaction the buyer already fully signed. It cannot alter the
destination or amount (any change would invalidate the buyer's signature), and it never sees or
needs a private key. This matches the x402 facilitator's designed trust model exactly.

## Mainnet proof (prototype validation)

Ran the full verify+settle flow against Kaspa mainnet:
- Client-signed, unbroadcast payload built with `serializeToSafeJSON()` (no private key present).
- Facilitator parsed it, verified the exact output amount, then broadcast via `submitTransaction()`.
- Result: TX `96ac9a37c97c887442012ef5a1f4ea503c469e6ab099b3815f02c32fd8a5a550`, confirmed
  `is_accepted: true` on mainnet, exact expected amount delivered.

Reference implementation: `client_sign_payment.mjs` (buyer) + `facilitator_verify_and_settle.mjs`
(facilitator), both using the official `@onekeyfe/kaspa-wasm` bindings against Kaspa mainnet RPC.

## Open items before this could be a real x402-foundation submission

- HTTP-layer resource-server reference implementation (Express/Fastify middleware, matching the
  `paymentMiddleware()` pattern other chains use).
- Formal JSON schema for the `PAYMENT-REQUIRED` / `PAYMENT-SIGNATURE` / `PAYMENT-RESPONSE` headers
  matching the x402 v2 transport spec exactly.
- A hosted facilitator service (this prototype runs the facilitator as a local script; a real
  deployment needs it as a standing HTTP service with its own funded fee wallet for priority fees).
- Toccata-native extension: a `covenant-metered` scheme using P2SH state covenants for prepaid,
  streaming micropayments (deposit once, decrement balance per request, enforced entirely on-chain)
  — something no EVM/SVM x402 scheme can do today, since none of those chains have Kaspa's covenant
  model. This would be a genuine Kaspa-only differentiator, not just parity with existing chains.

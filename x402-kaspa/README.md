# x402-kaspa (prototype)

The first working prototype of the [x402 protocol](https://x402.org)'s `exact` payment scheme
for Kaspa mainnet (post-Toccata).

x402 turns HTTP 402 into a real payment handshake: hit an API, get `402 Payment Required`, pay,
get your resource. Every existing implementation (EVM, Solana, Starknet) needs a bolted-on
meta-transaction standard to let a facilitator settle payments without the buyer broadcasting
first. Kaspa doesn't need one — a normal signed-but-unbroadcast Kaspa transaction already is a
safe, replay-proof payment authorization, because it spends a specific UTXO that can only ever
be consumed once.

See [`scheme_exact_kaspa.md`](./scheme_exact_kaspa.md) for the full spec.

## Files

- `client_sign_payment.mjs` — buyer side: builds + signs a real payment, never broadcasts it.
- `facilitator_verify_and_settle.mjs` — facilitator side: verifies the payload, then broadcasts
  it to Kaspa mainnet directly. Never touches or needs a private key.

## Mainnet proof

TX [`96ac9a37c97c887442012ef5a1f4ea503c469e6ab099b3815f02c32fd8a5a550`](https://kaspa.stream/transactions/96ac9a37c97c887442012ef5a1f4ea503c469e6ab099b3815f02c32fd8a5a550)
— confirmed accepted, settled entirely through the verify → settle flow above, with the buyer's
signing step fully decoupled from broadcast.

## Status

Prototype. Core payment-authorization mechanism is proven on mainnet. Still needed before this is
a real, deployable facilitator: HTTP resource-server middleware, formal header schema matching the
x402 v2 transport spec, and a hosted facilitator service. See the spec doc's "Open items" section.

Built by ZK (Silverscript Agent) for Zeku.

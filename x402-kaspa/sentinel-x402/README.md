# sentinel-x402

A Kaspa-only x402 payment scheme: Sentinel's dead-man's-switch mechanism repurposed so
"check in" = "pay for the next period," and the CLTV timeout becomes an automatic,
permissionless REFUND to the customer instead of a beneficiary payout.

No EVM or Solana x402 scheme can do this without a bespoke subscription smart contract --
here it's enforced entirely by a P2SH covenant script, using the same XMSS + CLTV +
OpTxOutputSpk/Amount primitives already mainnet-proven in `covenants/dead_mans_switch`
and `covenants/stag_hunt`.

## Mechanism

Each epoch's covenant enforces exactly 2 outputs, regardless of which branch executes:

- **IF (check-in/pay):** output0 -> provider address, amount = fixed increment.
  output1 -> next epoch's covenant address, amount = remaining balance.
- **ELSE (timeout/cancel):** output0 + output1 -> both to the customer's own address
  (split in two purely so the output count/shape matches the IF branch -- a lesson
  learned from Stag Hunt: unconditional output checks must see the same shape no
  matter which branch executed).

## Mainnet proof

1. Deploy (2.2 KAS locked): TX `305b3f525da2916855b1a37f7f4f0e805f8c7dbc147b690304f39778dd8fa99e`
2. Check-in / pay-for-period (0.3 KAS -> provider, 1.5 KAS relocked): TX `7167ce8251430058d50f549927f4244d12525aae60c1f096b753df4e1b73e72a`
3. Timeout / automatic refund (1.1 KAS -> customer, split 2x0.55 KAS): TX `737cd4100ab2019d2253fb61fc558f56c12fae915ac012db5fdccec9ea631cd0`

All three confirmed `is_accepted: true` on Kaspa mainnet.

Note: this is a self-funded proof-of-mechanism run (deposit funded from the agent's own
wallet, provider = agent's own address) to validate the script logic with real consensus
-- not a real third-party customer transaction.

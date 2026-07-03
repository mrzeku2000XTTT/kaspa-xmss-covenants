# Stag Hunt — Game-Theory Covenant (XMSS-secured)

A live, mainnet-verified Kaspa covenant implementing the classic **Stag Hunt** game: two players each commit a signed move ("STAG" = cooperate, "HARE" = defect), reveal both in a single atomic transaction, and the payout splits between them according to the outcome — entirely enforced by consensus, with zero trust in whoever submits the transaction.

Moves are authenticated with **XMSS** (hash-based, post-quantum signatures) instead of ECDSA/Schnorr, reusing the on-chain WOTS+/Merkle verification primitive from this repo's `xmsslock`/hypertree work.

## How it works

- Each player has their own tiny XMSS tree (height=2, 4 leaves: 2 reserved for STAG, 2 for HARE — enough for one game).
- To resolve the game, someone submits a transaction revealing **both** players' XMSS signatures over their chosen move.
- The redeem script verifies each signature independently, recovers which move each player made, and then enforces the payout via `OpTxOutputAmount` (0xc2) + `OpTxOutputSpk` (0xc3) — the network itself checks that output amounts and destinations match the rules for that outcome. No one (including the transaction submitter) can redirect funds or fake an outcome.
- A forged/garbage move is rejected by the script before it ever reaches the payout logic.

### Payout table (this instance, 1.5 KAS pot)

| Outcome | Player A | Player B | House/fee |
|---|---|---|---|
| SS (both cooperate) | 0.60 KAS | 0.60 KAS | 0.30 KAS |
| SH (A cooperates, B defects) | 0.30 KAS | 0.70 KAS | 0.50 KAS |
| HS (A defects, B cooperates) | 0.70 KAS | 0.30 KAS | 0.50 KAS |
| HH (both defect) | 0.55 KAS | 0.55 KAS | 0.40 KAS |

Every branch's smallest output stays comfortably above Kaspa's KIP-9 storage-mass floor (see "Lessons" below) — this matters, see the bug writeup.

## Mainnet proof

- Deploy TX: `fe7e421b517f3ff806d084361bd45260853120449b31dba6fc4bbadbdeb1edea`
- Resolve TX (outcome = SS, both cooperated): `e073e9a12dcd1236f42f2473aee969c957b001fdff53ce0730fd6f83923964ec` — `is_accepted: true`, mass 206014.
- Outputs landed exactly as scripted: Player A +0.6 KAS, Player B +0.6 KAS, house +0.3 KAS.

## Files

- `build_stag_hunt.py` — builds the two players' XMSS trees, computes payout amounts per outcome, and locally simulates all 4 branches + a forgery test before anything touches mainnet.
- `build_scripts.mjs` — assembles the full redeem script (XMSS verify x2 + branch-aware payout enforcement) and derives the P2SH covenant address.
- `deploy_stag_hunt.mjs` — funds the covenant (deposits the game pot).
- `spend_stag_hunt.mjs` — resolves the game for a chosen outcome, submitting both players' witnesses in one transaction.
- `player_wallets.public.json` / `stag_hunt_build.public.json` — sanitized public artifacts (no private keys or secret seeds — those are stripped before publishing, same convention as the rest of this repo).

**Note on this reference implementation:** in this demo, one operator generated both players' keys for testing convenience. For real independent players, each side should run their own local keygen (see `xmsslock/`'s keygen tool for the pattern) and only hand over their public redeem-script contribution + signed witness at reveal time — private key material should never leave a player's own machine.

## Two real bugs found while building this (worth reading if you're building your own covenant)

1. **Missing SPK version prefix.** `OpTxOutputSpk` returns `version(u16, little-endian) + script`, not just the raw script bytes. Forgetting the 2-byte version prefix when constructing your "expected destination" constant makes the check unconditionally unsatisfiable — a genuinely bricked, permanently unspendable covenant. This is exactly what happened on our first deploy attempt (a separate 0.2 KAS test deposit that is now permanently stuck for this reason). Always prefix expected SPK comparison constants with `0x00 0x00`.
2. **KIP-9 storage-mass floor.** Kaspa's anti-spam storage-mass rule effectively rejects any transaction with an output below roughly **0.2 KAS**. A payout branch with a small "loser" amount (a few hundred thousand sompi) spiked the transaction's storage mass past the network's cap. Make sure every possible output in every branch of your covenant clears ~0.3 KAS with real margin — check this at design time, not after a mainnet rejection.

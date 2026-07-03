# Cold Wallet / Air-Gapped Signing

XMSS-SHA2_10_192 is a **hash-based** signature scheme — no elliptic curve
math, no pairing, nothing that benefits from being online. That means key
generation and signing can run entirely on a machine that has never touched
a network, with zero loss of functionality.

## Why this works

`keygen/xmss_keygen.py` and `keygen/xmss_sign.py` have **zero network
imports** — no `socket`, `requests`, `urllib`, `http`, `websocket`. Verify it
yourself:

```bash
grep -rn "socket\|requests\|urllib\|http\.\|websocket" ../keygen/*.py ../core/*.py
# -> no output
```

Or run the included proof, which monkeypatches `socket.socket` to hard-fail
on any use, then runs a full keygen + sign cycle:

```bash
python3 air_gap_test.py
```

If either tool tried to phone home in any way, this would crash with
`NoNetwork` instead of printing `ALL CHECKS PASSED`.

## The recommended cold-wallet flow

1. **Generate keys offline.** Take `keygen/xmss_keygen.py`, `keygen/xmss_sign.py`,
   and `core/` to a machine that is airplane-mode / has no wifi card / has
   never been on the internet. Run:
   ```bash
   python3 xmss_keygen.py --height=10 --out_private=my.private.json --out_public=my.public.json
   ```
   `my.private.json` never leaves that machine. `my.public.json` contains
   `redeem_script_hex` and `master_root_hex` — safe to hand to anyone (it's
   the equivalent of a public key / deposit address).

2. **Deploy on your online machine.** Feed `my.public.json` to
   `covenants/xmsslock/deploy_xmss_generic.mjs --pubkit=my.public.json` to
   create the on-chain covenant. This step needs network access but never
   touches your private key.

3. **Sign offline when you want to spend.** Bring the offline device back
   out, run:
   ```bash
   python3 xmss_sign.py --private=my.private.json --message="whatever you're authorizing" --out=my.witness.json
   ```
   Takes well under a second regardless of tree height — signing only
   derives the one target leaf, it doesn't rebuild the tree.

4. **Broadcast from an online machine.** Carry `my.witness.json` back and
   feed it to `covenants/xmsslock/spend_xmss_generic.mjs`. Only public
   witness data crosses the air gap in this direction — nothing secret.

## Going fully air-gapped: QR transfer

USB drives crossing the gap is fine, but if you want zero physical
connection at all, see [`QR_TRANSFER.md`](./QR_TRANSFER.md) for the data
format to move `public.json`/`witness.json` via QR codes instead.

## Hard rule

**Never regenerate a signature from the same leaf pair.** WOTS+ security
depends on each leaf signing exactly once — `xmss_sign.py` prints a warning
after signing for this reason. There is currently no on-chain enforcement of
this (see main README's "Known Limitations") — it's on you to track which
leaves you've used.

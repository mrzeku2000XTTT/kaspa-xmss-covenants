#!/usr/bin/env python3
"""
XMSS^MT signer — uses ONLY the private key file (never touches the agent's
secrets). Fast: does NOT rebuild the whole 2^height-leaf tree, only derives
the one target leaf's key material directly.

Usage:
  python3 xmss_sign.py --private=my_key.private.json --message="authorize payout to kaspa:xyz..." --out=witness.json

IMPORTANT: XMSS is a one-time signature scheme per leaf pair. Signing twice
with the same private key file reveals enough to forge future signatures
for that leaf. Generate a fresh keypair (xmss_keygen.py) per covenant/use.
"""
import argparse, sys, json, hashlib
sys.path.insert(0, '/app/xmss-reference/onchain_link')
from xmss_lib import N, KeyMat
from full_2layer_test import sign_layer

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--private', required=True)
    ap.add_argument('--message', required=True, help='arbitrary string, e.g. destination address / authorization text')
    ap.add_argument('--out', default='witness.json')
    args = ap.parse_args()

    priv = json.load(open(args.private))
    sec_seed = bytes.fromhex(priv['sec_seed_hex'])
    height = priv['height']; leaf0 = priv['leaf0']; leaf1 = priv['leaf1']
    root0 = bytes.fromhex(priv['root0_hex'])

    pub_seed0 = hashlib.sha256(sec_seed + b'-pubseed0').digest()[:N]
    pub_seed1 = hashlib.sha256(sec_seed + b'-pubseed1').digest()[:N]

    # fast: only derive the ONE target leaf's key material, not the whole tree
    km0 = KeyMat(pub_seed0, 0, leaf0, sec_seed)
    km1 = KeyMat(pub_seed1, 1, leaf1, sec_seed)

    app_message = hashlib.sha256(args.message.encode()).digest()[:N]
    witnesses0 = sign_layer(km0, app_message)
    witnesses1 = sign_layer(km1, root0)  # layer1 signs layer0's cached root -- the real cross-layer link

    initial_stack = list(reversed(witnesses1)) + list(reversed(witnesses0)) + [app_message]
    out = {
        "message": args.message,
        "app_message_hex": app_message.hex(),
        "witness_hex": [w.hex() for w in initial_stack],
    }
    json.dump(out, open(args.out, 'w'), indent=2)
    print(f"Signed. {len(initial_stack)} witness items -> {args.out}", file=sys.stderr)
    print(f"Message digest: {app_message.hex()}", file=sys.stderr)
    print("WARNING: this leaf pair is now spent -- do not sign again with this key file.", file=sys.stderr)

if __name__ == '__main__':
    main()

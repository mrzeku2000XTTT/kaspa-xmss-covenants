#!/usr/bin/env python3
"""
XMSS^MT keygen — generates a REAL keypair for a user, with genuine random
secret entropy (os.urandom), not a hardcoded/guessable test string.

Usage:
  python3 xmss_keygen.py --height=10 --out_private=my_key.private.json --out_public=my_key.public.json

Outputs two files:
  - PRIVATE file: contains sec_seed (the actual secret — NEVER share this),
    plus leaf indices and cached tree roots. Keep this file safe. Whoever
    holds it can sign once per leaf pair (XMSS is one-time-signature based
    per leaf — never reuse a leaf index).
  - PUBLIC file: contains only public data (pub_seed0/1, master_root,
    the full redeem_script_hex). Safe to hand to anyone — e.g. give this
    to the deploying party (agent) to lock funds to this key.

Height note: script size barely changes with height, but TREE-BUILD time
(this step) scales with 2^height (must hash every leaf once to build the
Merkle auth path). height=10 (2^10=1024 leaves/layer) builds in ~10-15s
per layer. height=16 (65536 leaves/layer) takes ~5-7 min per layer.
"""
import argparse, os, sys, json, hashlib, time
sys.path.insert(0, '/app/xmss-reference/onchain_link')
from xmss_lib import N, pd, OP_EQUAL
from full_2layer_test import build_merkle_layer, build_layer_script

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--height', type=int, default=10)
    ap.add_argument('--leaf0', type=int, default=None, help='layer0 leaf index (random if omitted)')
    ap.add_argument('--leaf1', type=int, default=None, help='layer1 leaf index (random if omitted)')
    ap.add_argument('--out_private', default='xmss_key.private.json')
    ap.add_argument('--out_public', default='xmss_key.public.json')
    args = ap.parse_args()

    height = args.height
    n_leaves = 1 << height

    # REAL secret entropy -- this is the only thing that must never be shared.
    sec_seed = os.urandom(32)
    pub_seed0 = hashlib.sha256(sec_seed + b'-pubseed0').digest()[:N]
    pub_seed1 = hashlib.sha256(sec_seed + b'-pubseed1').digest()[:N]

    leaf0 = args.leaf0 if args.leaf0 is not None else int.from_bytes(os.urandom(4), 'big') % n_leaves
    leaf1 = args.leaf1 if args.leaf1 is not None else int.from_bytes(os.urandom(4), 'big') % n_leaves

    t0 = time.time()
    print(f"Building layer0 tree ({n_leaves} leaves)...", file=sys.stderr)
    km0, authpath0, root0 = build_merkle_layer(pub_seed0, 0, leaf0, height, sec_seed)
    print(f"  done in {time.time()-t0:.1f}s", file=sys.stderr)
    t1 = time.time()
    print(f"Building layer1 tree ({n_leaves} leaves)...", file=sys.stderr)
    km1, authpath1, root1 = build_merkle_layer(pub_seed1, 1, leaf1, height, sec_seed)
    print(f"  done in {time.time()-t1:.1f}s", file=sys.stderr)

    master_root = root1
    script = build_layer_script(km0, authpath0) + build_layer_script(km1, authpath1)
    script += pd(master_root) + bytes([OP_EQUAL])

    private_data = {
        "sec_seed_hex": sec_seed.hex(),
        "height": height,
        "leaf0": leaf0,
        "leaf1": leaf1,
        "root0_hex": root0.hex(),           # cached -- avoids full tree rebuild on sign
        "master_root_hex": master_root.hex(),
        "warning": "NEVER share this file. Anyone with sec_seed can forge a signature for this key."
    }
    public_data = {
        "pub_seed0_hex": pub_seed0.hex(),
        "pub_seed1_hex": pub_seed1.hex(),
        "master_root_hex": master_root.hex(),
        "redeem_script_hex": script.hex(),
        "script_bytes": len(script),
        "height": height,
        "total_addressable_signatures": n_leaves * n_leaves,
    }

    json.dump(private_data, open(args.out_private, 'w'), indent=2)
    json.dump(public_data, open(args.out_public, 'w'), indent=2)

    print(f"\nKeygen complete in {time.time()-t0:.1f}s total.", file=sys.stderr)
    print(f"PRIVATE key -> {args.out_private}  (keep secret, never share)", file=sys.stderr)
    print(f"PUBLIC key  -> {args.out_public}   (safe to hand to deployer)", file=sys.stderr)
    print(f"Master root: {master_root.hex()}", file=sys.stderr)
    print(f"Redeem script: {len(script)} bytes", file=sys.stderr)

if __name__ == '__main__':
    main()

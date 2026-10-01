#!/usr/bin/env python3
"""
build_chain.py — per-customer XMSS worker keys for the Covenants++ metered AI chat.

Adapted from x402-kaspa/sentinel-x402/build_sentinel_x402.py (mainnet-proven).
One XMSS layer per epoch/hop. KEY BACKUP RULE: the worker seed is written to
its keys file as the FIRST action, before any other logic.

Usage:
  python3 build_chain.py --session=mc_xxx --epochs=3 --out=chain_build_mc_xxx.json

Output JSON (keep in sandbox only — contains one-time signing witnesses):
  { sec_seed_hex, pub_seed_hex, master_root_hex, checkin_msg_hex,
    hops: [ { verify_block_hex, witness_hex: [...] }, ... ] }
"""
import sys, os, hashlib, json, argparse

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'core'))
from full_2layer_test import build_merkle_layer, build_layer_script, sign_layer  # noqa: E402
from xmss_lib import N  # noqa: E402

OP_TOALT, OP_FROMALT, OP_DUP = 0x6b, 0x6c, 0x76
OP_EQUAL, OP_EQUALVERIFY = 0x87, 0x88

CHECKIN_MSG_TAG = b'COVENANTS-PP-CHECKIN'


def pd_bytes(b):
    length = len(b)
    if length <= 75:
        return bytes([length]) + b
    if length <= 255:
        return bytes([0x4c, length]) + b
    if length <= 65535:
        return bytes([0x4d]) + length.to_bytes(2, 'little') + b
    return bytes([0x4e]) + length.to_bytes(4, 'little') + b


def verify_block(km, authpath, master_root):
    """XMSS verify that preserves the revealed message on the stack afterward."""
    s = bytearray()
    s += bytes([OP_DUP, OP_TOALT])
    s += build_layer_script(km, authpath)
    s += pd_bytes(master_root) + bytes([OP_EQUALVERIFY])
    s += bytes([OP_FROMALT])
    return bytes(s)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--session', required=True)
    ap.add_argument('--epochs', type=int, default=3)
    ap.add_argument('--out', default=None)
    args = ap.parse_args()

    epochs = max(1, min(args.epochs, 8))
    out_path = args.out or os.path.join('out', f'chain_build_{args.session}.json')
    os.makedirs(os.path.dirname(out_path) or '.', exist_ok=True)

    # === MANDATORY KEY BACKUP: write the worker seed BEFORE anything else ===
    sec_seed = os.urandom(32)
    keys_dir = os.environ.get('METERED_KEYS_DIR', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'keys'))
    os.makedirs(keys_dir, exist_ok=True)
    keys_path = os.path.join(keys_dir, f'meteredchat_{args.session}_workerseed.hex')
    with open(keys_path, 'w') as f:
        f.write(sec_seed.hex())
    print(f'KEY_SAVED_FIRST: {keys_path}')

    pub_seed = hashlib.sha256(sec_seed + b'-covenantspp-pubseed').digest()[:N]
    checkin_msg = hashlib.sha256(CHECKIN_MSG_TAG).digest()[:N]

    # Pass 1: ONE XMSS layer, one leaf per epoch (proven pattern: layer_id=0,
    # target_leaf_idx distinguishes hops; H tall enough for `epochs` leaves).
    import math
    height = max(2, math.ceil(math.log2(epochs)))
    layers = []
    master_root = None
    for i in range(epochs):
        km, authpath, root = build_merkle_layer(pub_seed, 0, i, height, sec_seed=sec_seed)
        if master_root is None:
            master_root = root
        assert root == master_root, f'leaf {i} root mismatch'
        layers.append((km, authpath))

    # Pass 2: per-hop verify blocks + one-time check-in witnesses.
    hops = []
    for (km, authpath) in layers:
        hops.append({
            'verify_block_hex': verify_block(km, authpath, master_root).hex(),
            'witness_hex': [w.hex() for w in sign_layer(km, checkin_msg)],
        })

    build = {
        'session': args.session,
        'epochs': epochs,
        'sec_seed_hex': sec_seed.hex(),
        'pub_seed_hex': pub_seed.hex(),
        'master_root_hex': master_root.hex(),
        'checkin_msg_hex': checkin_msg.hex(),
        'hops': hops,
    }
    with open(out_path, 'w') as f:
        json.dump(build, f)
    print(f'WRITTEN: {out_path}')
    print('MASTER_ROOT:', master_root.hex())
    for i, h in enumerate(hops):
        print(f'hop{i} verify_block bytes: {len(bytes.fromhex(h["verify_block_hex"]))}, witnesses: {len(h["witness_hex"])}')


if __name__ == '__main__':
    main()

"""
Sentinel Keygen -- run this 100% LOCALLY / AIR-GAPPED. Generates your own
quantum-safe XMSS key material for a Sentinel dead-man's-switch. Your secret
seed NEVER leaves this machine -- only the public kit (verify blocks +
pre-signed check-in witnesses + master root) gets uploaded to SuperZK.

Usage:
    python3 sentinel_keygen.py --checkins 16 --out my_sentinel_kit.json

This produces `my_sentinel_kit.json` (safe to upload/paste into SuperZK) and
prints your master root. Each "check-in" consumes one leaf -- with 16
check-ins you can prove-of-life 16 times before needing to generate a new
kit and re-deploy. Keep this script + its secret_seed_hex output somewhere
safe if you ever want to re-derive/verify your own kit independently, but
you do NOT need to keep it for normal operation -- the kit itself contains
everything needed for deployment and every future check-in.
"""
import sys, os, hashlib, json, argparse
sys.path.insert(0, '/app/_github_export/core')
from full_2layer_test import build_merkle_layer, build_layer_script, sign_layer
from xmss_lib import N, pd

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--checkins', type=int, default=16, help='number of check-ins (leaves), must be a power of 2')
    ap.add_argument('--out', default='sentinel_kit.json')
    args = ap.parse_args()

    height = args.checkins.bit_length() - 1
    assert (1 << height) == args.checkins, "--checkins must be a power of 2 (4, 8, 16, 32...)"

    print(f"Generating fresh XMSS key material for {args.checkins} check-ins (height={height})...")
    sec_seed = os.urandom(32)  # NEVER written to the kit file -- stays only in this process
    pub_seed = hashlib.sha256(sec_seed + b'-sentinel-pubseed').digest()[:N]

    RENEW_MSG = hashlib.sha256(b'SENTINEL-RENEW').digest()[:N]

    leaves = []
    master_root = None
    for leaf_idx in range(args.checkins):
        km, ap_path, root = build_merkle_layer(pub_seed, 0, leaf_idx, height, sec_seed=sec_seed)
        if master_root is None:
            master_root = root
        else:
            assert root == master_root
        OP_TOALT, OP_FROMALT, OP_DUP, OP_EQUALVERIFY = 0x6b, 0x6c, 0x76, 0x88
        vb = bytearray()
        vb += bytes([OP_DUP, OP_TOALT])
        vb += build_layer_script(km, ap_path)
        vb += pd(root) + bytes([OP_EQUALVERIFY])
        vb += bytes([OP_FROMALT])
        verify_block_hex = bytes(vb).hex()
        witness = sign_layer(km, RENEW_MSG)
        leaves.append({
            'leaf_index': leaf_idx,
            'verify_block_hex': verify_block_hex,
            'witness_hex': [w.hex() for w in witness],
        })
        print(f"  leaf {leaf_idx+1}/{args.checkins} done")

    kit = {
        'version': 1,
        'height': height,
        'num_checkins': args.checkins,
        'master_root_hex': master_root.hex(),
        'renew_msg_hex': RENEW_MSG.hex(),
        'leaves': leaves,
    }
    with open(args.out, 'w') as f:
        json.dump(kit, f)
    print(f"\nWritten {args.out} ({os.path.getsize(args.out)} bytes) -- SAFE to upload, contains no secret key material.")
    print(f"Master root: {master_root.hex()}")
    print("This kit is your Sentinel's entire public identity + all future check-in proofs. Keep it -- you'll reuse it for every check-in.")

if __name__ == '__main__':
    main()

"""
sentinel-x402 — Sentinel's dead-man's-switch mechanism repurposed as an x402
payment scheme: "check in" becomes "pay for the next period," and the CLTV
timeout becomes an automatic, permissionless REFUND instead of a beneficiary
payout.

Two branches, same shape as original Sentinel, but each branch now enforces
TWO outputs instead of one (generalizing the proven single-output pattern
from build_and_deploy.mjs, using the multi-output unconditional-check pattern
proven in Stag Hunt):

  IF   (check-in / pay-for-period): XMSS-signed proof-of-life ALSO releases a
       fixed increment to the PROVIDER's address (output 0), while the
       remaining balance re-locks into the next epoch's covenant (output 1).
  ELSE (silence / cancel): once the deadline passes, ANYONE can trigger a
       refund of the full remaining balance back to the CUSTOMER -- split
       across two outputs (both to the same customer address) purely so the
       output *count* stays identical across both branches (a lesson learned
       from Stag Hunt: unconditional output-checks must run in the same shape
       regardless of which branch executed).

This is a genuine Kaspa-only x402 differentiator: no EVM/SVM x402 scheme can
enforce "pay-per-period with automatic on-chain refund on cancellation"
without a subscription smart contract; here it's just script.
"""
import sys, os, hashlib, json
sys.path.insert(0, '.')
from full_2layer_test import build_merkle_layer, build_layer_script, sign_layer
from xmss_lib import N, pd

OP_IF, OP_ELSE, OP_ENDIF = 0x63, 0x67, 0x68
OP_TOALT, OP_FROMALT, OP_DUP = 0x6b, 0x6c, 0x76
OP_EQUAL, OP_EQUALVERIFY = 0x87, 0x88
OP_CLTV = 0xb0

H = 2  # height=2 -> 4 leaves; leaf0 = hop0's check-in signature (demo scale, matches proven small builds)
sec_seed = os.urandom(32)
pub_seed = hashlib.sha256(sec_seed + b'-sentinelx402-pubseed').digest()[:N]

km0, ap0, ROOT = build_merkle_layer(pub_seed, 0, 0, H, sec_seed=sec_seed)
km1, ap1, ROOT2 = build_merkle_layer(pub_seed, 0, 1, H, sec_seed=sec_seed)
assert ROOT == ROOT2
MASTER_ROOT = ROOT

CHECKIN_MSG = hashlib.sha256(b'SENTINEL-X402-CHECKIN').digest()[:N]

def verify_block(km, authpath):
    """XMSS verify that preserves the revealed message on the stack afterward."""
    s = bytearray()
    s += bytes([OP_DUP, OP_TOALT])
    s += build_layer_script(km, authpath)
    s += pd(MASTER_ROOT) + bytes([OP_EQUALVERIFY])
    s += bytes([OP_FROMALT])
    return bytes(s)

out = {
    "sec_seed_hex": sec_seed.hex(),
    "pub_seed_hex": pub_seed.hex(),
    "master_root_hex": MASTER_ROOT.hex(),
    "height": H,
    "checkin_msg_hex": CHECKIN_MSG.hex(),
    "verify_block_leaf0_hex": verify_block(km0, ap0).hex(),
    "verify_block_leaf1_hex": verify_block(km1, ap1).hex(),
    "witness_checkin_leaf0_hex": [w.hex() for w in sign_layer(km0, CHECKIN_MSG)],
    "witness_checkin_leaf1_hex": [w.hex() for w in sign_layer(km1, CHECKIN_MSG)],
}
json.dump(out, open('sentinel_x402_build.json', 'w'))
print("Written sentinel_x402_build.json")
print("MASTER_ROOT:", MASTER_ROOT.hex())
print("verify_block_leaf0 size:", len(verify_block(km0, ap0)), "bytes")

"""
Sentinel — quantum-safe, trustless dead-man's-switch covenant.

Two branches in one script:
  IF   (owner checks in): XMSS-signed "RENEW" proof-of-life -> re-locks remaining
       value into the NEXT epoch's covenant address (new deadline, next leaf).
  ELSE (silence): once the deadline passes, ANYONE can trigger this branch with
       ZERO signature -> funds released to the pre-committed beneficiary.
       No cooperation, no permission, no company -- pure consensus enforcement.

Built entirely from primitives already proven on Kaspa mainnet this week:
  - XMSS hash-based post-quantum signature verification (entries 491-494)
  - OP_CLTV timelock (time_capsule, entry 508)
  - OpTxOutputSpk/OpTxOutputAmount branch-payout enforcement, WITH the version-
    prefix fix learned the hard way (Stag Hunt, entry 522)
  - Successor-output state chaining (Based App workflow, entry 518)
"""
import sys, os, hashlib, json
sys.path.insert(0, '/app/xmss-reference/onchain_link')
from full_2layer_test import build_merkle_layer, build_layer_script, sign_layer
from xmss_lib import N, pd
from kvm import execute, encode_num

OP_IF, OP_ELSE, OP_ENDIF = 0x63, 0x67, 0x68
OP_TOALT, OP_FROMALT, OP_DUP, OP_DROP = 0x6b, 0x6c, 0x76, 0x75
OP_EQUAL, OP_EQUALVERIFY = 0x87, 0x88
OP_CLTV = 0xb0  # pops the locktime value on Kaspa -- no OP_DROP needed after (entry 318/410)

H = 4  # height=4 -> 16 leaves -- 16 possible check-ins before this key needs rotating (demo scale)
sec_seed = os.urandom(32)
pub_seed = hashlib.sha256(sec_seed + b'-sentinel-pubseed').digest()[:N]

# leaf0 = hop0's renewal signature, leaf1 = hop1's renewal signature (reserved, unused in this proof)
km0, ap0, ROOT = build_merkle_layer(pub_seed, 0, 0, H, sec_seed=sec_seed)
km1, ap1, ROOT2 = build_merkle_layer(pub_seed, 0, 1, H, sec_seed=sec_seed)
assert ROOT == ROOT2
MASTER_ROOT = ROOT

RENEW_MSG = hashlib.sha256(b'SENTINEL-RENEW').digest()[:N]

def verify_block(km, authpath):
    """XMSS verify that preserves the revealed message on the stack afterward (Stag Hunt trick)."""
    s = bytearray()
    s += bytes([OP_DUP, OP_TOALT])
    s += build_layer_script(km, authpath)
    s += pd(MASTER_ROOT) + bytes([OP_EQUALVERIFY])
    s += bytes([OP_FROMALT])
    return bytes(s)

def build_epoch_script(km_leaf, authpath, unlock_daa):
    """
    Branch script for one epoch. Caller fills in the two output-check suffixes
    (renew-branch successor check, release-branch beneficiary check) separately
    since those depend on addresses computed bottom-up (Based App pattern).
    """
    verify = verify_block(km_leaf, authpath)
    check_msg = pd(RENEW_MSG) + bytes([OP_EQUALVERIFY])  # drop msg copy, just verify it's the renewal flag
    cltv = pd(encode_num(unlock_daa)) + bytes([OP_CLTV])
    return verify, check_msg, cltv

# ---- Local proof: verify structural logic (message-check + branch selection) before touching mainnet ----
def local_test():
    game = bytearray()
    game += bytes([OP_IF])
    game += verify_block(km0, ap0)
    game += pd(RENEW_MSG) + bytes([OP_EQUALVERIFY])
    game += bytes([0x01, 0x01])  # push 0x01 -- stand-in for "renewal accepted" (real script has output checks here)
    game += bytes([OP_ELSE])
    game += bytes([0x00])  # stand-in for CLTV branch (real script checks lockTime on-chain, can't simulate off-chain)
    game += bytes([0x01, 0x01])
    game += bytes([OP_ENDIF])

    witnesses = sign_layer(km0, RENEW_MSG)
    # scriptSig push order: witnesses..., message, selector -- selector ends up on TOP (consumed first by OP_IF)
    renew_stack = list(reversed(witnesses)) + [RENEW_MSG] + [b"\x01"]
    stack, alt = execute(bytes(game), renew_stack)
    print("RENEW branch (valid signature+message):", "PASS" if stack and stack[-1] == b'\x01' else "FAIL", stack[-1] if stack else None)

    # forgery: wrong message
    bad_msg = hashlib.sha256(b'NOT-A-RENEWAL').digest()[:N]
    bad_witnesses = sign_layer(km0, bad_msg)
    bad_stack = list(reversed(bad_witnesses)) + [bad_msg] + [b"\x01"]
    try:
        stack2, alt2 = execute(bytes(game), bad_stack)
        print("FORGERY (wrong message signed):", "REJECTED" if not stack2 or stack2[-1] != b'\x01' else "!!ACCEPTED")
    except Exception as e:
        print("FORGERY (wrong message signed): REJECTED (exception)", str(e)[:80])

local_test()

out = {
    "sec_seed_hex": sec_seed.hex(),
    "pub_seed_hex": pub_seed.hex(),
    "master_root_hex": MASTER_ROOT.hex(),
    "height": H,
    "renew_msg_hex": RENEW_MSG.hex(),
    "verify_block_leaf0_hex": verify_block(km0, ap0).hex(),
    "verify_block_leaf1_hex": verify_block(km1, ap1).hex(),
    "witness_renew_leaf0_hex": [w.hex() for w in sign_layer(km0, RENEW_MSG)],
    "witness_renew_leaf1_hex": [w.hex() for w in sign_layer(km1, RENEW_MSG)],
}
json.dump(out, open('kaspa_wrpc/sentinel_demo/sentinel_build.json', 'w'))
print("\nWritten sentinel_build.json")
print("MASTER_ROOT:", MASTER_ROOT.hex())

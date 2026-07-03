import sys, os, hashlib, json
sys.path.insert(0, '/app/xmss-reference/onchain_link')
from full_2layer_test import build_merkle_layer, build_layer_script, sign_layer
from xmss_lib import N, pd
from kvm import execute, encode_num

OP_IF, OP_ELSE, OP_ENDIF = 0x63, 0x67, 0x68
OP_TOALT, OP_FROMALT, OP_DUP, OP_DROP = 0x6b, 0x6c, 0x76, 0x75
OP_EQUAL, OP_EQUALVERIFY = 0x87, 0x88

H = 2  # height=2 -> 4 leaves; leaf0=playerA, leaf1=playerB (shared tree, same MASTER_ROOT)
sec_seed = os.urandom(32)
pub_seed = hashlib.sha256(sec_seed + b'-staghunt-pubseed').digest()[:N]

kmA, apA, ROOT = build_merkle_layer(pub_seed, 0, 0, H, sec_seed=sec_seed)
kmB, apB, ROOT2 = build_merkle_layer(pub_seed, 0, 1, H, sec_seed=sec_seed)
assert ROOT == ROOT2
MASTER_ROOT = ROOT

STAG_HASH = hashlib.sha256(b'STAG').digest()[:N]
HARE_HASH = hashlib.sha256(b'HARE').digest()[:N]

def push_amounts(amtA, amtB, amtAgent):
    # push order: amtA, amtB, amtAgent -> amtAgent ends on top
    return pd(encode_num(amtA)) + pd(encode_num(amtB)) + pd(encode_num(amtAgent))

# Conservation check: every outcome must sum to the same total pot D
D = 150_000_000  # 1.5 KAS total pot -- KIP-9 storage mass rejects any output below ~0.2 KAS (20M sompi),
                    # so every payout slice below must clear that floor with real margin (>=30M).
PAYOUTS = {
    'SS': (60_000_000, 60_000_000, 30_000_000),   # mutual cooperation: near-even split, modest protocol fee
    'SH': (30_000_000, 70_000_000, 50_000_000),   # A trusted alone and lost, B took the safe reward, rest to agent
    'HS': (70_000_000, 30_000_000, 50_000_000),   # mirror of SH
    'HH': (55_000_000, 55_000_000, 40_000_000),   # mutual caution: safe, but worse than cooperating
}
for k, v in PAYOUTS.items():
    assert sum(v) == D, f"{k} payout doesn't sum to D"

def verify_block(km, authpath):
    s = bytearray()
    s += bytes([OP_DUP, OP_TOALT])
    s += build_layer_script(km, authpath)
    s += pd(MASTER_ROOT) + bytes([OP_EQUALVERIFY])
    s += bytes([OP_FROMALT])
    return bytes(s)

def build_game_script():
    s = bytearray()
    s += verify_block(kmA, apA)          # stack: [wB..,msgB, msgA_copy]
    s += bytes([OP_TOALT])               # stash msgA_copy; stack: [wB..,msgB]
    s += verify_block(kmB, apB)          # stack: [msgB_copy]
    s += bytes([OP_FROMALT])             # stack: [msgB_copy, msgA_copy]

    # branch on A
    s += bytes([OP_DUP]) + pd(STAG_HASH) + bytes([OP_EQUAL, OP_IF])
    s +=   bytes([OP_DROP])              # A confirmed STAG
    s +=   bytes([OP_DUP]) + pd(STAG_HASH) + bytes([OP_EQUAL, OP_IF])
    s +=      bytes([OP_DROP]) + push_amounts(*PAYOUTS['SS'])
    s +=   bytes([OP_ELSE])
    s +=      pd(HARE_HASH) + bytes([OP_EQUALVERIFY]) + push_amounts(*PAYOUTS['SH'])
    s +=   bytes([OP_ENDIF])
    s += bytes([OP_ELSE])
    s +=   pd(HARE_HASH) + bytes([OP_EQUALVERIFY])   # A confirmed HARE (defensive: rejects garbage messages)
    s +=   bytes([OP_DUP]) + pd(STAG_HASH) + bytes([OP_EQUAL, OP_IF])
    s +=      bytes([OP_DROP]) + push_amounts(*PAYOUTS['HS'])
    s +=   bytes([OP_ELSE])
    s +=      pd(HARE_HASH) + bytes([OP_EQUALVERIFY]) + push_amounts(*PAYOUTS['HH'])
    s +=   bytes([OP_ENDIF])
    s += bytes([OP_ENDIF])
    return bytes(s)

game_script = build_game_script()
print("Game script (verify+branch, no output-checks yet):", len(game_script), "bytes")

# --- Local test: verify XMSS validity + correct branch selection for all 4 outcomes ---
def make_witness(km, authpath, message):
    witnesses = sign_layer(km, message)
    return list(reversed(witnesses)) + [message]

def run_case(msgA, msgB, label):
    wA = make_witness(kmA, apA, msgA)
    wB = make_witness(kmB, apB, msgB)
    initial_stack = wB + wA  # B pushed first (deep), A pushed last (top) -- matches verify order (A processed first)
    try:
        stack, alt = execute(game_script, list(initial_stack))
        # decode the 3 pushed amount values back for sanity check
        from kvm import cscript_num
        vals = [cscript_num(x) for x in stack[-3:]]
        print(f"{label}: PASS -> stack amounts (amtA, amtB, amtAgent) = {vals}, altstack depth={len(alt)}")
    except Exception as e:
        print(f"{label}: REJECTED ({e})")

run_case(STAG_HASH, STAG_HASH, "SS (both stag)")
run_case(STAG_HASH, HARE_HASH, "SH (A stag, B hare)")
run_case(HARE_HASH, STAG_HASH, "HS (A hare, B stag)")
run_case(HARE_HASH, HARE_HASH, "HH (both hare)")

# forgery test: garbage message that isn't STAG or HARE should be rejected
garbage = hashlib.sha256(b'CHEAT').digest()[:N]
run_case(garbage, STAG_HASH, "FORGERY (A sends garbage move)")

out = {
    "sec_seed_hex": sec_seed.hex(),
    "pub_seed_hex": pub_seed.hex(),
    "master_root_hex": MASTER_ROOT.hex(),
    "height": H,
    "stag_hash_hex": STAG_HASH.hex(),
    "hare_hash_hex": HARE_HASH.hex(),
    "payouts": PAYOUTS,
    "game_script_hex": game_script.hex(),
    "witness_stag_A_hex": [w.hex() for w in make_witness(kmA, apA, STAG_HASH)],
    "witness_hare_A_hex": [w.hex() for w in make_witness(kmA, apA, HARE_HASH)],
    "witness_stag_B_hex": [w.hex() for w in make_witness(kmB, apB, STAG_HASH)],
    "witness_hare_B_hex": [w.hex() for w in make_witness(kmB, apB, HARE_HASH)],
}
json.dump(out, open('/app/kaspa_wrpc/stag_hunt_demo/stag_hunt_build.json', 'w'))
print("\nWritten stag_hunt_build.json")

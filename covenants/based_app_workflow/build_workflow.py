import sys, os, hashlib, json
sys.path.insert(0, '/app/xmss-reference/onchain_link')
from full_2layer_test import build_merkle_layer, build_layer_script, sign_layer
from xmss_lib import N, pd, OP_EQUAL
from kvm import execute

H = 2  # height=2 -> 4 leaves, we use 3 (hop0, hop1, hop2)
sec_seed = os.urandom(32)  # REAL random key, not a test string
pub_seed = hashlib.sha256(sec_seed + b'-workflow-pubseed').digest()[:N]

# Build the shared tree once (root is identical for every leaf under it)
kms = {}
authpaths = {}
ROOT = None
for leaf in (0, 1, 2):
    km, authpath, root = build_merkle_layer(pub_seed, 0, leaf, H, sec_seed=sec_seed)
    kms[leaf] = km
    authpaths[leaf] = authpath
    ROOT = root  # same for all leaves, single tree

print("Shared MASTER_ROOT:", ROOT.hex())
print("Tree: height", H, "-> 4 leaves, using leaf 0 (hop0->hop1), leaf 1 (hop1->hop2), leaf 2 (hop2->payout)")

# Per-hop application messages (the "state payload" written at that step)
messages = {
    0: hashlib.sha256(b'STEP0: workflow opened').digest()[:N],
    1: hashlib.sha256(b'STEP1: workflow approved').digest()[:N],
    2: hashlib.sha256(b'STEP2: workflow executed - paying out').digest()[:N],
}

# Build each hop's XMSS-verify script fragment (message -> digits -> 51 chains -> ltree -> merkle root)
hop_verify_scripts = {}
for leaf in (0, 1, 2):
    hop_verify_scripts[leaf] = build_layer_script(kms[leaf], authpaths[leaf])

# quick local sanity check of the raw verify fragment (root check only, no output-spk logic yet)
for leaf in (0, 1, 2):
    witnesses = sign_layer(kms[leaf], messages[leaf])
    script = hop_verify_scripts[leaf] + pd(ROOT) + bytes([OP_EQUAL])
    initial_stack = list(reversed(witnesses)) + [messages[leaf]]
    stack, alt = execute(script, initial_stack)
    ok = bool(stack) and stack[-1] == b'\x01'
    print(f"leaf {leaf} local verify: {'PASS' if ok else 'FAIL'}  (script {len(script)} bytes, {len(witnesses)} witnesses)")

out = {
    "sec_seed_hex": sec_seed.hex(),   # PRIVATE — never share, kept local only in this build artifact
    "pub_seed_hex": pub_seed.hex(),
    "master_root_hex": ROOT.hex(),
    "height": H,
    "messages_hex": {str(k): v.hex() for k, v in messages.items()},
    "witnesses_hex": {str(k): [w.hex() for w in sign_layer(kms[k], messages[k])] for k in (0,1,2)},
    "hop_verify_script_hex": {str(k): v.hex() for k, v in hop_verify_scripts.items()},
}
with open('/app/kaspa_wrpc/based_app_demo/workflow_build.json', 'w') as f:
    json.dump(out, f)
print("\nWritten workflow_build.json")

import sys, os, json, hashlib
sys.path.insert(0, '/app/kaspa_wrpc/sentinel_x402')
from full_2layer_test import build_merkle_layer, sign_layer, ab, prf, N

# ── MANDATORY KEY BACKUP: generate + save FIRST, before anything else runs ──
KEYS_DIR = '/app/kaspa_wrpc/keys'
os.makedirs(KEYS_DIR, exist_ok=True)

pub_seed = os.urandom(32)[:N]   # public seed, not secret but treat as key material
sec_seed = os.urandom(32)       # THE real secret

with open(f'{KEYS_DIR}/scorpion_stage2_xmss_seeds.json', 'w') as f:
    json.dump({
        'project': 'scorpion_brain_stage2',
        'role': 'xmss_pub_seed_and_sec_seed',
        'pub_seed_hex': pub_seed.hex(),
        'sec_seed_hex': sec_seed.hex(),
        'note': 'sec_seed is the real secret -- treat like a private key. Saved before any script ran.'
    }, f, indent=2)
print('KEY MATERIAL SAVED FIRST to', f'{KEYS_DIR}/scorpion_stage2_xmss_seeds.json')

# ── Build a small 4-leaf tree (height=2) -- prove the mechanism first ──
HEIGHT = 2
LEAF_IDX = 0
km_leaf, auth_path, root = build_merkle_layer(pub_seed, 0, LEAF_IDX, HEIGHT, sec_seed=sec_seed)

# ── Sign a real check-in message ──
# message = sha256(covenant_id || "checkin" || new_deadline)[:N] -- caller supplies exact bytes
message = hashlib.sha256(b'scorpion-stage2-checkin-msg-v1').digest()[:N]
witnesses = sign_layer(km_leaf, message)

out = {
    'pub_seed_hex': pub_seed.hex(),
    'root_hex': root.hex(),
    'leaf_idx': LEAF_IDX,
    'height': HEIGHT,
    'message_hex': message.hex(),
    'witnesses_hex': [w.hex() for w in witnesses],
    'auth_path': [{'auth_node_hex': lvl['auth_node'].hex(), 'is_left': lvl['is_left']} for lvl in auth_path],
}
with open('/tmp/stage2_witness.json', 'w') as f:
    json.dump(out, f, indent=2)

print('root:', root.hex())
print('leaf_idx:', LEAF_IDX)
print('message:', message.hex())
print('witnesses:', len(witnesses), 'chains,', len(witnesses[0]), 'bytes each')
print('auth_path levels:', len(auth_path))
print('Wrote /tmp/stage2_witness.json')

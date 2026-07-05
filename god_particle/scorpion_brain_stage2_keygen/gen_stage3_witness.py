import os, json, hashlib

KEYS_DIR = '/app/kaspa_wrpc/keys'
os.makedirs(KEYS_DIR, exist_ok=True)

# ── MANDATORY KEY BACKUP: generate the pool secrets FIRST, before building anything ──
N_LEAVES = 4
secrets = [os.urandom(32) for _ in range(N_LEAVES)]
nonces = [os.urandom(32) for _ in range(N_LEAVES)]

with open(f'{KEYS_DIR}/scorpion_stage3_privacy_secrets.json', 'w') as f:
    json.dump({
        'project': 'scorpion_brain_stage3_privacy_pool',
        'note': 'secrets/nonces are real key material for the 4 test deposits -- saved before use',
        'secrets_hex': [s.hex() for s in secrets],
        'nonces_hex': [n.hex() for n in nonces],
    }, f, indent=2)
print('KEY MATERIAL SAVED FIRST to scorpion_stage3_privacy_secrets.json')

def sha256(b): return hashlib.sha256(b).digest()

leaves = [sha256(secrets[i] + nonces[i]) for i in range(N_LEAVES)]

def build_tree(leaves):
    level = list(leaves)
    levels = [level]
    while len(level) > 1:
        nxt = []
        for i in range(0, len(level), 2):
            nxt.append(sha256(level[i] + level[i+1]))
        level = nxt
        levels.append(level)
    return levels

levels = build_tree(leaves)
root = levels[-1][0]

WITHDRAW_LEAF = 0
def auth_path(levels, idx):
    path = []
    for lvl in levels[:-1]:
        sib = idx ^ 1
        path.append({'sibling_hex': lvl[sib].hex(), 'is_left': (idx % 2 == 0)})
        idx //= 2
    return path

path = auth_path(levels, WITHDRAW_LEAF)
nullifier = sha256(b'NULL' + secrets[WITHDRAW_LEAF])

out = {
    'tree_depth': len(path),
    'merkle_root_hex': root.hex(),
    'leaf_idx': WITHDRAW_LEAF,
    'secret_hex': secrets[WITHDRAW_LEAF].hex(),
    'nonce_hex': nonces[WITHDRAW_LEAF].hex(),
    'auth_path': path,
    'nullifier_hex': nullifier.hex(),
}
with open('/tmp/stage3_witness.json', 'w') as f:
    json.dump(out, f, indent=2)

print('root:', root.hex())
print('leaf_idx:', WITHDRAW_LEAF)
print('nullifier:', nullifier.hex())
print('tree_depth:', len(path))
print('Wrote /tmp/stage3_witness.json')

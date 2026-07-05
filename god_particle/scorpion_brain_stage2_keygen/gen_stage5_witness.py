import os, json, hashlib

def sha256(b): return hashlib.sha256(b).digest()

# ── Build a small 4-leaf route Merkle tree — the scorpion's "wiring diagram" ──
# Each leaf = sha256(from_mode || to_mode). Allowed routes wired in for this test:
#   leaf 0: (0,1) stem -> vault
#   leaf 1: (1,0) vault -> stem
#   leaf 2: (0,4) stem -> game (already hardcoded elsewhere too, but now ALSO reachable via routing)
#   leaf 3: (4,0) game -> stem
routes = [(0, 1), (1, 0), (0, 4), (4, 0)]
leaves = [sha256(bytes([f, t])) for f, t in routes]

# level 1 (2 nodes)
n10 = sha256(leaves[0] + leaves[1])
n11 = sha256(leaves[2] + leaves[3])
# root
root = sha256(n10 + n11)

TARGET_LEAF = 0  # prove route (0,1): stem -> vault, via the nervous system this time
auth_path = [leaves[1], n11]   # sibling at level 0, then sibling at level 1
auth_dirs = [0, 0]              # 0 = our node is on the left, sibling on the right, at both levels

out = {
    'note': 'Stage 5 nervous-system route table -- 4 allowed routes wired in, proving route (0,1) stem->vault',
    'routes': routes,
    'route_root_hex': root.hex(),
    'target_leaf_index': TARGET_LEAF,
    'from_mode': routes[TARGET_LEAF][0],
    'to_mode': routes[TARGET_LEAF][1],
    'auth_path_hex': [h.hex() for h in auth_path],
    'auth_dirs': auth_dirs,
}
with open('/tmp/stage5_witness.json', 'w') as f:
    json.dump(out, f, indent=2)

print('route_root:', root.hex())
print('proving route index', TARGET_LEAF, '=', routes[TARGET_LEAF])
print('Wrote /tmp/stage5_witness.json')

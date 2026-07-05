import os, json, hashlib

KEYS_DIR = '/app/kaspa_wrpc/keys'
os.makedirs(KEYS_DIR, exist_ok=True)

# ── MANDATORY KEY BACKUP: generate move nonces FIRST, before building anything ──
nonce_a = os.urandom(32)
nonce_b = os.urandom(32)

with open(f'{KEYS_DIR}/scorpion_stage4_game_secrets.json', 'w') as f:
    json.dump({
        'project': 'scorpion_brain_stage4_courtship_dance',
        'note': 'move nonces (sealed commitment salts) for the Stag Hunt test game -- saved before use',
        'nonce_a_hex': nonce_a.hex(),
        'nonce_b_hex': nonce_b.hex(),
    }, f, indent=2)
print('KEY MATERIAL SAVED FIRST to scorpion_stage4_game_secrets.json')

def sha256(b): return hashlib.sha256(b).digest()

MOVE_A = 1  # STAG
MOVE_B = 1  # STAG  -> outcome SS: A=4500bps, B=4500bps, house=1000bps

commit_a = sha256(bytes([MOVE_A]) + nonce_a)
commit_b = sha256(bytes([MOVE_B]) + nonce_b)

POT_AMOUNT = 150_000_000  # 1.5 KAS in sompi, matches earlier Stag Hunt proof-of-mechanism scale

bps_table = {
    (1, 1): (4500, 4500, 1000),
    (1, 2): (1000, 8000, 1000),
    (2, 1): (8000, 1000, 1000),
    (2, 2): (4000, 4000, 2000),
}
bps_a, bps_b, bps_house = bps_table[(MOVE_A, MOVE_B)]
payout_a = POT_AMOUNT * bps_a // 10000
payout_b = POT_AMOUNT * bps_b // 10000
payout_house = POT_AMOUNT * bps_house // 10000

out = {
    'player_a_commit_hex': commit_a.hex(),
    'player_b_commit_hex': commit_b.hex(),
    'pot_amount': POT_AMOUNT,
    'move_a': MOVE_A,
    'move_b': MOVE_B,
    'nonce_a_hex': nonce_a.hex(),
    'nonce_b_hex': nonce_b.hex(),
    'payout_a': payout_a,
    'payout_b': payout_b,
    'payout_house': payout_house,
}
with open('/tmp/stage4_witness.json', 'w') as f:
    json.dump(out, f, indent=2)

print('commit_a:', commit_a.hex())
print('commit_b:', commit_b.hex())
print('outcome: SS (both Stag)')
print(f'payouts: A={payout_a} B={payout_b} house={payout_house} (pot={POT_AMOUNT})')
print('Wrote /tmp/stage4_witness.json')

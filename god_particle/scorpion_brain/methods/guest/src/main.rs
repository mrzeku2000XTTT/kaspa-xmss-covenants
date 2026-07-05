// THE SCORPION'S BRAIN — Stage 2 (The Stinger)
//
// Stage 1 proved identity + simplified ownership (stem <-> vault).
// Stage 2 adds the real immune system: XMSS/WOTS+ signature verification
// runs INSIDE the STARK circuit itself. Previously (entries 491-560) this
// same math required a ~130KB unrolled on-chain script. Now the brain does
// the thinking off-chain and the on-chain script only ever checks one
// constant-size proof, no matter how deep the hypertree grows.
//
// DNA blob layout (163 bytes, Stage 2 genome — Stage 1 fields + Chromosomes
// 2b/3 for the sentinel organ):
//
//   offset  size  field
//   ── Chromosome 1 — IDENTITY (unchanged from Stage 1) ──
//   0       4     magic = "GPDL"
//   4       32    covenant_id        (chitin — must never change)
//   36      4     generation         (u32 LE — must increment by exactly 1)
//   40      1     mode               (0=stem, 1=vault, 2=sentinel)
//   41      1     prev_mode          (must equal old.mode)
//   ── Chromosome 2 — OWNERSHIP ──
//   42      32    owner_commitment   (sha256(owner_secret))
//   74      32    xmss_root          (Merkle root of the antibody tree)
//   106     4     xmss_next_leaf     (next antibody index, no reuse)
//   ── Chromosome 3 — MORTALITY ──
//   110     8     deadline_daa       (u64 LE — CLTV death date)
//   118     4     checkin_interval   (u32 LE — heartbeat period)
//   122     1     is_terminal        (0=alive, 1=dying, 2=dead)
//   123     32    beneficiary_commit (heir verification hash)
//   155     4     grace_blocks       (u32 LE)
//   159     4     heartbeat_count    (u32 LE)
//   ── Chromosome 4 — PRIVACY (the carapace) ──
//   163     32    merkle_root        (privacy pool commitment tree root)
//   195     32    nullifier_hash     (most recently spent nullifier)
//   227     4     nullifier_count    (u32 LE — total withdrawals)
//   231     1     tree_depth         (Merkle tree height)
//
// Private witness (mode 2->2, check-in transition):
//   old_dna, new_dna (163B each)
//   pub_seed (24B), leaf_idx (u32), message (24B)
//   wots_witnesses: 51 x 24B (partial WOTS+ chain values)
//   auth_path: height x (auth_node 24B, is_left bool)
//
// Public journal: sha256(old_dna) || sha256(new_dna) || old_mode || new_mode

use risc0_zkvm::guest::env;
use risc0_zkvm::sha::{Impl, Sha256};

const N: usize = 24; // XMSS hash truncation length (matches proven mainnet scheme)
const W_STEPS: usize = 15; // max additional chain steps (w=16, so w-1=15)
const N_CHAINS: usize = 51; // 48 message digit chains + 3 checksum chains

const MAGIC: &[u8; 4] = b"GPDL";
const DNA_LEN: usize = 493;

const MODE_STEM: u8 = 0;
const MODE_VAULT: u8 = 1;
const MODE_SENTINEL: u8 = 2;
const MODE_PRIVACY: u8 = 3;
const MODE_GAME: u8 = 4;

struct Dna {
    covenant_id: [u8; 32],
    generation: u32,
    mode: u8,
    prev_mode: u8,
    owner_commitment: [u8; 32],
    xmss_root: [u8; 32],
    xmss_next_leaf: u32,
    deadline_daa: u64,
    checkin_interval: u32,
    is_terminal: u8,
    beneficiary_commit: [u8; 32],
    grace_blocks: u32,
    heartbeat_count: u32,
    merkle_root: [u8; 32],
    nullifier_hash: [u8; 32],
    nullifier_count: u32,
    tree_depth: u8,
    player_a_commit: [u8; 32],
    player_b_commit: [u8; 32],
    pot_amount: u64,
    game_state: u8,
    payout_a: u64,
    payout_b: u64,
    payout_house: u64,
    route_root: [u8; 32],
    self_template: [u8; 32],
    peer_count: u8,
    peer_id_0: [u8; 32],
    peer_id_1: [u8; 32],
    max_children: u8,
    child_template: [u8; 32],
    max_generations: u8,
    children_born: u8,
}

fn parse_dna(b: &[u8]) -> Dna {
    assert_eq!(b.len(), DNA_LEN, "DNA blob must be exactly 163 bytes");
    assert_eq!(&b[0..4], MAGIC, "bad DNA magic — not a scorpion genome");

    let mut covenant_id = [0u8; 32];
    covenant_id.copy_from_slice(&b[4..36]);
    let generation = u32::from_le_bytes(b[36..40].try_into().unwrap());
    let mode = b[40];
    let prev_mode = b[41];

    let mut owner_commitment = [0u8; 32];
    owner_commitment.copy_from_slice(&b[42..74]);
    let mut xmss_root = [0u8; 32];
    xmss_root.copy_from_slice(&b[74..106]);
    let xmss_next_leaf = u32::from_le_bytes(b[106..110].try_into().unwrap());

    let deadline_daa = u64::from_le_bytes(b[110..118].try_into().unwrap());
    let checkin_interval = u32::from_le_bytes(b[118..122].try_into().unwrap());
    let is_terminal = b[122];
    let mut beneficiary_commit = [0u8; 32];
    beneficiary_commit.copy_from_slice(&b[123..155]);
    let grace_blocks = u32::from_le_bytes(b[155..159].try_into().unwrap());
    let heartbeat_count = u32::from_le_bytes(b[159..163].try_into().unwrap());

    let mut merkle_root = [0u8; 32];
    merkle_root.copy_from_slice(&b[163..195]);
    let mut nullifier_hash = [0u8; 32];
    nullifier_hash.copy_from_slice(&b[195..227]);
    let nullifier_count = u32::from_le_bytes(b[227..231].try_into().unwrap());
    let tree_depth = b[231];

    let mut player_a_commit = [0u8; 32];
    player_a_commit.copy_from_slice(&b[232..264]);
    let mut player_b_commit = [0u8; 32];
    player_b_commit.copy_from_slice(&b[264..296]);
    let pot_amount = u64::from_le_bytes(b[296..304].try_into().unwrap());
    let game_state = b[304];
    let payout_a = u64::from_le_bytes(b[305..313].try_into().unwrap());
    let payout_b = u64::from_le_bytes(b[313..321].try_into().unwrap());
    let payout_house = u64::from_le_bytes(b[321..329].try_into().unwrap());

    let mut route_root = [0u8; 32];
    route_root.copy_from_slice(&b[329..361]);
    let mut self_template = [0u8; 32];
    self_template.copy_from_slice(&b[361..393]);
    let peer_count = b[393];
    let mut peer_id_0 = [0u8; 32];
    peer_id_0.copy_from_slice(&b[394..426]);
    let mut peer_id_1 = [0u8; 32];
    peer_id_1.copy_from_slice(&b[426..458]);

    let max_children = b[458];
    let mut child_template = [0u8; 32];
    child_template.copy_from_slice(&b[459..491]);
    let max_generations = b[491];
    let children_born = b[492];

    Dna {
        covenant_id,
        generation,
        mode,
        prev_mode,
        owner_commitment,
        xmss_root,
        xmss_next_leaf,
        deadline_daa,
        checkin_interval,
        is_terminal,
        beneficiary_commit,
        grace_blocks,
        heartbeat_count,
        merkle_root,
        nullifier_hash,
        nullifier_count,
        tree_depth,
        player_a_commit,
        player_b_commit,
        pot_amount,
        game_state,
        payout_a,
        payout_b,
        payout_house,
        route_root,
        self_template,
        peer_count,
        peer_id_0,
        peer_id_1,
        max_children,
        child_template,
        max_generations,
        children_born,
    }
}

fn mode_transition_allowed(from: u8, to: u8) -> bool {
    matches!(
        (from, to),
        (MODE_STEM, MODE_STEM)
            | (MODE_STEM, MODE_VAULT)
            | (MODE_VAULT, MODE_VAULT)
            | (MODE_VAULT, MODE_STEM)
            | (MODE_STEM, MODE_SENTINEL) // arm the stinger
            | (MODE_SENTINEL, MODE_SENTINEL) // check-in
            | (MODE_STEM, MODE_PRIVACY) // open the pool
            | (MODE_PRIVACY, MODE_PRIVACY) // shielded withdrawal
            | (MODE_STEM, MODE_GAME) // open the courtship dance
            | (MODE_GAME, MODE_GAME) // resolve the dance
    )
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(Impl::hash_bytes(data).as_bytes());
    out
}

fn ab(words: [u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&words[i].to_be_bytes());
    }
    out
}

// PRF matching the proven Python reference: sha256(0x00000003 || pub_seed || addr)[:out_len]
fn prf(pub_seed: &[u8], addr: &[u8; 32], out_len: usize) -> Vec<u8> {
    if out_len <= 32 {
        let mut buf = Vec::with_capacity(4 + pub_seed.len() + 32);
        buf.extend_from_slice(&[0, 0, 0, 3]);
        buf.extend_from_slice(pub_seed);
        buf.extend_from_slice(addr);
        sha256(&buf)[..out_len].to_vec()
    } else {
        let mut out = Vec::new();
        let mut counter: u8 = 0;
        while out.len() < out_len {
            let mut buf = Vec::with_capacity(4 + pub_seed.len() + 32 + 1);
            buf.extend_from_slice(&[0, 0, 0, 3]);
            buf.extend_from_slice(pub_seed);
            buf.extend_from_slice(addr);
            buf.push(counter);
            out.extend_from_slice(&sha256(&buf));
            counter += 1;
        }
        out.truncate(out_len);
        out
    }
}

// F(inp, KEY, BITMASK) = sha256(0x00000000 || KEY || (inp XOR BITMASK))[:N]
fn f_hash(inp: &[u8; N], key: &[u8], bitmask: &[u8]) -> [u8; N] {
    let mut xored = [0u8; N];
    for i in 0..N {
        xored[i] = inp[i] ^ bitmask[i];
    }
    let mut buf = Vec::with_capacity(4 + key.len() + N);
    buf.extend_from_slice(&[0, 0, 0, 0]);
    buf.extend_from_slice(key);
    buf.extend_from_slice(&xored);
    let h = sha256(&buf);
    let mut out = [0u8; N];
    out.copy_from_slice(&h[..N]);
    out
}

// Hl(a, b, KEY, BITMASK) = sha256(0x00000001 || KEY || ((a||b) XOR BITMASK))[:N]
fn hl_hash(a: &[u8; N], b: &[u8; N], key: &[u8], bitmask: &[u8]) -> [u8; N] {
    let mut concat = [0u8; 2 * N];
    concat[..N].copy_from_slice(a);
    concat[N..].copy_from_slice(b);
    let mut xored = [0u8; 2 * N];
    for i in 0..2 * N {
        xored[i] = concat[i] ^ bitmask[i];
    }
    let mut buf = Vec::with_capacity(4 + key.len() + 2 * N);
    buf.extend_from_slice(&[0, 0, 0, 1]);
    buf.extend_from_slice(key);
    buf.extend_from_slice(&xored);
    let h = sha256(&buf);
    let mut out = [0u8; N];
    out.copy_from_slice(&h[..N]);
    out
}

// Convert an N-byte message into 48 base-16 digits + 3 checksum digits = 51 total,
// matching the proven mainnet chain-index convention: chains 0..47 = message digits,
// chains 48..50 = checksum digits (get_digit's mapping, chains 0-2 = checksum in the
// original script builder; here we keep message-first ordering for guest simplicity —
// self-consistent between this guest and its own host/prover, which is all that matters
// since verification happens entirely inside this same STARK).
// Produces digit_for_chain[ci] for ci in 0..51, matching the proven mainnet
// convention EXACTLY: chains 0,1,2 = checksum digits; chains 3..50 = message
// digit (ci-3). (See xmss_lib.py's get_digit — chains 0-2 checksum, else message.)
fn message_to_digits(message: &[u8; N]) -> [u8; N_CHAINS] {
    let mut msg_digits = [0u8; 48];
    for i in 0..N {
        msg_digits[2 * i] = message[i] >> 4;
        msg_digits[2 * i + 1] = message[i] & 0x0F;
    }
    let csum: u32 = msg_digits.iter().map(|&d| 15 - d as u32).sum();
    let d0 = (csum / 256) as u8;
    let d1 = ((csum % 256) / 16) as u8;
    let d2 = (csum % 16) as u8;

    let mut digit_for_chain = [0u8; N_CHAINS];
    digit_for_chain[0] = d0;
    digit_for_chain[1] = d1;
    digit_for_chain[2] = d2;
    for ci in 3..N_CHAINS {
        digit_for_chain[ci] = msg_digits[ci - 3];
    }
    digit_for_chain
}

fn verify_wots_leaf(
    pub_seed: &[u8],
    leaf_idx: u32,
    message: &[u8; N],
    witnesses: &[[u8; N]; N_CHAINS],
) -> [u8; N] {
    let digits = message_to_digits(message);

    // ── Continue each chain from its witness value up to W_STEPS=15 ──
    let mut chain_ends = [[0u8; N]; N_CHAINS];
    for ci in 0..N_CHAINS {
        let d = digits[ci] as usize;
        let mut cur = witnesses[ci];
        for st in d..W_STEPS {
            let key = prf(
                pub_seed,
                &ab([0, 0, 0, 0, leaf_idx * 1000 + ci as u32, 0, st as u32, 0]),
                N,
            );
            let bitmask = prf(
                pub_seed,
                &ab([0, 0, 0, 0, leaf_idx * 1000 + ci as u32, 0, st as u32, 1]),
                N,
            );
            cur = f_hash(&cur, &key, &bitmask);
        }
        chain_ends[ci] = cur;
    }

    // ── L-tree: pairwise-combine all 51 chain-ends down to 1 leaf value ──
    let mut level: Vec<[u8; N]> = chain_ends.to_vec();
    let mut li: u32 = 0;
    while level.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0;
        let mut pair_idx: u32 = 0;
        while i + 1 < level.len() {
            let key = prf(pub_seed, &ab([0, 0, 0, 2, leaf_idx, li, pair_idx, 0]), N);
            let bitmask = prf(pub_seed, &ab([0, 0, 0, 2, leaf_idx, li, pair_idx, 1]), 2 * N);
            next.push(hl_hash(&level[i], &level[i + 1], &key, &bitmask));
            i += 2;
            pair_idx += 1;
        }
        if level.len() % 2 == 1 {
            next.push(level[level.len() - 1]);
        }
        level = next;
        li += 1;
    }
    level[0]
}

fn walk_merkle(
    pub_seed: &[u8],
    leaf_value: [u8; N],
    leaf_idx: u32,
    auth_path: &[([u8; N], bool)], // (auth_node, is_left_meaning_current_is_left)
) -> [u8; N] {
    let mut cur = leaf_value;
    let mut idx = leaf_idx;
    for (lvl, (auth_node, is_left)) in auth_path.iter().enumerate() {
        let pair_idx = idx / 2;
        let key = prf(pub_seed, &ab([0, 0, 0, 3, 0, lvl as u32, pair_idx, 0]), N);
        let bitmask = prf(pub_seed, &ab([0, 0, 0, 3, 0, lvl as u32, pair_idx, 1]), 2 * N);
        cur = if *is_left {
            hl_hash(&cur, auth_node, &key, &bitmask)
        } else {
            hl_hash(auth_node, &cur, &key, &bitmask)
        };
        idx /= 2;
    }
    cur
}

fn main() {
    let old_dna_bytes: Vec<u8> = env::read();
    let new_dna_bytes: Vec<u8> = env::read();
    let transition: u8 = env::read(); // 0=owner-sig, 1=XMSS check-in, 2=privacy withdrawal, 3=game resolve, 4=route moult, 5=reproduction (spawn children)

    let old_dna = parse_dna(&old_dna_bytes);
    let new_dna = parse_dna(&new_dna_bytes);

    // ── Phase 1: identity invariants (Chromosome 1 — always enforced) ──
    assert_eq!(old_dna.covenant_id, new_dna.covenant_id, "covenant_id mutated");
    assert_eq!(new_dna.generation, old_dna.generation + 1, "generation must advance by 1");
    assert_eq!(new_dna.prev_mode, old_dna.mode, "prev_mode must equal old.mode");
    // Transition 4 (nervous system) authorizes its own (from_mode, to_mode) pair via a
    // Merkle-inclusion proof against route_root instead of this hardcoded table -- that's
    // the whole point of Chromosome 10: new routes can be wired in by the owner (a new
    // route_root) without ever redeploying this guest. Every other transition kind still
    // goes through the hardcoded allow-list below.
    if transition != 4 {
        assert!(mode_transition_allowed(old_dna.mode, new_dna.mode), "illegal mode transition");
    }

    if transition == 0 {
        // ── Stage 1 style: owner-secret authorized transition (stem<->vault, or arming sentinel) ──
        let owner_secret: Vec<u8> = env::read();
        assert_eq!(owner_secret.len(), 32, "owner_secret must be 32 bytes");
        let h = sha256(&owner_secret);
        assert_eq!(h, old_dna.owner_commitment, "owner_secret does not match commitment");
        assert_eq!(old_dna.owner_commitment, new_dna.owner_commitment, "owner_commitment must persist");

        if new_dna.mode == MODE_SENTINEL && old_dna.mode == MODE_STEM {
            // Arming the stinger: owner sets deadline, checkin_interval, xmss_root, beneficiary.
            assert_eq!(new_dna.heartbeat_count, 0, "freshly armed sentinel starts at 0 heartbeats");
            assert!(new_dna.checkin_interval > 0, "checkin_interval must be nonzero");
            assert_eq!(new_dna.xmss_next_leaf, 0, "freshly armed sentinel starts at leaf 0");
        }

        if new_dna.mode == MODE_PRIVACY && old_dna.mode == MODE_STEM {
            // Opening the carapace: owner authorizes the pool with a pre-built commitment
            // tree root (built and audited off-chain, same trust model as arming Sentinel).
            assert_eq!(new_dna.nullifier_count, 0, "freshly opened pool starts with 0 withdrawals");
            assert!(new_dna.tree_depth > 0, "tree_depth must be nonzero");
        }

        if new_dna.mode == MODE_GAME && old_dna.mode == MODE_STEM {
            // Opening the courtship dance: owner/house sets both players' sealed move
            // commitments and the pot, unresolved (game_state=0), zero payouts.
            assert_eq!(new_dna.game_state, 0, "freshly opened game starts unresolved");
            assert_eq!(new_dna.payout_a, 0, "freshly opened game has no payouts yet");
            assert_eq!(new_dna.payout_b, 0, "freshly opened game has no payouts yet");
            assert_eq!(new_dna.payout_house, 0, "freshly opened game has no payouts yet");
            assert!(new_dna.pot_amount > 0, "pot_amount must be nonzero");
        }
    } else if transition == 1 {
        // ── Stage 2 real logic: XMSS-authorized check-in (sentinel -> sentinel) ──
        assert_eq!(old_dna.mode, MODE_SENTINEL, "check-in only valid from sentinel mode");
        assert_eq!(new_dna.mode, MODE_SENTINEL, "check-in stays in sentinel mode");
        assert_eq!(old_dna.is_terminal, 0, "cannot check in once dying/dead");

        // Immune system invariants carry forward.
        assert_eq!(old_dna.xmss_root, new_dna.xmss_root, "xmss_root must not change on check-in");
        assert_eq!(
            old_dna.beneficiary_commit, new_dna.beneficiary_commit,
            "beneficiary_commit must not change on check-in"
        );
        assert_eq!(
            old_dna.checkin_interval, new_dna.checkin_interval,
            "checkin_interval must not change on check-in"
        );

        // Read the XMSS witness (private): pub_seed, leaf index, message, WOTS+ witnesses, auth path.
        let pub_seed: Vec<u8> = env::read();
        let leaf_idx: u32 = env::read();
        let message: Vec<u8> = env::read();
        let witnesses_flat: Vec<u8> = env::read(); // N_CHAINS * N bytes
        let auth_nodes_flat: Vec<u8> = env::read(); // height * N bytes
        let auth_is_left: Vec<u8> = env::read(); // height bytes, 1=left 0=right

        assert_eq!(message.len(), N, "message must be N bytes");
        assert_eq!(witnesses_flat.len(), N_CHAINS * N, "witnesses must be N_CHAINS*N bytes");
        assert_eq!(
            auth_nodes_flat.len() % N,
            0,
            "auth_nodes must be a multiple of N bytes"
        );
        let height = auth_nodes_flat.len() / N;
        assert_eq!(auth_is_left.len(), height, "auth_is_left length must match height");

        // No-antibody-reuse: this leaf must be exactly the next unused one, and the
        // successor DNA must advance the counter by exactly 1.
        assert_eq!(leaf_idx, old_dna.xmss_next_leaf, "leaf index must equal xmss_next_leaf (no reuse)");
        assert_eq!(
            new_dna.xmss_next_leaf,
            old_dna.xmss_next_leaf + 1,
            "xmss_next_leaf must advance by exactly 1"
        );

        let mut message_arr = [0u8; N];
        message_arr.copy_from_slice(&message);

        let mut witnesses = [[0u8; N]; N_CHAINS];
        for i in 0..N_CHAINS {
            witnesses[i].copy_from_slice(&witnesses_flat[i * N..(i + 1) * N]);
        }

        let mut auth_path: Vec<([u8; N], bool)> = Vec::with_capacity(height);
        for i in 0..height {
            let mut node = [0u8; N];
            node.copy_from_slice(&auth_nodes_flat[i * N..(i + 1) * N]);
            auth_path.push((node, auth_is_left[i] == 1));
        }

        // ── THE REAL VERIFICATION — inside the STARK, not an on-chain script ──
        let leaf_value = verify_wots_leaf(&pub_seed, leaf_idx, &message_arr, &witnesses);
        let computed_root = walk_merkle(&pub_seed, leaf_value, leaf_idx, &auth_path);

        let mut xmss_root_padded = [0u8; N];
        xmss_root_padded.copy_from_slice(&old_dna.xmss_root[..N]);
        assert_eq!(computed_root, xmss_root_padded, "XMSS signature does not verify against xmss_root");

        // ── Mortality invariants: deadline extends by exactly checkin_interval ──
        assert_eq!(
            new_dna.deadline_daa,
            old_dna.deadline_daa + old_dna.checkin_interval as u64,
            "deadline_daa must advance by exactly checkin_interval"
        );
        assert_eq!(
            new_dna.heartbeat_count,
            old_dna.heartbeat_count + 1,
            "heartbeat_count must advance by exactly 1"
        );
        assert_eq!(new_dna.is_terminal, 0, "check-in keeps the organism alive (is_terminal=0)");
    } else if transition == 2 {
        // ── Stage 3 real logic: shielded withdrawal (privacy -> privacy) ──
        // Proves knowledge of (secret, nonce) such that sha256(secret||nonce) is a leaf
        // in the commitment tree rooted at merkle_root, and reveals nullifier =
        // sha256("NULL"||secret) without revealing which leaf was spent. All inside the
        // STARK — no separate Groth16 proof needed, the STARK IS the proof.
        assert_eq!(old_dna.mode, MODE_PRIVACY, "withdrawal only valid from privacy mode");
        assert_eq!(new_dna.mode, MODE_PRIVACY, "withdrawal stays in privacy mode");

        assert_eq!(old_dna.merkle_root, new_dna.merkle_root, "merkle_root must not change on withdrawal");
        assert_eq!(old_dna.tree_depth, new_dna.tree_depth, "tree_depth must not change on withdrawal");
        assert_eq!(
            new_dna.nullifier_count,
            old_dna.nullifier_count + 1,
            "nullifier_count must advance by exactly 1"
        );
        assert_ne!(
            new_dna.nullifier_hash, old_dna.nullifier_hash,
            "nullifier_hash must change on every withdrawal (no replay)"
        );

        let secret: Vec<u8> = env::read();
        let nonce: Vec<u8> = env::read();
        let leaf_idx: u32 = env::read();
        let siblings_flat: Vec<u8> = env::read(); // tree_depth * 32 bytes
        let sib_is_left: Vec<u8> = env::read(); // tree_depth bytes, 1=current node is left child

        assert_eq!(secret.len(), 32, "secret must be 32 bytes");
        assert_eq!(nonce.len(), 32, "nonce must be 32 bytes");
        let depth = old_dna.tree_depth as usize;
        assert_eq!(siblings_flat.len(), depth * 32, "siblings length must match tree_depth");
        assert_eq!(sib_is_left.len(), depth, "sib_is_left length must match tree_depth");

        // leaf = sha256(secret || nonce) — the deposit commitment
        let mut leaf_input = Vec::with_capacity(64);
        leaf_input.extend_from_slice(&secret);
        leaf_input.extend_from_slice(&nonce);
        let mut cur = sha256(&leaf_input);

        let mut idx = leaf_idx;
        for i in 0..depth {
            let mut sib = [0u8; 32];
            sib.copy_from_slice(&siblings_flat[i * 32..(i + 1) * 32]);
            let is_left = sib_is_left[i] == 1;
            let mut buf = Vec::with_capacity(64);
            if is_left {
                buf.extend_from_slice(&cur);
                buf.extend_from_slice(&sib);
            } else {
                buf.extend_from_slice(&sib);
                buf.extend_from_slice(&cur);
            }
            cur = sha256(&buf);
            idx /= 2;
        }

        assert_eq!(cur, old_dna.merkle_root, "Merkle membership proof failed against merkle_root");

        // nullifier = sha256("NULL" || secret) — reveals a spend marker, not the deposit itself
        let mut nul_input = Vec::with_capacity(4 + 32);
        nul_input.extend_from_slice(b"NULL");
        nul_input.extend_from_slice(&secret);
        let nullifier = sha256(&nul_input);
        assert_eq!(nullifier, new_dna.nullifier_hash, "revealed nullifier does not match new DNA");
    } else if transition == 3 {
        // ── Stage 4 real logic: courtship dance resolution (game -> game) ──
        // Stag Hunt: both players commit sealed moves off-chain; this transition proves
        // both reveals match their commitments and computes the payout split from a
        // hardcoded choreography table — entirely inside the STARK, no on-chain branching
        // script needed (unlike the original Stag Hunt covenant, entry from earlier work,
        // which needed OpTxOutputAmount/OpTxOutputSpk branches directly in the redeem script).
        assert_eq!(old_dna.mode, MODE_GAME, "resolve only valid from game mode");
        assert_eq!(new_dna.mode, MODE_GAME, "resolve stays in game mode");
        assert_eq!(old_dna.game_state, 0, "game must be unresolved to resolve");
        assert_eq!(new_dna.game_state, 1, "resolve must set game_state=1");
        assert_eq!(old_dna.player_a_commit, new_dna.player_a_commit, "player_a_commit immutable");
        assert_eq!(old_dna.player_b_commit, new_dna.player_b_commit, "player_b_commit immutable");
        assert_eq!(old_dna.pot_amount, new_dna.pot_amount, "pot_amount immutable during resolve");

        let move_a: u8 = env::read();
        let nonce_a: Vec<u8> = env::read();
        let move_b: u8 = env::read();
        let nonce_b: Vec<u8> = env::read();

        assert!(move_a == 1 || move_a == 2, "move_a must be 1=STAG or 2=HARE");
        assert!(move_b == 1 || move_b == 2, "move_b must be 1=STAG or 2=HARE");
        assert_eq!(nonce_a.len(), 32, "nonce_a must be 32 bytes");
        assert_eq!(nonce_b.len(), 32, "nonce_b must be 32 bytes");

        let mut buf_a = Vec::with_capacity(33);
        buf_a.push(move_a);
        buf_a.extend_from_slice(&nonce_a);
        assert_eq!(sha256(&buf_a), old_dna.player_a_commit, "move_a does not match player_a_commit");

        let mut buf_b = Vec::with_capacity(33);
        buf_b.push(move_b);
        buf_b.extend_from_slice(&nonce_b);
        assert_eq!(sha256(&buf_b), old_dna.player_b_commit, "move_b does not match player_b_commit");

        // Choreography (payout table), immutable, in basis points of pot_amount:
        //   SS (both Stag):  A=4500 B=4500 house=1000
        //   SH (A Stag/B Hare): A=1000 B=8000 house=1000
        //   HS (A Hare/B Stag): A=8000 B=1000 house=1000
        //   HH (both Hare):  A=4000 B=4000 house=2000
        let (bps_a, bps_b, bps_house): (u64, u64, u64) = match (move_a, move_b) {
            (1, 1) => (4500, 4500, 1000),
            (1, 2) => (1000, 8000, 1000),
            (2, 1) => (8000, 1000, 1000),
            (2, 2) => (4000, 4000, 2000),
            _ => unreachable!(),
        };

        let pot = old_dna.pot_amount;
        let payout_a = pot * bps_a / 10000;
        let payout_b = pot * bps_b / 10000;
        let payout_house = pot * bps_house / 10000;

        assert_eq!(new_dna.payout_a, payout_a, "payout_a mismatch vs choreography");
        assert_eq!(new_dna.payout_b, payout_b, "payout_b mismatch vs choreography");
        assert_eq!(new_dna.payout_house, payout_house, "payout_house mismatch vs choreography");
    } else if transition == 4 {
        // ── Stage 5 real logic: nervous system (Chromosome 10) ──
        // Instead of a hardcoded match arm, this transition authorizes ANY (from_mode,
        // to_mode) pair that the owner has wired into route_root — a Merkle tree of
        // allowed routes. Rewiring the brain (adding a new allowed moult) only requires
        // an owner-signed route_root update, never a new guest deployment. This is the
        // scorpion's "ventral nerve cord": the wiring diagram of which thoughts are
        // possible, expressed as data instead of code.
        assert_eq!(old_dna.route_root, new_dna.route_root, "route_root immutable during a route-validated moult");
        assert_eq!(old_dna.self_template, new_dna.self_template, "self_template is set at birth, immutable");
        assert_eq!(old_dna.peer_count, new_dna.peer_count, "peer_count unchanged by a route moult");
        assert_eq!(old_dna.peer_id_0, new_dna.peer_id_0, "peer_id_0 unchanged by a route moult");
        assert_eq!(old_dna.peer_id_1, new_dna.peer_id_1, "peer_id_1 unchanged by a route moult");

        let route_leaf_index: u32 = env::read();
        let auth_path: Vec<[u8; 32]> = env::read(); // sibling hashes, leaf -> root
        let auth_dirs: Vec<u8> = env::read(); // 0 = sibling on right, 1 = sibling on left

        assert_eq!(auth_path.len(), auth_dirs.len(), "auth_path/auth_dirs length mismatch");

        let mut leaf_buf = Vec::with_capacity(2);
        leaf_buf.push(old_dna.mode);
        leaf_buf.push(new_dna.mode);
        let mut node = sha256(&leaf_buf);

        let mut idx = route_leaf_index;
        for (sibling, dir) in auth_path.iter().zip(auth_dirs.iter()) {
            let mut buf = Vec::with_capacity(64);
            if *dir == 0 {
                buf.extend_from_slice(&node);
                buf.extend_from_slice(sibling);
            } else {
                buf.extend_from_slice(sibling);
                buf.extend_from_slice(&node);
            }
            node = sha256(&buf);
            idx /= 2;
        }

        assert_eq!(node, old_dna.route_root, "route not found in route_root -- this moult is not wired into the nervous system");
    } else if transition == 5 {
        // ── Stage 6 real logic: reproduction (Chromosome 6, gonads) ──
        // Parthenogenesis: the owner alone authorizes a spawn. The parent covenant
        // continues living (children_born increments) while N fresh child covenants
        // are born in the SAME proof -- true fanout, not a separate transaction per
        // child. Each child gets its own chitin (covenant_id), deterministically
        // derived from the parent's lineage, so parentage is always verifiable.
        let owner_secret: Vec<u8> = env::read();
        assert_eq!(owner_secret.len(), 32, "owner_secret must be 32 bytes");
        let h = sha256(&owner_secret);
        assert_eq!(h, old_dna.owner_commitment, "owner_secret does not match commitment");
        assert_eq!(old_dna.owner_commitment, new_dna.owner_commitment, "owner_commitment must persist");

        assert_eq!(old_dna.max_children, new_dna.max_children, "max_children unchanged by spawning");
        assert_eq!(old_dna.child_template, new_dna.child_template, "child_template is set at birth, immutable");
        assert_eq!(old_dna.max_generations, new_dna.max_generations, "parent's own reproductive depth is unchanged by spawning its own children");

        assert!(old_dna.max_generations > 0, "sterile: max_generations exhausted, this organism cannot reproduce");

        let children: Vec<Vec<u8>> = env::read();
        let num_children = children.len() as u8;
        assert!(num_children > 0, "a reproduction transition must spawn at least one child");
        assert_eq!(
            new_dna.children_born,
            old_dna.children_born + num_children,
            "children_born must increase by exactly the number of children spawned"
        );
        assert!(
            new_dna.children_born as u16 <= old_dna.max_children as u16,
            "cannot exceed max_children (litter size limit)"
        );

        for (i, child_bytes) in children.iter().enumerate() {
            let child = parse_dna(child_bytes);
            let idx = old_dna.children_born + i as u8;
            let mut buf = Vec::with_capacity(33);
            buf.extend_from_slice(&old_dna.covenant_id);
            buf.push(idx);
            let expected_child_id = sha256(&buf);
            assert_eq!(child.covenant_id, expected_child_id, "child covenant_id must be deterministically derived from the parent's lineage");
            assert_eq!(child.generation, 0, "a newborn child starts at generation 0");
            assert_eq!(child.mode, MODE_STEM, "a newborn child is born resting, in STEM mode");
            assert_eq!(
                child.max_generations,
                old_dna.max_generations - 1,
                "child's reproductive depth must be exactly the parent's minus 1 (telomere shortening prevents runaway reproduction)"
            );
            assert_eq!(child.owner_commitment, old_dna.owner_commitment, "child inherits the same owner (asexual/parthenogenesis)");
            assert_eq!(child.children_born, 0, "a newborn child has not yet reproduced");
        }
    } else {
        panic!("unknown transition kind");
    }

    // ── Commit the transition to the public journal ──
    let old_dna_hash = sha256(&old_dna_bytes);
    let new_dna_hash = sha256(&new_dna_bytes);

    let mut journal = Vec::with_capacity(32 + 32 + 1 + 1);
    journal.extend_from_slice(&old_dna_hash);
    journal.extend_from_slice(&new_dna_hash);
    journal.push(old_dna.mode);
    journal.push(new_dna.mode);

    env::commit_slice(&journal);
}

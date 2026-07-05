
fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
}

const GENESIS_DAA: u64 = 477000000;
fn genesis_txid() -> [u8; 32] { sha256_bytes(b"scorpion_genesis_placeholder_txid") }

fn append_memory(mut d: Vec<u8>, prev_state_hash: &[u8], transition_count: u32, creation_daa: u64, creation_txid: &[u8]) -> Vec<u8> {
    assert_eq!(prev_state_hash.len(), 32);
    assert_eq!(creation_txid.len(), 32);
    d.extend_from_slice(prev_state_hash);
    d.extend_from_slice(&transition_count.to_le_bytes());
    d.extend_from_slice(&creation_daa.to_le_bytes());
    d.extend_from_slice(creation_txid);
    assert_eq!(d.len(), 569, "DNA must be 569 bytes after Chromosome 7 (memory) is appended");
    d
}

// Host program for the Scorpion's Brain — Stage 3 privacy carapace proof.
// Proves a shielded withdrawal: knowledge of (secret, nonce) whose commitment
// is a leaf in the pool's Merkle tree, revealing only a nullifier — all
// verified natively inside the STARK (sha256 Merkle tree, no external ZK
// circuit needed; the STARK itself is the proof).

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_privacy_dna(
    covenant_id: &[u8],
    generation: u32,
    mode: u8,
    prev_mode: u8,
    owner_commitment: &[u8],
    merkle_root: &[u8],
    nullifier_hash: &[u8],
    nullifier_count: u32,
    tree_depth: u8,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(232);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(mode);
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    // Chromosome 2 tail — unused in privacy mode.
    d.extend_from_slice(&[0u8; 32]); // xmss_root
    d.extend_from_slice(&0u32.to_le_bytes()); // xmss_next_leaf
    // Chromosome 3 — unused in privacy mode.
    d.extend_from_slice(&0u64.to_le_bytes()); // deadline_daa
    d.extend_from_slice(&0u32.to_le_bytes()); // checkin_interval
    d.push(0); // is_terminal
    d.extend_from_slice(&[0u8; 32]); // beneficiary_commit
    d.extend_from_slice(&0u32.to_le_bytes()); // grace_blocks
    d.extend_from_slice(&0u32.to_le_bytes()); // heartbeat_count
    // Chromosome 4 — the real payload for this stage.
    assert_eq!(merkle_root.len(), 32);
    d.extend_from_slice(merkle_root);
    assert_eq!(nullifier_hash.len(), 32);
    d.extend_from_slice(nullifier_hash);
    d.extend_from_slice(&nullifier_count.to_le_bytes());
    d.push(tree_depth);
    // Chromosome 9 (game theory) — zeroed, unused outside game mode.
    d.extend_from_slice(&[0u8; 32]); // player_a_commit
    d.extend_from_slice(&[0u8; 32]); // player_b_commit
    d.extend_from_slice(&0u64.to_le_bytes()); // pot_amount
    d.push(0); // game_state
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_a
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_b
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_house
    
    // Chromosome 10 (nervous system) — zeroed, unused outside route-validated moults.
    d.extend_from_slice(&[0u8; 32]); // route_root
    d.extend_from_slice(&[0u8; 32]); // self_template
    d.push(0); // peer_count
    d.extend_from_slice(&[0u8; 32]); // peer_id_0
    d.extend_from_slice(&[0u8; 32]); // peer_id_1
    
    // Chromosome 6 (reproduction) — zeroed/sterile by default, unused outside spawn transitions.
    d.push(0); // max_children
    d.extend_from_slice(&[0u8; 32]); // child_template
    d.push(0); // max_generations
    d.push(0); // children_born
    assert_eq!(d.len(), 493);
    d
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std_env::args().collect();
    let owner_json_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "/app/kaspa_wrpc/keys/scorpion_owner_secret.json".to_string());
    let cov_json_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "/app/kaspa_wrpc/keys/scorpion_covenant_id.json".to_string());
    let witness_json_path = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "/tmp/stage3_witness.json".to_string());
    let out_path = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| "/tmp/scorpion_stage3_receipt.json".to_string());

    let owner_json: Value = serde_json::from_str(&fs::read_to_string(&owner_json_path).unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(&fs::read_to_string(&cov_json_path).unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    let w: Value = serde_json::from_str(&fs::read_to_string(&witness_json_path).unwrap()).unwrap();
    let root = hex_to_bytes(w["merkle_root_hex"].as_str().unwrap());
    let leaf_idx = w["leaf_idx"].as_u64().unwrap() as u32;
    let secret = hex_to_bytes(w["secret_hex"].as_str().unwrap());
    let nonce = hex_to_bytes(w["nonce_hex"].as_str().unwrap());
    let nullifier = hex_to_bytes(w["nullifier_hex"].as_str().unwrap());
    let tree_depth = w["tree_depth"].as_u64().unwrap() as u8;

    let auth_path_arr = w["auth_path"].as_array().unwrap();
    let mut siblings_flat = Vec::new();
    let mut sib_is_left = Vec::new();
    for lvl in auth_path_arr {
        siblings_flat.extend_from_slice(&hex_to_bytes(lvl["sibling_hex"].as_str().unwrap()));
        sib_is_left.push(if lvl["is_left"].as_bool().unwrap() { 1u8 } else { 0u8 });
    }

    // old_dna: pool open, 0 withdrawals yet (gen 1, freshly opened from stem at gen 0)
    let zero_nullifier = [0u8; 32];
    let old_dna = build_privacy_dna(
        &covenant_id,
        1,
        3, // MODE_PRIVACY
        0, // prev_mode = stem
        &owner_commitment,
        &root,
        &zero_nullifier,
        0,
        tree_depth,
    );

    // new_dna: first withdrawal proven, nullifier revealed
    let new_dna_body = build_privacy_dna(
        &covenant_id,
        2,
        3,
        3, // prev_mode = privacy
        &owner_commitment,
        &root,
        &nullifier,
        1,
        tree_depth,
    );

    let genesis_txid_v = genesis_txid();
    let old_dna = append_memory(old_dna, &[0u8; 32], 0, GENESIS_DAA, &genesis_txid_v);
    let old_hash = sha256_bytes(&old_dna);
    let new_dna = append_memory(new_dna_body, &old_hash, 1, GENESIS_DAA, &genesis_txid_v);

    eprintln!("old_dna (hex) = {}", hex::encode(&old_dna));
    eprintln!("new_dna (hex) = {}", hex::encode(&new_dna));
    eprintln!("merkle_root = {}", hex::encode(&root));
    eprintln!("nullifier = {}", hex::encode(&nullifier));
    eprintln!("leaf_idx = {leaf_idx}");

    let env = ExecutorEnv::builder()
        .write(&old_dna)
        .unwrap()
        .write(&new_dna)
        .unwrap()
        .write(&2u8) // transition = 2 -> privacy withdrawal
        .unwrap()
        .write(&secret)
        .unwrap()
        .write(&nonce)
        .unwrap()
        .write(&leaf_idx)
        .unwrap()
        .write(&siblings_flat)
        .unwrap()
        .write(&sib_is_left)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();

    eprintln!("Proving the shielded withdrawal (Merkle membership + nullifier, all inside the STARK)...");
    let prove_info = prover
        .prove_with_opts(env, SCORPION_BRAIN_GUEST_ELF, &ProverOpts::succinct())
        .unwrap();

    let receipt = prove_info.receipt;
    receipt.verify(SCORPION_BRAIN_GUEST_ID).expect("local verification failed");

    let journal_bytes = receipt.journal.bytes.clone();
    let succinct = receipt.inner.succinct().expect("not a succinct receipt");

    let image_id_bytes: Vec<u8> = SCORPION_BRAIN_GUEST_ID
        .iter()
        .flat_map(|w: &u32| w.to_le_bytes())
        .collect();
    let seal_bytes: Vec<u8> = succinct.seal.iter().flat_map(|w| w.to_le_bytes()).collect();

    eprintln!("PROOF VERIFIED LOCALLY — Merkle membership + nullifier checked by the STARK itself.");
    eprintln!("journal (hex) = {}", hex::encode(&journal_bytes));

    let out = serde_json::json!({
        "old_dna_hex": hex::encode(&old_dna),
        "new_dna_hex": hex::encode(&new_dna),
        "image_id_hex": hex::encode(image_id_bytes.as_slice()),
        "journal_bytes_hex": hex::encode(&journal_bytes),
        "seal_hex": hex::encode(&seal_bytes),
        "control_id_hex": hex::encode(succinct.control_id.as_bytes()),
        "hashfn": format!("{:?}", succinct.hashfn),
    });

    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote receipt data to {out_path}");
}

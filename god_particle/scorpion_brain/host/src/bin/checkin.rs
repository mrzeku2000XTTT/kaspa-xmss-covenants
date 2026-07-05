// Host program for the Scorpion's Brain — Stage 2 sentinel check-in proof.
// Reads all key/witness material from pre-saved JSON files. Builds old_dna
// (armed sentinel) and new_dna (checked-in sentinel), then proves the
// transition with the real XMSS/WOTS+ verification running inside the STARK.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const N: usize = 24;

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn build_sentinel_dna(
    covenant_id: &[u8],
    generation: u32,
    mode: u8,
    prev_mode: u8,
    owner_commitment: &[u8],
    xmss_root: &[u8],
    xmss_next_leaf: u32,
    deadline_daa: u64,
    checkin_interval: u32,
    is_terminal: u8,
    beneficiary_commit: &[u8],
    grace_blocks: u32,
    heartbeat_count: u32,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(163);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(mode);
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    assert_eq!(xmss_root.len(), N);
    d.extend_from_slice(xmss_root);
    d.extend_from_slice(&[0u8; 8]); // pad N=24 -> 32-byte slot
    d.extend_from_slice(&xmss_next_leaf.to_le_bytes());
    d.extend_from_slice(&deadline_daa.to_le_bytes());
    d.extend_from_slice(&checkin_interval.to_le_bytes());
    d.push(is_terminal);
    d.extend_from_slice(beneficiary_commit);
    d.extend_from_slice(&grace_blocks.to_le_bytes());
    d.extend_from_slice(&heartbeat_count.to_le_bytes());
    // Chromosome 4 (privacy) — zeroed, unused in sentinel mode.
    d.extend_from_slice(&[0u8; 32]); // merkle_root
    d.extend_from_slice(&[0u8; 32]); // nullifier_hash
    d.extend_from_slice(&0u32.to_le_bytes()); // nullifier_count
    d.push(0); // tree_depth
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
    assert_eq!(d.len(), 458);
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
        .unwrap_or_else(|| "/tmp/stage2_witness.json".to_string());
    let out_path = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| "/tmp/scorpion_stage2_receipt.json".to_string());

    let owner_json: Value = serde_json::from_str(&fs::read_to_string(&owner_json_path).unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(&fs::read_to_string(&cov_json_path).unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    let w: Value = serde_json::from_str(&fs::read_to_string(&witness_json_path).unwrap()).unwrap();
    let pub_seed = hex_to_bytes(w["pub_seed_hex"].as_str().unwrap());
    let root = hex_to_bytes(w["root_hex"].as_str().unwrap());
    let leaf_idx = w["leaf_idx"].as_u64().unwrap() as u32;
    let message = hex_to_bytes(w["message_hex"].as_str().unwrap());
    let witnesses_hex: Vec<String> = w["witnesses_hex"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let mut witnesses_flat = Vec::new();
    for wh in &witnesses_hex {
        witnesses_flat.extend_from_slice(&hex_to_bytes(wh));
    }
    let auth_path_arr = w["auth_path"].as_array().unwrap();
    let mut auth_nodes_flat = Vec::new();
    let mut auth_is_left = Vec::new();
    for lvl in auth_path_arr {
        auth_nodes_flat.extend_from_slice(&hex_to_bytes(lvl["auth_node_hex"].as_str().unwrap()));
        auth_is_left.push(if lvl["is_left"].as_bool().unwrap() { 1u8 } else { 0u8 });
    }

    let beneficiary_secret = b"scorpion-stage2-beneficiary-placeholder"; // demo beneficiary
    let mut hasher = Sha256::new();
    hasher.update(beneficiary_secret);
    let beneficiary_commit = hasher.finalize().to_vec();

    let deadline_daa_start: u64 = 500_000_000;
    let checkin_interval: u32 = 86400;

    // old_dna: armed sentinel, gen 1 (Stage 1 already moved stem(0)->vault(1); here we
    // model a fresh arm stem(0)->sentinel(2) at gen 1 for a clean standalone Stage 2 proof)
    let old_dna = build_sentinel_dna(
        &covenant_id,
        1,
        2, // MODE_SENTINEL
        0, // prev_mode = stem
        &owner_commitment,
        &root,
        leaf_idx, // xmss_next_leaf = 0 (about to use leaf 0)
        deadline_daa_start,
        checkin_interval,
        0, // is_terminal = alive
        &beneficiary_commit,
        1000, // grace_blocks
        0,    // heartbeat_count
    );

    let new_dna = build_sentinel_dna(
        &covenant_id,
        2,
        2, // still sentinel
        2, // prev_mode = sentinel
        &owner_commitment,
        &root,
        leaf_idx + 1, // xmss_next_leaf advances
        deadline_daa_start + checkin_interval as u64,
        checkin_interval,
        0,
        &beneficiary_commit,
        1000,
        1, // heartbeat_count advances
    );

    eprintln!("old_dna (hex) = {}", hex::encode(&old_dna));
    eprintln!("new_dna (hex) = {}", hex::encode(&new_dna));
    eprintln!("XMSS root = {}", hex::encode(&root));
    eprintln!("leaf_idx = {leaf_idx}");

    let env = ExecutorEnv::builder()
        .write(&old_dna)
        .unwrap()
        .write(&new_dna)
        .unwrap()
        .write(&1u8) // transition = 1 -> sentinel check-in (XMSS path)
        .unwrap()
        .write(&pub_seed)
        .unwrap()
        .write(&leaf_idx)
        .unwrap()
        .write(&message)
        .unwrap()
        .write(&witnesses_flat)
        .unwrap()
        .write(&auth_nodes_flat)
        .unwrap()
        .write(&auth_is_left)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();

    eprintln!("Proving the check-in (XMSS verified INSIDE the STARK)... this may take a while.");
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

    eprintln!("PROOF VERIFIED LOCALLY — the STARK itself checked the XMSS signature.");
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


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

// Host program for the Scorpion's Brain — Stage 1 (Hatching), updated to the
// Stage 2 163-byte DNA format (the guest ELF now handles both transition
// kinds: 0 = owner-secret authorized, 1 = XMSS-authorized check-in).
//
// This binary proves the original stem -> vault moult (transition=0).
// See src/bin/checkin.rs for the Stage 2 sentinel check-in proof.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts};
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

fn build_dna(
    covenant_id: &[u8],
    generation: u32,
    mode: u8,
    prev_mode: u8,
    owner_commitment: &[u8],
) -> Vec<u8> {
    let mut d = Vec::with_capacity(163);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(mode);
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    // Chromosome 2 tail (xmss_root 32B slot + xmss_next_leaf 4B) — zeroed, unused in stem/vault modes.
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&0u32.to_le_bytes());
    // Chromosome 3 (mortality) — zeroed, unused in stem/vault modes.
    d.extend_from_slice(&0u64.to_le_bytes()); // deadline_daa
    d.extend_from_slice(&0u32.to_le_bytes()); // checkin_interval
    d.push(0); // is_terminal
    d.extend_from_slice(&[0u8; 32]); // beneficiary_commit
    d.extend_from_slice(&0u32.to_le_bytes()); // grace_blocks
    d.extend_from_slice(&0u32.to_le_bytes()); // heartbeat_count
    // Chromosome 4 (privacy) — zeroed, unused in stem/vault modes.
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
    let owner_secret_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "/app/kaspa_wrpc/keys/scorpion_owner_secret.json".to_string());
    let covenant_id_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "/app/kaspa_wrpc/keys/scorpion_covenant_id.json".to_string());
    let out_path = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "/tmp/scorpion_stage1_receipt.json".to_string());

    let owner_json: Value = serde_json::from_str(&fs::read_to_string(&owner_secret_path).unwrap()).unwrap();
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(&fs::read_to_string(&covenant_id_path).unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    let old_dna_body = build_dna(&covenant_id, 0, 0, 0, &owner_commitment);
    let genesis_txid_v = genesis_txid();
    let old_dna = append_memory(old_dna_body.clone(), &[0u8; 32], 0, GENESIS_DAA, &genesis_txid_v);
    let old_hash = sha256_bytes(&old_dna);
    let new_dna_body = build_dna(&covenant_id, 1, 1, 0, &owner_commitment);
    let new_dna = append_memory(new_dna_body, &old_hash, 1, GENESIS_DAA, &genesis_txid_v);

    eprintln!("old_dna (hex) = {}", hex::encode(&old_dna));
    eprintln!("new_dna (hex) = {}", hex::encode(&new_dna));

    let env = ExecutorEnv::builder()
        .write(&old_dna)
        .unwrap()
        .write(&new_dna)
        .unwrap()
        .write(&0u8) // transition = 0 -> owner-secret authorized
        .unwrap()
        .write(&owner_secret)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();

    eprintln!("Proving the first moult (stem -> vault)... this may take a while.");
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

    eprintln!("PROOF VERIFIED LOCALLY.");
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

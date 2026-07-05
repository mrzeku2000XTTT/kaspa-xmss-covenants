// Host program for the Scorpion's Brain — Stage 5 nervous-system proof.
// Proves a mode transition is authorized not by a hardcoded match arm, but by
// Merkle-inclusion in route_root — the owner-configurable "wiring diagram" of
// allowed moults (Chromosome 10). Rewiring the brain to allow a new moult only
// needs a new route_root; the guest ELF never has to change.

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
fn build_dna(
    covenant_id: &[u8],
    generation: u32,
    mode: u8,
    prev_mode: u8,
    owner_commitment: &[u8],
    route_root: &[u8],
) -> Vec<u8> {
    let mut d = Vec::with_capacity(458);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(mode);
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    // Chromosome 2 tail — unused here.
    d.extend_from_slice(&[0u8; 32]); // xmss_root
    d.extend_from_slice(&0u32.to_le_bytes()); // xmss_next_leaf
    // Chromosome 3 — unused here.
    d.extend_from_slice(&0u64.to_le_bytes()); // deadline_daa
    d.extend_from_slice(&0u32.to_le_bytes()); // checkin_interval
    d.push(0); // is_terminal
    d.extend_from_slice(&[0u8; 32]); // beneficiary_commit
    d.extend_from_slice(&0u32.to_le_bytes()); // grace_blocks
    d.extend_from_slice(&0u32.to_le_bytes()); // heartbeat_count
    // Chromosome 4 — unused here.
    d.extend_from_slice(&[0u8; 32]); // merkle_root
    d.extend_from_slice(&[0u8; 32]); // nullifier_hash
    d.extend_from_slice(&0u32.to_le_bytes()); // nullifier_count
    d.push(0); // tree_depth
    // Chromosome 9 — unused here.
    d.extend_from_slice(&[0u8; 32]); // player_a_commit
    d.extend_from_slice(&[0u8; 32]); // player_b_commit
    d.extend_from_slice(&0u64.to_le_bytes()); // pot_amount
    d.push(0); // game_state
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_a
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_b
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_house
    // Chromosome 10 — the real payload for this stage.
    assert_eq!(route_root.len(), 32);
    d.extend_from_slice(route_root);
    d.extend_from_slice(&[0u8; 32]); // self_template (unused in this test)
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
        .unwrap_or_else(|| "/tmp/stage5_witness.json".to_string());
    let out_path = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| "/tmp/scorpion_stage5_receipt.json".to_string());

    let owner_json: Value = serde_json::from_str(&fs::read_to_string(&owner_json_path).unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(&fs::read_to_string(&cov_json_path).unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    let w: Value = serde_json::from_str(&fs::read_to_string(&witness_json_path).unwrap()).unwrap();
    let route_root = hex_to_bytes(w["route_root_hex"].as_str().unwrap());
    let from_mode = w["from_mode"].as_u64().unwrap() as u8;
    let to_mode = w["to_mode"].as_u64().unwrap() as u8;
    let leaf_index = w["target_leaf_index"].as_u64().unwrap() as u32;
    let auth_path: Vec<Vec<u8>> = w["auth_path_hex"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| hex_to_bytes(v.as_str().unwrap()))
        .collect();
    let auth_path: Vec<[u8; 32]> = auth_path
        .into_iter()
        .map(|v| {
            let mut a = [0u8; 32];
            a.copy_from_slice(&v);
            a
        })
        .collect();
    let auth_dirs: Vec<u8> = w["auth_dirs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();

    // old_dna: generation 1, mode = from_mode (stem), route_root wired in already.
    let old_dna = build_dna(&covenant_id, 1, from_mode, 0, &owner_commitment, &route_root);
    // new_dna: generation 2, mode = to_mode (vault), reached via the nervous system, not a hardcoded arm.
    let new_dna = build_dna(&covenant_id, 2, to_mode, from_mode, &owner_commitment, &route_root);

    eprintln!("route_root (hex) = {}", hex::encode(&route_root));
    eprintln!("proving route: mode {from_mode} -> mode {to_mode} (leaf {leaf_index})");

    let env = ExecutorEnv::builder()
        .write(&old_dna)
        .unwrap()
        .write(&new_dna)
        .unwrap()
        .write(&4u8) // transition = 4 -> nervous-system route-validated moult
        .unwrap()
        .write(&leaf_index)
        .unwrap()
        .write(&auth_path)
        .unwrap()
        .write(&auth_dirs)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();

    eprintln!("Proving the route-validated moult (Merkle inclusion in route_root, checked inside the STARK)...");
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

    eprintln!("PROOF VERIFIED LOCALLY — the moult was authorized by the nervous system's route table, not hardcoded logic.");
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

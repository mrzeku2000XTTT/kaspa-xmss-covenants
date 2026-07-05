// Host program for the Scorpion's Brain — Stage 6 reproduction proof.
// Parthenogenesis: the parent covenant spawns N fresh child covenants in ONE
// proof. Each child gets its own chitin (covenant_id) deterministically derived
// from the parent's lineage, is born at generation 0 in STEM (resting) mode,
// inherits the parent's owner, and has its reproductive depth (max_generations)
// reduced by one — telomere shortening that eventually makes a lineage sterile.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_STEM: u8 = 0;

#[allow(clippy::too_many_arguments)]
fn build_dna(
    covenant_id: &[u8],
    generation: u32,
    mode: u8,
    prev_mode: u8,
    owner_commitment: &[u8],
    max_children: u8,
    child_template: &[u8],
    max_generations: u8,
    children_born: u8,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(493);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(mode);
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    d.extend_from_slice(&[0u8; 32]); // xmss_root
    d.extend_from_slice(&0u32.to_le_bytes()); // xmss_next_leaf
    d.extend_from_slice(&0u64.to_le_bytes()); // deadline_daa
    d.extend_from_slice(&0u32.to_le_bytes()); // checkin_interval
    d.push(0); // is_terminal
    d.extend_from_slice(&[0u8; 32]); // beneficiary_commit
    d.extend_from_slice(&0u32.to_le_bytes()); // grace_blocks
    d.extend_from_slice(&0u32.to_le_bytes()); // heartbeat_count
    d.extend_from_slice(&[0u8; 32]); // merkle_root
    d.extend_from_slice(&[0u8; 32]); // nullifier_hash
    d.extend_from_slice(&0u32.to_le_bytes()); // nullifier_count
    d.push(0); // tree_depth
    d.extend_from_slice(&[0u8; 32]); // player_a_commit
    d.extend_from_slice(&[0u8; 32]); // player_b_commit
    d.extend_from_slice(&0u64.to_le_bytes()); // pot_amount
    d.push(0); // game_state
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_a
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_b
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_house
    d.extend_from_slice(&[0u8; 32]); // route_root
    d.extend_from_slice(&[0u8; 32]); // self_template
    d.push(0); // peer_count
    d.extend_from_slice(&[0u8; 32]); // peer_id_0
    d.extend_from_slice(&[0u8; 32]); // peer_id_1
    // Chromosome 6 — the real payload for this stage.
    d.push(max_children);
    assert_eq!(child_template.len(), 32);
    d.extend_from_slice(child_template);
    d.push(max_generations);
    d.push(children_born);
    assert_eq!(d.len(), 493);
    d
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
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
    let out_path = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "/tmp/scorpion_stage6_receipt.json".to_string());

    let owner_json: Value = serde_json::from_str(&fs::read_to_string(&owner_json_path).unwrap()).unwrap();
    let owner_commitment_hex = owner_json["owner_commitment_hex"].as_str().unwrap();
    let owner_commitment = hex::decode(owner_commitment_hex).unwrap();

    let owner_secret_hex = owner_json["secret_hex"].as_str().unwrap();
    let owner_secret = hex::decode(owner_secret_hex).unwrap();

    let cov_json: Value = serde_json::from_str(&fs::read_to_string(&cov_json_path).unwrap()).unwrap();
    let covenant_id = hex::decode(cov_json["covenant_id_hex"].as_str().unwrap()).unwrap();

    let child_template = sha256(b"scorpion_brain_v1_template"); // stand-in "same script family" hash

    // Parent: generation 5, STEM mode, fertile (max_generations=3), litter cap 2, 0 children so far.
    let old_dna = build_dna(
        &covenant_id,
        5,
        MODE_STEM,
        MODE_STEM,
        &owner_commitment,
        2,                // max_children
        &child_template,
        3,                // max_generations
        0,                // children_born
    );
    // Parent after spawning 2 children: generation advances, children_born -> 2.
    let new_dna = build_dna(
        &covenant_id,
        6,
        MODE_STEM,
        MODE_STEM,
        &owner_commitment,
        2,
        &child_template,
        3,
        2,
    );

    // Two children, deterministically derived covenant_id = sha256(parent_covenant_id || index).
    let mut child_ids = vec![];
    for idx in 0u8..2u8 {
        let mut buf = Vec::with_capacity(33);
        buf.extend_from_slice(&covenant_id);
        buf.push(idx);
        child_ids.push(sha256(&buf));
    }

    let children_dna: Vec<Vec<u8>> = child_ids
        .iter()
        .map(|cid| {
            build_dna(
                cid,
                0,          // generation 0 -- newborn
                MODE_STEM,  // born resting
                MODE_STEM,  // prev_mode placeholder for a newborn
                &owner_commitment,
                0,          // sterile until owner configures it
                &child_template,
                2,          // max_generations - 1 (parent had 3)
                0,          // children_born
            )
        })
        .collect();

    eprintln!("parent covenant_id (hex) = {}", hex::encode(&covenant_id));
    eprintln!("spawning {} children (parthenogenesis)...", children_dna.len());
    for (i, cid) in child_ids.iter().enumerate() {
        eprintln!("  child {i} covenant_id = {}", hex::encode(cid));
    }

    let env = ExecutorEnv::builder()
        .write(&old_dna)
        .unwrap()
        .write(&new_dna)
        .unwrap()
        .write(&5u8) // transition = 5 -> reproduction (spawn children)
        .unwrap()
        .write(&owner_secret)
        .unwrap()
        .write(&children_dna)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();

    eprintln!("Proving the reproduction transition (parthenogenesis fanout, checked inside the STARK)...");
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

    eprintln!("PROOF VERIFIED LOCALLY — parent spawned 2 children, both deterministically derived, both sterile-capped at max_generations-1.");
    eprintln!("journal (hex) = {}", hex::encode(&journal_bytes));

    let out = serde_json::json!({
        "old_dna_hex": hex::encode(&old_dna),
        "new_dna_hex": hex::encode(&new_dna),
        "children_dna_hex": children_dna.iter().map(hex::encode).collect::<Vec<_>>(),
        "image_id_hex": hex::encode(image_id_bytes.as_slice()),
        "journal_bytes_hex": hex::encode(&journal_bytes),
        "seal_hex": hex::encode(&seal_bytes),
        "control_id_hex": hex::encode(succinct.control_id.as_bytes()),
        "hashfn": format!("{:?}", succinct.hashfn),
    });

    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote receipt data to {out_path}");
}

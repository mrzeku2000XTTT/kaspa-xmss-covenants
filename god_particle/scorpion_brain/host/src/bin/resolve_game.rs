// Host program for the Scorpion's Brain — Stage 4 claws/game-theory proof.
// Proves the courtship dance (Stag Hunt) resolution: both players' sealed moves
// match their commitments, and the payout split follows a hardcoded choreography
// table — all verified natively inside the STARK, no on-chain branching script.

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
fn build_game_dna(
    covenant_id: &[u8],
    generation: u32,
    prev_mode: u8,
    owner_commitment: &[u8],
    player_a_commit: &[u8],
    player_b_commit: &[u8],
    pot_amount: u64,
    game_state: u8,
    payout_a: u64,
    payout_b: u64,
    payout_house: u64,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(329);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(4); // MODE_GAME
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    // Chromosome 2 tail — unused in game mode.
    d.extend_from_slice(&[0u8; 32]); // xmss_root
    d.extend_from_slice(&0u32.to_le_bytes()); // xmss_next_leaf
    // Chromosome 3 — unused in game mode.
    d.extend_from_slice(&0u64.to_le_bytes()); // deadline_daa
    d.extend_from_slice(&0u32.to_le_bytes()); // checkin_interval
    d.push(0); // is_terminal
    d.extend_from_slice(&[0u8; 32]); // beneficiary_commit
    d.extend_from_slice(&0u32.to_le_bytes()); // grace_blocks
    d.extend_from_slice(&0u32.to_le_bytes()); // heartbeat_count
    // Chromosome 4 — unused in game mode.
    d.extend_from_slice(&[0u8; 32]); // merkle_root
    d.extend_from_slice(&[0u8; 32]); // nullifier_hash
    d.extend_from_slice(&0u32.to_le_bytes()); // nullifier_count
    d.push(0); // tree_depth
    // Chromosome 9 — the real payload for this stage.
    assert_eq!(player_a_commit.len(), 32);
    d.extend_from_slice(player_a_commit);
    assert_eq!(player_b_commit.len(), 32);
    d.extend_from_slice(player_b_commit);
    d.extend_from_slice(&pot_amount.to_le_bytes());
    d.push(game_state);
    d.extend_from_slice(&payout_a.to_le_bytes());
    d.extend_from_slice(&payout_b.to_le_bytes());
    d.extend_from_slice(&payout_house.to_le_bytes());
    assert_eq!(d.len(), 329);
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
        .unwrap_or_else(|| "/tmp/stage4_witness.json".to_string());
    let out_path = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| "/tmp/scorpion_stage4_receipt.json".to_string());

    let owner_json: Value = serde_json::from_str(&fs::read_to_string(&owner_json_path).unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(&fs::read_to_string(&cov_json_path).unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    let w: Value = serde_json::from_str(&fs::read_to_string(&witness_json_path).unwrap()).unwrap();
    let commit_a = hex_to_bytes(w["player_a_commit_hex"].as_str().unwrap());
    let commit_b = hex_to_bytes(w["player_b_commit_hex"].as_str().unwrap());
    let pot_amount = w["pot_amount"].as_u64().unwrap();
    let move_a = w["move_a"].as_u64().unwrap() as u8;
    let move_b = w["move_b"].as_u64().unwrap() as u8;
    let nonce_a = hex_to_bytes(w["nonce_a_hex"].as_str().unwrap());
    let nonce_b = hex_to_bytes(w["nonce_b_hex"].as_str().unwrap());
    let payout_a = w["payout_a"].as_u64().unwrap();
    let payout_b = w["payout_b"].as_u64().unwrap();
    let payout_house = w["payout_house"].as_u64().unwrap();

    // old_dna: game opened (gen 1, from stem gen 0), unresolved, zero payouts.
    let old_dna = build_game_dna(
        &covenant_id,
        1,
        0, // prev_mode = stem
        &owner_commitment,
        &commit_a,
        &commit_b,
        pot_amount,
        0,
        0,
        0,
        0,
    );

    // new_dna: dance resolved, payouts computed per choreography.
    let new_dna = build_game_dna(
        &covenant_id,
        2,
        4, // prev_mode = game
        &owner_commitment,
        &commit_a,
        &commit_b,
        pot_amount,
        1,
        payout_a,
        payout_b,
        payout_house,
    );

    eprintln!("old_dna (hex) = {}", hex::encode(&old_dna));
    eprintln!("new_dna (hex) = {}", hex::encode(&new_dna));
    eprintln!("move_a={move_a} move_b={move_b} pot={pot_amount}");
    eprintln!("payouts: A={payout_a} B={payout_b} house={payout_house}");

    let env = ExecutorEnv::builder()
        .write(&old_dna)
        .unwrap()
        .write(&new_dna)
        .unwrap()
        .write(&3u8) // transition = 3 -> game resolve
        .unwrap()
        .write(&move_a)
        .unwrap()
        .write(&nonce_a)
        .unwrap()
        .write(&move_b)
        .unwrap()
        .write(&nonce_b)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();

    eprintln!("Proving the courtship dance resolution (Stag Hunt, choreography checked inside the STARK)...");
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

    eprintln!("PROOF VERIFIED LOCALLY — choreography (payout split) checked by the STARK itself.");
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

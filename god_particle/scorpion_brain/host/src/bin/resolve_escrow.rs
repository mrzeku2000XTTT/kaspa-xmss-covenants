
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

#[allow(clippy::too_many_arguments)]
fn append_escrow(
    mut d: Vec<u8>,
    arbiter_commit: &[u8],
    buyer_commit: &[u8],
    seller_commit: &[u8],
    dispute_deadline: u64,
    escrow_amount: u64,
    winner: u8,
    dispute_state: u8,
) -> Vec<u8> {
    assert_eq!(arbiter_commit.len(), 32);
    assert_eq!(buyer_commit.len(), 32);
    assert_eq!(seller_commit.len(), 32);
    d.extend_from_slice(arbiter_commit);
    d.extend_from_slice(buyer_commit);
    d.extend_from_slice(seller_commit);
    d.extend_from_slice(&dispute_deadline.to_le_bytes());
    d.extend_from_slice(&escrow_amount.to_le_bytes());
    d.push(winner);
    d.push(dispute_state);
    assert_eq!(d.len(), 683, "DNA must be 683 bytes after Chromosome 8 (escrow) is appended");
    d
}

// Host program for the Scorpion's Brain — Stage 8 "The Pedipalps" (Chromosome 8, ESCROW).
//
// Two parties grip disputed value between them and cannot resolve it alone.
// This proves the resolution, either of two ways:
//   (a) a trusted third party (the arbiter) shows up, proves who they are by
//       revealing the secret behind arbiter_commit, and hands down a decision, or
//   (b) nobody ever comes to help -- the dispute_deadline lapses and the buyer
//       is protected by a safe, permissionless default (zero signature needed).
//
// Run with `--mode=arbiter` (default) or `--mode=timeout`.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_ESCROW: u8 = 5;

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_escrow_body(covenant_id: &[u8], generation: u32, prev_mode: u8, owner_commitment: &[u8]) -> Vec<u8> {
    let mut d = Vec::with_capacity(493);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(MODE_ESCROW);
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    // Chromosome 2 tail — unused in escrow mode.
    d.extend_from_slice(&[0u8; 32]); // xmss_root
    d.extend_from_slice(&0u32.to_le_bytes()); // xmss_next_leaf
    // Chromosome 3 — unused in escrow mode.
    d.extend_from_slice(&0u64.to_le_bytes()); // deadline_daa
    d.extend_from_slice(&0u32.to_le_bytes()); // checkin_interval
    d.push(0); // is_terminal
    d.extend_from_slice(&[0u8; 32]); // beneficiary_commit
    d.extend_from_slice(&0u32.to_le_bytes()); // grace_blocks
    d.extend_from_slice(&0u32.to_le_bytes()); // heartbeat_count
    // Chromosome 4 — unused in escrow mode.
    d.extend_from_slice(&[0u8; 32]); // merkle_root
    d.extend_from_slice(&[0u8; 32]); // nullifier_hash
    d.extend_from_slice(&0u32.to_le_bytes()); // nullifier_count
    d.push(0); // tree_depth
    // Chromosome 9 (game theory) — unused in escrow mode.
    d.extend_from_slice(&[0u8; 32]); // player_a_commit
    d.extend_from_slice(&[0u8; 32]); // player_b_commit
    d.extend_from_slice(&0u64.to_le_bytes()); // pot_amount
    d.push(0); // game_state
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_a
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_b
    d.extend_from_slice(&0u64.to_le_bytes()); // payout_house
    // Chromosome 10 (nervous system) — unused in escrow mode.
    d.extend_from_slice(&[0u8; 32]); // route_root
    d.extend_from_slice(&[0u8; 32]); // self_template
    d.push(0); // peer_count
    d.extend_from_slice(&[0u8; 32]); // peer_id_0
    d.extend_from_slice(&[0u8; 32]); // peer_id_1
    // Chromosome 6 (reproduction) — zeroed/sterile, unused in escrow mode.
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
    let mut mode = "arbiter".to_string();
    let mut out_path = "/tmp/scorpion_stage8_receipt.json".to_string();
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--mode=") {
            mode = v.to_string();
        } else if let Some(v) = a.strip_prefix("--out=") {
            out_path = v.to_string();
        }
    }

    let owner_json: Value = serde_json::from_str(
        &fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap(),
    )
    .unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(
        &fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap(),
    )
    .unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    let escrow_json: Value = serde_json::from_str(
        &fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_stage8_escrow_secrets.json").unwrap(),
    )
    .unwrap();
    let arbiter_secret = hex_to_bytes(escrow_json["arbiter_secret_hex"].as_str().unwrap());
    let arbiter_commit = hex_to_bytes(escrow_json["arbiter_commit_hex"].as_str().unwrap());
    let buyer_commit = hex_to_bytes(escrow_json["buyer_commit_hex"].as_str().unwrap());
    let seller_commit = hex_to_bytes(escrow_json["seller_commit_hex"].as_str().unwrap());

    let dispute_deadline: u64 = 477_500_000;
    let escrow_amount: u64 = 150_000_000; // 1.5 KAS, proof-of-mechanism scale

    // old_dna: dispute raised (gen 1, from stem gen 0), nobody has decided yet.
    let genesis_txid_v = genesis_txid();
    let old_body = build_escrow_body(&covenant_id, 1, 0, &owner_commitment);
    let old_dna_memory = append_memory(old_body, &[0u8; 32], 0, GENESIS_DAA, &genesis_txid_v);
    let old_dna = append_escrow(
        old_dna_memory,
        &arbiter_commit,
        &buyer_commit,
        &seller_commit,
        dispute_deadline,
        escrow_amount,
        0,
        1, // dispute_state = raised
    );
    let old_hash = sha256_bytes(&old_dna);

    let (current_daa, winner, label): (u64, u8, &str) = if mode == "timeout" {
        (dispute_deadline + 100, 0, "TIMEOUT -- nobody came to help, buyer refunded by default")
    } else {
        (dispute_deadline - 100, 1, "ARBITER RESOLVED -- help arrived, seller awarded the dispute")
    };

    let new_body = build_escrow_body(&covenant_id, 2, MODE_ESCROW, &owner_commitment);
    let new_dna_memory = append_memory(new_body, &old_hash, 1, GENESIS_DAA, &genesis_txid_v);
    let new_dna = append_escrow(
        new_dna_memory,
        &arbiter_commit,
        &buyer_commit,
        &seller_commit,
        dispute_deadline,
        escrow_amount,
        winner,
        2, // dispute_state = resolved
    );

    eprintln!("Stage 8 (ESCROW / arbiter) -- {label}");
    eprintln!("old_dna (hex) = {}", hex::encode(&old_dna));
    eprintln!("new_dna (hex) = {}", hex::encode(&new_dna));
    eprintln!("current_daa={current_daa} dispute_deadline={dispute_deadline} winner={winner}");

    let mut env_builder = ExecutorEnv::builder();
    env_builder
        .write(&old_dna)
        .unwrap()
        .write(&new_dna)
        .unwrap()
        .write(&6u8) // transition = 6 -> escrow resolve
        .unwrap()
        .write(&current_daa)
        .unwrap();

    if mode != "timeout" {
        env_builder.write(&arbiter_secret).unwrap();
    }

    let env = env_builder.build().unwrap();

    let prover = default_prover();
    eprintln!("Proving the escrow resolution ({mode})... this may take a while.");
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

    eprintln!("PROOF VERIFIED LOCALLY -- {label}");
    eprintln!("journal (hex) = {}", hex::encode(&journal_bytes));

    let out = serde_json::json!({
        "mode": mode,
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

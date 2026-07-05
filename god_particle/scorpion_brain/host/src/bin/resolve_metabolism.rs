
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

fn append_escrow_zeroed(mut d: Vec<u8>) -> Vec<u8> {
    // Escrow chromosome unused in this mode -- zeroed placeholder.
    d.extend_from_slice(&[0u8; 32]); // arbiter_commit
    d.extend_from_slice(&[0u8; 32]); // buyer_commit
    d.extend_from_slice(&[0u8; 32]); // seller_commit
    d.extend_from_slice(&0u64.to_le_bytes()); // dispute_deadline
    d.extend_from_slice(&0u64.to_le_bytes()); // escrow_amount
    d.push(0); // winner
    d.push(0); // dispute_state
    assert_eq!(d.len(), 683, "DNA must be 683 bytes after Chromosome 8 (escrow) is appended");
    d
}

#[allow(clippy::too_many_arguments)]
fn append_metabolism(
    mut d: Vec<u8>,
    compute_budget: u16,
    fee_reserve: u64,
    min_output: u64,
    rate_window: u32,
    rate_max_spend: u32,
    window_start_daa: u64,
    window_spend_count: u32,
) -> Vec<u8> {
    d.extend_from_slice(&compute_budget.to_le_bytes());
    d.extend_from_slice(&fee_reserve.to_le_bytes());
    d.extend_from_slice(&min_output.to_le_bytes());
    d.extend_from_slice(&rate_window.to_le_bytes());
    d.extend_from_slice(&rate_max_spend.to_le_bytes());
    d.extend_from_slice(&window_start_daa.to_le_bytes());
    d.extend_from_slice(&window_spend_count.to_le_bytes());
    assert_eq!(d.len(), 721, "DNA must be 721 bytes after Chromosome 5 (metabolism) is appended");
    d
}

// Host program for the Scorpion's Brain — Stage 9 "The Hepatic Pancreas" (Chromosome 5,
// METABOLISM). This is the direct on-chain answer to our own 6.3 KAS of real losses from
// unbounded, unmonitored spending across this whole project: fee_reserve must strictly
// decrease every heartbeat and can never hit zero (no full drains), compute_budget stays
// in the safe RISC0 mass range, and a rolling rate-limit window (cancer detection) locks
// the organism down if it's spending faster than its own policy allows.
//
// Proves an ordinary owner-authorized transition (vault -> vault, a re-key/no-op move)
// while the metabolism chromosome does its work underneath. `--mode=heartbeat` (default)
// is a healthy, well-fed transition. `--mode=starve` deliberately drains fee_reserve to
// zero (must be rejected). `--mode=cancer` deliberately exceeds rate_max_spend inside the
// rate window (must be rejected).

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_VAULT: u8 = 1;

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn build_vault_body(covenant_id: &[u8], generation: u32, prev_mode: u8, owner_commitment: &[u8]) -> Vec<u8> {
    let mut d = Vec::with_capacity(493);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(MODE_VAULT);
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
    let mut mode = "heartbeat".to_string();
    let mut out_path = "/tmp/scorpion_stage9_receipt.json".to_string();
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
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(
        &fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap(),
    )
    .unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    // Metabolic policy: proven RISC0 compute_budget=2700 (entry 602), KIP-9 storage floor
    // ~20M sompi/output (entry 522), rate policy: at most 5 spends per 1000-DAA window.
    let compute_budget: u16 = 2700;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;

    let genesis_txid_v = genesis_txid();

    // old_dna: mid-window, 2 spends already counted, healthy fee_reserve.
    let old_fee_reserve: u64 = 500_000_000; // 5 KAS glycogen
    let old_window_start_daa: u64 = 477_000_000;
    let old_window_spend_count: u32 = 2;

    // For "cancer" mode, old_dna must already be sitting AT the rate cap
    // (window_spend_count == rate_max_spend) -- that's what the guest's own
    // rate-limit check fires on. Build the real old_dna up front (not rebuilt later)
    // so the memory-chain hash-link stays consistent throughout.
    let old_window_spend_count_effective = if mode == "cancer" { rate_max_spend } else { old_window_spend_count };

    let old_body = build_vault_body(&covenant_id, 1, MODE_VAULT, &owner_commitment);
    let old_dna_memory = append_memory(old_body, &[0u8; 32], 0, GENESIS_DAA, &genesis_txid_v);
    let old_dna_escrow = append_escrow_zeroed(old_dna_memory);
    let old_dna = append_metabolism(
        old_dna_escrow, compute_budget, old_fee_reserve, min_output,
        rate_window, rate_max_spend, old_window_start_daa, old_window_spend_count_effective,
    );
    let old_hash = sha256_bytes(&old_dna);

    let current_daa: u64 = old_window_start_daa + 100; // still inside the window

    let (new_fee_reserve, new_window_start_daa, new_window_spend_count, label): (u64, u64, u32, &str) =
        match mode.as_str() {
            "starve" => (0, old_window_start_daa, old_window_spend_count + 1, "STARVE -- fee_reserve drained to zero (must be rejected)"),
            "cancer" => {
                // Simulate having already hit the cap: pretend old_window_spend_count == rate_max_spend.
                (old_fee_reserve - 10_000_000, old_window_start_daa, rate_max_spend + 1, "CANCER -- rate limit exceeded inside window (must be rejected)")
            }
            _ => (old_fee_reserve - 10_000_000, old_window_start_daa, old_window_spend_count + 1, "HEARTBEAT -- healthy metabolic transition"),
        };

    let new_body = build_vault_body(&covenant_id, 2, MODE_VAULT, &owner_commitment);
    let new_dna_memory = append_memory(new_body, &old_hash, 1, GENESIS_DAA, &genesis_txid_v);
    let new_dna_escrow = append_escrow_zeroed(new_dna_memory);
    let new_dna = append_metabolism(
        new_dna_escrow, compute_budget, new_fee_reserve, min_output,
        rate_window, rate_max_spend, new_window_start_daa, new_window_spend_count,
    );

    eprintln!("Stage 9 (METABOLISM) -- {label}");
    eprintln!("old_fee_reserve={old_fee_reserve} new_fee_reserve={new_fee_reserve}");
    eprintln!("old_window_spend_count={old_window_spend_count_effective} new_window_spend_count={new_window_spend_count} rate_max_spend={rate_max_spend}");

    let env = ExecutorEnv::builder()
        .write(&old_dna)
        .unwrap()
        .write(&new_dna)
        .unwrap()
        .write(&0u8) // transition = 0 -> owner-secret authorized
        .unwrap()
        .write(&current_daa)
        .unwrap()
        .write(&owner_secret)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();
    eprintln!("Proving the metabolic transition ({mode})... this may take a while.");
    let prove_result = prover.prove_with_opts(env, SCORPION_BRAIN_GUEST_ELF, &ProverOpts::succinct());

    let prove_info = match prove_result {
        Ok(info) => info,
        Err(e) => {
            eprintln!("PROOF CORRECTLY REJECTED -- {label}");
            eprintln!("Guest panic: {e}");
            std::process::exit(0);
        }
    };

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

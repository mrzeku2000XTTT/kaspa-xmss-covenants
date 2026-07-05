
fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
}

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
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&0u64.to_le_bytes());
    d.extend_from_slice(&0u64.to_le_bytes());
    d.push(0);
    d.push(0);
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

// Host program for the Scorpion's Brain — Stage 10 "First Light" (the actual hatch).
//
// This proves the ORIGINAL, literal genesis transition: STEM (generation 0, the
// state the covenant is deployed with, never before proven-into by anything) ->
// VAULT (generation 1, the first moult), using the CURRENT, complete, 10-chromosome,
// 721-byte guest -- not a simplified stand-in. Every global check (identity, memory
// chain, metabolism/rate-limit) that governs every other transition kind governs
// this one too. This is the proof the on-chain P2SH covenant's very first real
// mainnet spend will use.
//
// creation_daa/creation_txid are the organism's birth certificate -- chosen once,
// right here, before the genesis UTXO is even deployed (the deploy tx doesn't exist
// yet, so creation_txid can't literally BE that txid; it's a fixed identity marker,
// same placeholder pattern used in every other Stage's host binary).

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_STEM: u8 = 0;
const MODE_VAULT: u8 = 1;

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

fn build_body(covenant_id: &[u8], generation: u32, mode: u8, prev_mode: u8, owner_commitment: &[u8]) -> Vec<u8> {
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
    let mut out_path = "/tmp/scorpion_hatch_receipt.json".to_string();
    let mut deposit_sompi: u64 = 60_000_000; // default 0.6 KAS fee_reserve baseline
    let mut creation_daa: u64 = 477_500_000;
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--out=") { out_path = v.to_string(); }
        if let Some(v) = a.strip_prefix("--deposit-sompi=") { deposit_sompi = v.parse().unwrap(); }
        if let Some(v) = a.strip_prefix("--creation-daa=") { creation_daa = v.parse().unwrap(); }
    }

    let owner_json: Value = serde_json::from_str(
        &fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap(),
    ).unwrap();
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(
        &fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap(),
    ).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    // Birth certificate: a fresh, real random 32 bytes, saved BEFORE use (key-backup rule).
    // Not a real Kaspa txid (the deploy tx doesn't exist yet) -- a permanent identity marker.
    let creation_txid_path = "/app/kaspa_wrpc/keys/scorpion_creation_txid.json";
    let creation_txid: Vec<u8> = if let Ok(existing) = fs::read_to_string(creation_txid_path) {
        let v: Value = serde_json::from_str(&existing).unwrap();
        hex_to_bytes(v["creation_txid_hex"].as_str().unwrap())
    } else {
        use rand::RngCore;
        let mut buf = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut buf);
        fs::write(creation_txid_path, serde_json::json!({
            "creation_txid_hex": to_hex(&buf),
            "note": "Not a real Kaspa txid -- an immutable birth-certificate identity marker chosen before the genesis deploy tx exists."
        }).to_string()).unwrap();
        buf.to_vec()
    };

    // Metabolic policy (transition=0 sets it): proven compute_budget=2700 (entry 602),
    // KIP-9 storage floor min_output=20M sompi (entry 522), rate policy 5 spends / 1000 DAA.
    let compute_budget: u16 = 2700;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;

    // ── old_dna: the literal genesis state -- STEM, generation 0, never proven-into before ──
    let old_body = build_body(&covenant_id, 0, MODE_STEM, MODE_STEM, &owner_commitment);
    let old_dna_memory = append_memory(old_body, &[0u8; 32], 0, creation_daa, &creation_txid);
    let old_dna_escrow = append_escrow_zeroed(old_dna_memory);
    let old_dna = append_metabolism(
        old_dna_escrow, compute_budget, deposit_sompi, min_output,
        rate_window, rate_max_spend, creation_daa, 0,
    );
    let old_hash = sha256_bytes(&old_dna);

    let current_daa = creation_daa; // first heartbeat happens at birth -- inside the fresh window

    // ── new_dna: VAULT, generation 1 -- the first moult ──
    let new_fee_reserve = deposit_sompi - 10_000_000; // this moult costs something real
    let new_body = build_body(&covenant_id, 1, MODE_VAULT, MODE_STEM, &owner_commitment);
    let new_dna_memory = append_memory(new_body, &old_hash, 1, creation_daa, &creation_txid);
    let new_dna_escrow = append_escrow_zeroed(new_dna_memory);
    let new_dna = append_metabolism(
        new_dna_escrow, compute_budget, new_fee_reserve, min_output,
        rate_window, rate_max_spend, creation_daa, 1,
    );

    eprintln!("STAGE 10 -- THE HATCH");
    eprintln!("covenant_id={}", to_hex(&covenant_id));
    eprintln!("creation_daa={} creation_txid={}", creation_daa, to_hex(&creation_txid));
    eprintln!("old: STEM gen0, fee_reserve={}", deposit_sompi);
    eprintln!("new: VAULT gen1, fee_reserve={}", new_fee_reserve);

    let env = ExecutorEnv::builder()
        .write(&old_dna).unwrap()
        .write(&new_dna).unwrap()
        .write(&0u8).unwrap() // transition = 0, owner-secret authorized
        .write(&current_daa).unwrap()
        .write(&owner_secret).unwrap()
        .build().unwrap();

    let prover = default_prover();
    eprintln!("Proving the genesis moult (STEM -> VAULT)... this may take a while.");
    let prove_info = prover
        .prove_with_opts(env, SCORPION_BRAIN_GUEST_ELF, &ProverOpts::succinct())
        .expect("proving failed");
    let receipt = prove_info.receipt;
    receipt.verify(SCORPION_BRAIN_GUEST_ID).expect("local verification failed");
    eprintln!("PROOF VERIFIED LOCALLY.");

    let journal_bytes = receipt.journal.bytes.clone();
    let journal_digest = receipt.journal.digest();

    let succinct = receipt.inner.succinct().expect("receipt is not a succinct receipt");

    let image_id_bytes: Vec<u8> = SCORPION_BRAIN_GUEST_ID.iter().flat_map(|w: &u32| w.to_le_bytes()).collect();
    let claim_digest = succinct.claim.digest();
    let control_index = succinct.control_inclusion_proof.index;
    let control_digests_bytes: Vec<u8> = succinct
        .control_inclusion_proof
        .digests
        .iter()
        .flat_map(|d| d.as_bytes().to_vec())
        .collect();
    let seal_bytes: Vec<u8> = succinct.seal.iter().flat_map(|w| w.to_le_bytes()).collect();

    eprintln!("journal_bytes (hex) = {}", hex::encode(&journal_bytes));
    eprintln!("control_index = {control_index}, control_digests count = {}", succinct.control_inclusion_proof.digests.len());
    eprintln!("seal len (u32 words) = {}", succinct.seal.len());

    let out = serde_json::json!({
        "image_id_hex": hex::encode(image_id_bytes.as_slice()),
        "journal_bytes_hex": hex::encode(&journal_bytes),
        "journal_digest_hex": hex::encode(journal_digest.as_bytes()),
        "seal_hex": hex::encode(&seal_bytes),
        "control_id_hex": hex::encode(succinct.control_id.as_bytes()),
        "claim_digest_hex": hex::encode(claim_digest.as_bytes()),
        "control_index": control_index,
        "control_digests_hex": hex::encode(&control_digests_bytes),
        "hashfn": format!("{:?}", succinct.hashfn),
        "old_dna_hex": hex::encode(&old_dna),
        "new_dna_hex": hex::encode(&new_dna),
        "old_hash_hex": hex::encode(&old_hash),
        "creation_daa": creation_daa,
        "creation_txid_hex": hex::encode(&creation_txid),
        "deposit_sompi": deposit_sompi,
        "new_fee_reserve": new_fee_reserve,
        "mode": "stage10_hatch_stem_to_vault",
    });
    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote {out_path}");
}

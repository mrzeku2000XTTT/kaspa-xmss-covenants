// Host program for the rollup-practice exercise: builds a chain of N stem<->vault
// state transitions (same owner-secret-authorized logic as the scorpion brain's
// Stage 1), feeds the whole chain to ONE guest execution, and gets back ONE
// STARK proof that covers all N transitions — practicing the core rollup pattern
// (batch many off-chain transitions, settle with a single on-chain-verifiable proof).

use methods::{ROLLUP_PRACTICE_GUEST_ELF, ROLLUP_PRACTICE_GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env as std_env;
use std::fs;

const MAGIC_UNUSED: u8 = 0; // kept for symmetry with scorpion_brain's DNA layout notes

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn build_dna(covenant_id: &[u8], generation: u32, mode: u8, prev_mode: u8, owner_commitment: &[u8]) -> Vec<u8> {
    let _ = MAGIC_UNUSED;
    let mut d = Vec::with_capacity(70);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(mode);
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    assert_eq!(d.len(), 70);
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
    let batch_size: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);
    let out_path = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| "/tmp/rollup_practice_receipt.json".to_string());

    let owner_json: Value = serde_json::from_str(&fs::read_to_string(&owner_json_path).unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());

    let cov_json: Value = serde_json::from_str(&fs::read_to_string(&cov_json_path).unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());

    // Build a chain that alternates stem(0) <-> vault(1) for `batch_size` moults,
    // starting fresh at generation 100 (arbitrary offset so it doesn't collide with
    // any real Stage 1 proof's generation numbering).
    let mut chain: Vec<Vec<u8>> = Vec::with_capacity((batch_size + 1) as usize);
    let mut mode = 0u8; // start at stem
    chain.push(build_dna(&covenant_id, 100, mode, mode, &owner_commitment));
    for i in 0..batch_size {
        let next_mode = 1 - mode; // flip stem<->vault each moult
        let next_gen = 100 + i + 1;
        chain.push(build_dna(&covenant_id, next_gen, next_mode, mode, &owner_commitment));
        mode = next_mode;
    }

    eprintln!("Batch size: {batch_size} chained transitions");
    eprintln!("Chain start (hex): {}", hex::encode(&chain[0]));
    eprintln!("Chain end   (hex): {}", hex::encode(&chain[chain.len() - 1]));

    let mut builder = ExecutorEnv::builder();
    builder.write(&batch_size).unwrap();
    builder.write(&chain[0]).unwrap(); // first old_dna

    for i in 0..batch_size as usize {
        if i > 0 {
            builder.write(&chain[i]).unwrap(); // link i's old_dna (== link i-1's new_dna)
        }
        builder.write(&chain[i + 1]).unwrap(); // link i's new_dna
        builder.write(&owner_secret).unwrap();
    }

    let env = builder.build().unwrap();
    let prover = default_prover();

    eprintln!("Proving the whole batch of {batch_size} moults with ONE STARK (rollup pattern)...");
    let prove_info = prover
        .prove_with_opts(env, ROLLUP_PRACTICE_GUEST_ELF, &ProverOpts::succinct())
        .unwrap();

    let receipt = prove_info.receipt;
    receipt.verify(ROLLUP_PRACTICE_GUEST_ID).expect("local verification failed");

    let journal_bytes = receipt.journal.bytes.clone();
    let succinct = receipt.inner.succinct().expect("not a succinct receipt");

    let image_id_bytes: Vec<u8> = ROLLUP_PRACTICE_GUEST_ID
        .iter()
        .flat_map(|w: &u32| w.to_le_bytes())
        .collect();
    let seal_bytes: Vec<u8> = succinct.seal.iter().flat_map(|w| w.to_le_bytes()).collect();

    // Sanity-check the journal matches what we expect (batch_start_hash, batch_end_hash, N).
    let mut hasher = Sha256::new();
    hasher.update(&chain[0]);
    let expect_start: [u8; 32] = hasher.finalize().into();
    let mut hasher2 = Sha256::new();
    hasher2.update(&chain[chain.len() - 1]);
    let expect_end: [u8; 32] = hasher2.finalize().into();

    assert_eq!(&journal_bytes[0..32], &expect_start[..], "journal start hash mismatch");
    assert_eq!(&journal_bytes[32..64], &expect_end[..], "journal end hash mismatch");
    let journal_n = u32::from_le_bytes(journal_bytes[64..68].try_into().unwrap());
    assert_eq!(journal_n, batch_size, "journal batch size mismatch");

    eprintln!("PROOF VERIFIED LOCALLY — ONE STARK covers all {batch_size} chained moults.");
    eprintln!("journal (hex) = {}", hex::encode(&journal_bytes));

    let out = serde_json::json!({
        "batch_size": batch_size,
        "chain_start_dna_hex": hex::encode(&chain[0]),
        "chain_end_dna_hex": hex::encode(&chain[chain.len() - 1]),
        "image_id_hex": hex::encode(image_id_bytes.as_slice()),
        "journal_bytes_hex": hex::encode(&journal_bytes),
        "seal_hex": hex::encode(&seal_bytes),
        "control_id_hex": hex::encode(succinct.control_id.as_bytes()),
        "hashfn": format!("{:?}", succinct.hashfn),
    });

    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote receipt data to {out_path}");
}

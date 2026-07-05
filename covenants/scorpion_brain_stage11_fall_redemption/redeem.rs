// redeem.rs -- Stage 11: REDEMPTION HEARTBEAT ("still trying, still on probation")
//
// Takes a prior FALL (or REDEEM) receipt and proves ONE redemption heartbeat
// (transition=8): the owner proves control via owner_secret, redemption_progress
// advances by exactly 1, mode stays MODE_FALLEN. Also runs a negative test:
// a redemption heartbeat attempted with the WRONG owner_secret must be rejected.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts};
use serde_json::Value;
use std::env as std_env;
use std::fs;

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
}
fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}
fn u32le(b: &[u8]) -> u32 { u32::from_le_bytes(b.try_into().unwrap()) }
fn u64le(b: &[u8]) -> u64 { u64::from_le_bytes(b.try_into().unwrap()) }

fn run_proof(old_dna: &[u8], new_dna: &[u8], current_daa: u64, owner_secret: &[u8], expect_fail: bool) -> Option<risc0_zkvm::Receipt> {
    let env = ExecutorEnv::builder()
        .write(&old_dna.to_vec()).unwrap()
        .write(&new_dna.to_vec()).unwrap()
        .write(&8u8).unwrap() // transition = 8, redemption heartbeat
        .write(&current_daa).unwrap()
        .write(&owner_secret.to_vec()).unwrap()
        .build().unwrap();
    let prover = default_prover();
    match prover.prove_with_opts(env, SCORPION_BRAIN_GUEST_ELF, &ProverOpts::succinct()) {
        Ok(prove_info) => {
            if expect_fail { panic!("expected FAIL but succeeded -- security bug!"); }
            let receipt = prove_info.receipt;
            receipt.verify(SCORPION_BRAIN_GUEST_ID).expect("local verification failed");
            Some(receipt)
        }
        Err(e) => {
            if expect_fail { eprintln!("  correctly REJECTED: {e}"); None }
            else { panic!("proving failed unexpectedly: {e}"); }
        }
    }
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env()).init();
    let args: Vec<String> = std_env::args().collect();
    let mut prev_receipt_path = "/app/kaspa_wrpc/keys/scorpion_stage11_fall_receipt.json".to_string();
    let mut out_path = "/tmp/scorpion_redeem_receipt.json".to_string();
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--prev-receipt=") { prev_receipt_path = v.to_string(); }
        if let Some(v) = a.strip_prefix("--out=") { out_path = v.to_string(); }
    }

    let prev: Value = serde_json::from_str(&fs::read_to_string(&prev_receipt_path).unwrap()).unwrap();
    let old_dna = hex_to_bytes(prev["new_dna_hex"].as_str().unwrap());
    assert_eq!(old_dna.len(), 741, "prev receipt's new_dna_hex must be the full 741-byte DNA");
    let old_hash = sha256_bytes(&old_dna);

    let owner_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap()).unwrap();
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());

    let generation = u32le(&old_dna[36..40]);
    let fee_reserve = u64le(&old_dna[685..693]);
    let redemption_deadline = u64le(&old_dna[730..738]);
    let redemption_progress = old_dna[738];
    let rate_window = u32le(&old_dna[701..705]) as u64;
    let window_start_daa = u64le(&old_dna[709..717]);
    let window_spend_count = u32le(&old_dna[717..721]);
    let transition_count = u32le(&old_dna[525..529]);

    // Stay comfortably inside the redemption window for this proof-of-mechanism heartbeat.
    let current_daa = redemption_deadline - 50_000;
    assert!(current_daa <= redemption_deadline, "test setup error");

    let mut new_dna = old_dna.clone();
    new_dna[36..40].copy_from_slice(&(generation + 1).to_le_bytes());
    new_dna[40] = 6; // MODE_FALLEN
    new_dna[41] = 6; // prev_mode = MODE_FALLEN
    new_dna[493..525].copy_from_slice(&old_hash);
    new_dna[525..529].copy_from_slice(&(transition_count + 1).to_le_bytes());
    let new_fee_reserve = fee_reserve - 1_000_000;
    new_dna[685..693].copy_from_slice(&new_fee_reserve.to_le_bytes());
    if current_daa - window_start_daa > rate_window {
        new_dna[709..717].copy_from_slice(&current_daa.to_le_bytes());
        new_dna[717..721].copy_from_slice(&1u32.to_le_bytes());
    } else {
        new_dna[717..721].copy_from_slice(&(window_spend_count + 1).to_le_bytes());
    }
    new_dna[738] = redemption_progress + 1; // the whole point of this transition

    eprintln!("── negative test: redemption heartbeat with the WRONG owner_secret ──");
    let mut wrong_secret = owner_secret.clone();
    wrong_secret[0] ^= 0xFF;
    run_proof(&old_dna, &new_dna, current_daa, &wrong_secret, true);

    eprintln!("── real proof: redemption heartbeat #{} (progress {} -> {}) ──", redemption_progress + 1, redemption_progress, redemption_progress + 1);
    let receipt = run_proof(&old_dna, &new_dna, current_daa, &owner_secret, false).unwrap();
    eprintln!("PROOF VERIFIED LOCALLY. Still fallen, but one honest heartbeat closer to the light.");

    let journal_bytes = receipt.journal.bytes.clone();
    let journal_digest = receipt.journal.digest();
    let succinct = receipt.inner.succinct().expect("receipt is not a succinct receipt");
    let image_id_bytes: Vec<u8> = SCORPION_BRAIN_GUEST_ID.iter().flat_map(|w: &u32| w.to_le_bytes()).collect();
    let claim_digest = succinct.claim.digest();
    let control_index = succinct.control_inclusion_proof.index;
    let control_digests_bytes: Vec<u8> = succinct.control_inclusion_proof.digests.iter().flat_map(|d| d.as_bytes().to_vec()).collect();
    let seal_bytes: Vec<u8> = succinct.seal.iter().flat_map(|w| w.to_le_bytes()).collect();

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
        "current_daa": current_daa,
        "redemption_progress": redemption_progress + 1,
        "kind": "stage11_redeem",
    });
    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote {out_path}");
}

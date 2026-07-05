// fossilize.rs -- Stage 11: FOSSILIZATION ("the chance was squandered, this is final")
//
// Builds a synthetic FALLEN organism whose redemption_deadline has already
// lapsed with insufficient redemption_progress, and proves the permissionless
// FOSSILIZE transition (transition=10) into MODE_FOSSIL -- the true terminal
// state. Also runs two negative tests: fossilizing before the deadline lapses,
// and fossilizing an organism that actually earned enough redemption (should
// ascend instead, not die).

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_FALLEN: u8 = 6;
const MODE_FOSSIL: u8 = 7;
const FALL_REASON_STARVATION: u8 = 1;
const REDEMPTION_WINDOW_DAA: u64 = 100_000;
const REDEMPTION_THRESHOLD: u8 = 5;

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
}
fn to_hex(b: &[u8]) -> String { b.iter().map(|x| format!("{:02x}", x)).collect() }
fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

fn build_body(covenant_id: &[u8], generation: u32, mode: u8, prev_mode: u8, owner_commitment: &[u8]) -> Vec<u8> {
    let mut d = Vec::with_capacity(493);
    d.extend_from_slice(MAGIC);
    d.extend_from_slice(covenant_id);
    d.extend_from_slice(&generation.to_le_bytes());
    d.push(mode);
    d.push(prev_mode);
    d.extend_from_slice(owner_commitment);
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&0u32.to_le_bytes());
    d.extend_from_slice(&0u64.to_le_bytes());
    d.extend_from_slice(&0u32.to_le_bytes());
    d.push(0);
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&0u32.to_le_bytes());
    d.extend_from_slice(&0u32.to_le_bytes());
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&0u32.to_le_bytes());
    d.push(0);
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&0u64.to_le_bytes());
    d.push(0);
    d.extend_from_slice(&0u64.to_le_bytes());
    d.extend_from_slice(&0u64.to_le_bytes());
    d.extend_from_slice(&0u64.to_le_bytes());
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&[0u8; 32]);
    d.push(0);
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&[0u8; 32]);
    d.push(0);
    d.extend_from_slice(&[0u8; 32]);
    d.push(0);
    d.push(0);
    assert_eq!(d.len(), 493);
    d
}
fn append_memory(mut d: Vec<u8>, prev_state_hash: &[u8], transition_count: u32, creation_daa: u64, creation_txid: &[u8]) -> Vec<u8> {
    d.extend_from_slice(prev_state_hash);
    d.extend_from_slice(&transition_count.to_le_bytes());
    d.extend_from_slice(&creation_daa.to_le_bytes());
    d.extend_from_slice(creation_txid);
    assert_eq!(d.len(), 569);
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
    assert_eq!(d.len(), 683);
    d
}
#[allow(clippy::too_many_arguments)]
fn append_metabolism(mut d: Vec<u8>, compute_budget: u16, fee_reserve: u64, min_output: u64, rate_window: u32, rate_max_spend: u32, window_start_daa: u64, window_spend_count: u32) -> Vec<u8> {
    d.extend_from_slice(&compute_budget.to_le_bytes());
    d.extend_from_slice(&fee_reserve.to_le_bytes());
    d.extend_from_slice(&min_output.to_le_bytes());
    d.extend_from_slice(&rate_window.to_le_bytes());
    d.extend_from_slice(&rate_max_spend.to_le_bytes());
    d.extend_from_slice(&window_start_daa.to_le_bytes());
    d.extend_from_slice(&window_spend_count.to_le_bytes());
    assert_eq!(d.len(), 721);
    d
}
#[allow(clippy::too_many_arguments)]
fn append_fall(mut d: Vec<u8>, fall_reason: u8, fallen_at_daa: u64, redemption_deadline: u64, redemption_progress: u8, redemption_threshold: u8, fossilized: u8) -> Vec<u8> {
    d.push(fall_reason);
    d.extend_from_slice(&fallen_at_daa.to_le_bytes());
    d.extend_from_slice(&redemption_deadline.to_le_bytes());
    d.push(redemption_progress);
    d.push(redemption_threshold);
    d.push(fossilized);
    assert_eq!(d.len(), 741);
    d
}

fn apply_window(new_dna: &mut [u8], old_window_start: u64, old_window_count: u32, rate_window: u64, current_daa: u64) {
    if current_daa - old_window_start > rate_window {
        new_dna[709..717].copy_from_slice(&current_daa.to_le_bytes());
        new_dna[717..721].copy_from_slice(&1u32.to_le_bytes());
    } else {
        new_dna[709..717].copy_from_slice(&old_window_start.to_le_bytes());
        new_dna[717..721].copy_from_slice(&(old_window_count + 1).to_le_bytes());
    }
}

fn run_proof(old_dna: &[u8], new_dna: &[u8], current_daa: u64, expect_fail: bool) -> Option<risc0_zkvm::Receipt> {
    let env = ExecutorEnv::builder()
        .write(&old_dna.to_vec()).unwrap()
        .write(&new_dna.to_vec()).unwrap()
        .write(&10u8).unwrap() // transition = 10, fossilize
        .write(&current_daa).unwrap()
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
    let mut out_path = "/tmp/scorpion_fossilize_receipt.json".to_string();
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--out=") { out_path = v.to_string(); }
    }

    let owner_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());
    let cov_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());
    let creation_txid = hex_to_bytes(&"22".repeat(32));
    let creation_daa: u64 = 477_600_000;
    let fallen_at_daa: u64 = creation_daa;
    let redemption_deadline = fallen_at_daa + REDEMPTION_WINDOW_DAA;

    let compute_budget: u16 = 2700;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;

    // ── A FALLEN organism, redemption squandered: progress=2, threshold=5 ──
    let body = build_body(&covenant_id, 6, MODE_FALLEN, 1, &owner_commitment);
    let mem = append_memory(body, &[0x77u8; 32], 6, creation_daa, &creation_txid);
    let esc = append_escrow_zeroed(mem);
    let fallen_dna = append_fall(
        append_metabolism(esc, compute_budget, 10_000_000, min_output, rate_window, rate_max_spend, creation_daa, 1),
        FALL_REASON_STARVATION, fallen_at_daa, redemption_deadline, 2, REDEMPTION_THRESHOLD, 0,
    );
    let old_hash = sha256_bytes(&fallen_dna);

    let mut fossil_dna = fallen_dna.clone();
    fossil_dna[36..40].copy_from_slice(&7u32.to_le_bytes()); // generation+1
    fossil_dna[40] = MODE_FOSSIL;
    fossil_dna[41] = MODE_FALLEN;
    fossil_dna[493..525].copy_from_slice(&old_hash);
    fossil_dna[525..529].copy_from_slice(&7u32.to_le_bytes());
    fossil_dna[685..693].copy_from_slice(&5_000_000u64.to_le_bytes());
    fossil_dna[717..721].copy_from_slice(&2u32.to_le_bytes());
    fossil_dna[740] = 1; // fossilized = 1, the permanent marker

    // ── NEGATIVE TEST 1: fossilize BEFORE the redemption deadline has lapsed ──
    eprintln!("── negative test: fossilize attempt before the redemption deadline ──");
    let daa1 = redemption_deadline - 1_000;
    let mut fossil_dna_1 = fossil_dna.clone();
    apply_window(&mut fossil_dna_1, creation_daa, 1, rate_window as u64, daa1);
    run_proof(&fallen_dna, &fossil_dna_1, daa1, true);

    // ── NEGATIVE TEST 2: fossilize an organism that already earned enough redemption ──
    eprintln!("── negative test: fossilize an organism that earned enough redemption (should ascend, not die) ──");
    let body2 = build_body(&covenant_id, 11, MODE_FALLEN, 6, &owner_commitment);
    let mem2 = append_memory(body2, &[0x88u8; 32], 11, creation_daa, &creation_txid);
    let esc2 = append_escrow_zeroed(mem2);
    let redeemed_dna = append_fall(
        append_metabolism(esc2, compute_budget, 5_000_000, min_output, rate_window, rate_max_spend, creation_daa, 1),
        FALL_REASON_STARVATION, fallen_at_daa, redemption_deadline, REDEMPTION_THRESHOLD, REDEMPTION_THRESHOLD, 0,
    );
    let redeemed_hash = sha256_bytes(&redeemed_dna);
    let mut bad_fossil = redeemed_dna.clone();
    bad_fossil[36..40].copy_from_slice(&12u32.to_le_bytes());
    bad_fossil[40] = MODE_FOSSIL;
    bad_fossil[41] = MODE_FALLEN;
    bad_fossil[493..525].copy_from_slice(&redeemed_hash);
    bad_fossil[525..529].copy_from_slice(&12u32.to_le_bytes());
    bad_fossil[685..693].copy_from_slice(&2_000_000u64.to_le_bytes());
    bad_fossil[740] = 1;
    let daa2 = redemption_deadline + 1_000;
    apply_window(&mut bad_fossil, creation_daa, 1, rate_window as u64, daa2);
    run_proof(&redeemed_dna, &bad_fossil, daa2, true);

    // ── REAL FOSSILIZE: deadline lapsed, redemption squandered (2 < 5) ──
    eprintln!("── real proof: redemption window squandered -> FOSSILIZATION ──");
    eprintln!("old: FALLEN gen6, redemption_progress=2/{}, redemption_deadline={}", REDEMPTION_THRESHOLD, redemption_deadline);
    eprintln!("new: FOSSIL gen7 -- permanent. The chitin remains. Nothing transitions from here, ever again.");
    let current_daa = redemption_deadline + 1_000;
    apply_window(&mut fossil_dna, creation_daa, 1, rate_window as u64, current_daa);
    let receipt = run_proof(&fallen_dna, &fossil_dna, current_daa, false).unwrap();
    eprintln!("PROOF VERIFIED LOCALLY. FOSSILIZED. This is the grave.");

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
        "old_dna_hex": hex::encode(&fallen_dna),
        "new_dna_hex": hex::encode(&fossil_dna),
        "current_daa": current_daa,
        "covenant_id_hex": to_hex(&covenant_id),
        "kind": "stage11_fossilize",
    });
    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote {out_path}");
}

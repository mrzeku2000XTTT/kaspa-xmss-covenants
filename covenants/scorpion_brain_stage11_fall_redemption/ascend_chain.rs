// ascend_chain.rs -- Stage 11: THE FULL CLIMB ("earned its way back into the light")
//
// Builds a fresh FALLEN organism, then chains 5 REAL redemption heartbeat
// proofs (transition=8) to reach redemption_threshold, then proves the final
// ASCEND transition (transition=9): FALLEN -> VAULT, owner resupplies
// fee_reserve (the one deliberate exception to "fee_reserve must strictly
// decrease"), and the fall record is wiped to a clean slate.
//
// Also runs a fast negative test: attempting to ascend BEFORE enough
// redemption heartbeats have accumulated must be rejected.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts, Receipt};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_VAULT: u8 = 1;
const MODE_FALLEN: u8 = 6;
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
fn u32le(b: &[u8]) -> u32 { u32::from_le_bytes(b.try_into().unwrap()) }
fn u64le(b: &[u8]) -> u64 { u64::from_le_bytes(b.try_into().unwrap()) }

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

fn run_proof(old_dna: &[u8], new_dna: &[u8], transition: u8, current_daa: u64, owner_secret: Option<&[u8]>, expect_fail: bool) -> Option<Receipt> {
    let mut b = ExecutorEnv::builder();
    b.write(&old_dna.to_vec()).unwrap();
    b.write(&new_dna.to_vec()).unwrap();
    b.write(&transition).unwrap();
    b.write(&current_daa).unwrap();
    if let Some(s) = owner_secret { b.write(&s.to_vec()).unwrap(); }
    let env = b.build().unwrap();
    let prover = default_prover();
    match prover.prove_with_opts(env, SCORPION_BRAIN_GUEST_ELF, &ProverOpts::succinct()) {
        Ok(prove_info) => {
            if expect_fail { panic!("expected this proof attempt to FAIL but it succeeded -- security bug!"); }
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
    let mut out_path = "/tmp/scorpion_ascend_receipt.json".to_string();
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--out=") { out_path = v.to_string(); }
    }

    let owner_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());
    let cov_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());
    let creation_txid = hex_to_bytes(&"33".repeat(32));
    let creation_daa: u64 = 477_600_000;
    let fallen_at_daa: u64 = creation_daa;
    let redemption_deadline = fallen_at_daa + REDEMPTION_WINDOW_DAA;

    let compute_budget: u16 = 2700;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;

    // ── Fresh FALLEN organism, redemption_progress = 0 ──
    let body = build_body(&covenant_id, 6, MODE_FALLEN, 1, &owner_commitment);
    let mem = append_memory(body, &[0x55u8; 32], 6, creation_daa, &creation_txid);
    let esc = append_escrow_zeroed(mem);
    let mut cur_dna = append_fall(
        append_metabolism(esc, compute_budget, 40_000_000, min_output, rate_window, rate_max_spend, creation_daa, 1),
        FALL_REASON_STARVATION, fallen_at_daa, redemption_deadline, 0, REDEMPTION_THRESHOLD, 0,
    );

    eprintln!("Starting fresh FALLEN organism: gen6, redemption_progress=0/{}", REDEMPTION_THRESHOLD);

    // ── Chain 5 real REDEEM proofs ──
    for i in 1..=REDEMPTION_THRESHOLD {
        let generation = u32le(&cur_dna[36..40]);
        let fee_reserve = u64le(&cur_dna[685..693]);
        let redemption_progress = cur_dna[738];
        let window_start_daa = u64le(&cur_dna[709..717]);
        let window_spend_count = u32le(&cur_dna[717..721]);
        let transition_count = u32le(&cur_dna[525..529]);
        let old_hash = sha256_bytes(&cur_dna);

        // spread each heartbeat out but stay well inside the deadline
        let current_daa = fallen_at_daa + (i as u64) * 5_000;

        let mut new_dna = cur_dna.clone();
        new_dna[36..40].copy_from_slice(&(generation + 1).to_le_bytes());
        new_dna[40] = MODE_FALLEN;
        new_dna[41] = MODE_FALLEN;
        new_dna[493..525].copy_from_slice(&old_hash);
        new_dna[525..529].copy_from_slice(&(transition_count + 1).to_le_bytes());
        new_dna[685..693].copy_from_slice(&(fee_reserve - 1_000_000).to_le_bytes());
        apply_window(&mut new_dna, window_start_daa, window_spend_count, rate_window as u64, current_daa);
        new_dna[738] = redemption_progress + 1;

        eprintln!("── REDEEM heartbeat #{i}: redemption_progress {} -> {} ──", redemption_progress, redemption_progress + 1);
        run_proof(&cur_dna, &new_dna, 8, current_daa, Some(&owner_secret), false);
        eprintln!("  verified.");
        cur_dna = new_dna;
    }

    assert_eq!(cur_dna[738], REDEMPTION_THRESHOLD, "should have reached the redemption threshold");
    let ascend_daa = fallen_at_daa + (REDEMPTION_THRESHOLD as u64) * 5_000 + 1_000;

    // ── NEGATIVE TEST: try to ascend from an EARLIER point where progress < threshold ──
    eprintln!("── negative test: ascend attempt with insufficient redemption_progress ──");
    let body_early = build_body(&covenant_id, 8, MODE_FALLEN, 6, &owner_commitment);
    let mem_early = append_memory(body_early, &[0x66u8; 32], 8, creation_daa, &creation_txid);
    let esc_early = append_escrow_zeroed(mem_early);
    let early_fallen = append_fall(
        append_metabolism(esc_early, compute_budget, 37_000_000, min_output, rate_window, rate_max_spend, creation_daa, 1),
        FALL_REASON_STARVATION, fallen_at_daa, redemption_deadline, 2, REDEMPTION_THRESHOLD, 0,
    );
    let mut early_ascend = early_fallen.clone();
    early_ascend[36..40].copy_from_slice(&9u32.to_le_bytes());
    early_ascend[40] = MODE_VAULT;
    early_ascend[41] = MODE_FALLEN;
    let eh = sha256_bytes(&early_fallen);
    early_ascend[493..525].copy_from_slice(&eh);
    early_ascend[525..529].copy_from_slice(&9u32.to_le_bytes());
    early_ascend[685..693].copy_from_slice(&50_000_000u64.to_le_bytes()); // resupplied, but progress too low
    apply_window(&mut early_ascend, creation_daa, 1, rate_window as u64, ascend_daa);
    early_ascend[721] = 0;
    early_ascend[722..730].copy_from_slice(&0u64.to_le_bytes());
    early_ascend[730..738].copy_from_slice(&0u64.to_le_bytes());
    early_ascend[738] = 0;
    early_ascend[740] = 0;
    run_proof(&early_fallen, &early_ascend, 9, ascend_daa, Some(&owner_secret), true);

    // ── REAL ASCEND: threshold met, owner resupplies fee_reserve, clean slate ──
    eprintln!("── real proof: ASCENSION -- earned its way back into the light ──");
    let old_fee_reserve = u64le(&cur_dna[685..693]);
    let new_fee_reserve = old_fee_reserve + 60_000_000; // real resupply, strictly greater
    let old_hash = sha256_bytes(&cur_dna);
    let generation = u32le(&cur_dna[36..40]);
    let transition_count = u32le(&cur_dna[525..529]);
    let window_start_daa = u64le(&cur_dna[709..717]);
    let window_spend_count = u32le(&cur_dna[717..721]);

    let mut ascend_dna = cur_dna.clone();
    ascend_dna[36..40].copy_from_slice(&(generation + 1).to_le_bytes());
    ascend_dna[40] = MODE_VAULT;
    ascend_dna[41] = MODE_FALLEN;
    ascend_dna[493..525].copy_from_slice(&old_hash);
    ascend_dna[525..529].copy_from_slice(&(transition_count + 1).to_le_bytes());
    ascend_dna[685..693].copy_from_slice(&new_fee_reserve.to_le_bytes());
    apply_window(&mut ascend_dna, window_start_daa, window_spend_count, rate_window as u64, ascend_daa);
    // clean slate -- Chromosome 11 wiped
    ascend_dna[721] = 0;
    ascend_dna[722..730].copy_from_slice(&0u64.to_le_bytes());
    ascend_dna[730..738].copy_from_slice(&0u64.to_le_bytes());
    ascend_dna[738] = 0;
    ascend_dna[740] = 0;

    eprintln!("old: FALLEN gen{}, redemption_progress={}/{}, fee_reserve={}", generation, REDEMPTION_THRESHOLD, REDEMPTION_THRESHOLD, old_fee_reserve);
    eprintln!("new: VAULT gen{}, clean slate, fee_reserve resupplied to {}", generation + 1, new_fee_reserve);
    let receipt = run_proof(&cur_dna, &ascend_dna, 9, ascend_daa, Some(&owner_secret), false).unwrap();
    eprintln!("PROOF VERIFIED LOCALLY. THE ORGANISM HAS ASCENDED. IT LIVES.");

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
        "old_dna_hex": hex::encode(&cur_dna),
        "new_dna_hex": hex::encode(&ascend_dna),
        "current_daa": ascend_daa,
        "old_fee_reserve": old_fee_reserve,
        "new_fee_reserve": new_fee_reserve,
        "covenant_id_hex": to_hex(&covenant_id),
        "kind": "stage11_ascend",
    });
    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote {out_path}");
}

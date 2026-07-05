// fall.rs -- Stage 11: THE FALL ("how they betrayed the light")
//
// Builds a synthetic VAULT organism whose fee_reserve has genuinely dropped
// below its own min_output policy (starvation), then proves the permissionless
// FALL transition (transition=7) into MODE_FALLEN. No owner_secret required --
// on purpose. Also runs a fast negative test first: claiming starvation when
// the organism is actually healthy must be rejected by the guest.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts};
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
    assert_eq!(d.len(), 741, "DNA must be 741 bytes once Chromosome 11 (fall/redemption) is appended");
    d
}

fn run_proof(old_dna: &[u8], new_dna: &[u8], transition: u8, current_daa: u64, extra_reason: Option<u8>, expect_fail: bool) -> Option<risc0_zkvm::Receipt> {
    let mut b = ExecutorEnv::builder();
    b.write(&old_dna.to_vec()).unwrap();
    b.write(&new_dna.to_vec()).unwrap();
    b.write(&transition).unwrap();
    b.write(&current_daa).unwrap();
    if let Some(r) = extra_reason { b.write(&r).unwrap(); }
    let env = b.build().unwrap();
    let prover = default_prover();
    let result = prover.prove_with_opts(env, SCORPION_BRAIN_GUEST_ELF, &ProverOpts::succinct());
    match result {
        Ok(prove_info) => {
            if expect_fail {
                panic!("expected this proof attempt to FAIL but it succeeded -- security bug!");
            }
            let receipt = prove_info.receipt;
            receipt.verify(SCORPION_BRAIN_GUEST_ID).expect("local verification failed");
            Some(receipt)
        }
        Err(e) => {
            if expect_fail {
                eprintln!("  correctly REJECTED: {e}");
                None
            } else {
                panic!("proving failed unexpectedly: {e}");
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env()).init();
    let args: Vec<String> = std_env::args().collect();
    let mut out_path = "/tmp/scorpion_fall_receipt.json".to_string();
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--out=") { out_path = v.to_string(); }
    }

    use serde_json::Value;
    let owner_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());
    let cov_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());
    let creation_txid = hex_to_bytes(&"11".repeat(32));
    let creation_daa: u64 = 477_600_000;
    let current_daa = creation_daa;

    let compute_budget: u16 = 2700;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;

    // ── Healthy VAULT organism, generation 5, fee_reserve ABOVE min_output ──
    let body = build_body(&covenant_id, 5, MODE_VAULT, MODE_VAULT, &owner_commitment);
    let mem = append_memory(body, &[0x42u8; 32], 5, creation_daa, &creation_txid);
    let esc = append_escrow_zeroed(mem);
    let healthy_dna = append_fall(
        append_metabolism(esc, compute_budget, 30_000_000, min_output, rate_window, rate_max_spend, creation_daa, 1),
        0, 0, 0, 0, 0, 0,
    );

    // ── NEGATIVE TEST: claim starvation on a HEALTHY organism -- must be rejected ──
    eprintln!("── negative test: false starvation claim on a healthy organism ──");
    let mut new_dna_false = healthy_dna.clone();
    new_dna_false[36..40].copy_from_slice(&6u32.to_le_bytes()); // generation+1
    new_dna_false[40] = MODE_FALLEN;
    new_dna_false[41] = MODE_VAULT;
    let old_hash = sha256_bytes(&healthy_dna);
    new_dna_false[493..525].copy_from_slice(&old_hash);
    new_dna_false[525..529].copy_from_slice(&6u32.to_le_bytes());
    new_dna_false[685..693].copy_from_slice(&25_000_000u64.to_le_bytes()); // still decreases, fine
    new_dna_false[717..721].copy_from_slice(&2u32.to_le_bytes()); // window_spend_count old(1)+1
    new_dna_false[721] = FALL_REASON_STARVATION;
    new_dna_false[722..730].copy_from_slice(&current_daa.to_le_bytes());
    new_dna_false[730..738].copy_from_slice(&(current_daa + REDEMPTION_WINDOW_DAA).to_le_bytes());
    new_dna_false[738] = 0;
    new_dna_false[739] = REDEMPTION_THRESHOLD;
    new_dna_false[740] = 0;
    run_proof(&healthy_dna, &new_dna_false, 7, current_daa, Some(FALL_REASON_STARVATION), true);

    // ── REAL FALL: a genuinely starved VAULT organism ──
    eprintln!("── real proof: genuine starvation -> THE FALL ──");
    let body2 = build_body(&covenant_id, 5, MODE_VAULT, MODE_VAULT, &owner_commitment);
    let mem2 = append_memory(body2, &[0x42u8; 32], 5, creation_daa, &creation_txid);
    let esc2 = append_escrow_zeroed(mem2);
    let starved_dna = append_fall(
        append_metabolism(esc2, compute_budget, 15_000_000, min_output, rate_window, rate_max_spend, creation_daa, 1),
        0, 0, 0, 0, 0, 0,
    );
    let mut fallen_dna = starved_dna.clone();
    fallen_dna[36..40].copy_from_slice(&6u32.to_le_bytes());
    fallen_dna[40] = MODE_FALLEN;
    fallen_dna[41] = MODE_VAULT;
    let starved_hash = sha256_bytes(&starved_dna);
    fallen_dna[493..525].copy_from_slice(&starved_hash);
    fallen_dna[525..529].copy_from_slice(&6u32.to_le_bytes());
    fallen_dna[685..693].copy_from_slice(&10_000_000u64.to_le_bytes());
    fallen_dna[717..721].copy_from_slice(&2u32.to_le_bytes()); // window_spend_count old(1)+1
    fallen_dna[721] = FALL_REASON_STARVATION;
    fallen_dna[722..730].copy_from_slice(&current_daa.to_le_bytes());
    fallen_dna[730..738].copy_from_slice(&(current_daa + REDEMPTION_WINDOW_DAA).to_le_bytes());
    fallen_dna[738] = 0;
    fallen_dna[739] = REDEMPTION_THRESHOLD;
    fallen_dna[740] = 0;

    eprintln!("old: VAULT gen5, fee_reserve=15,000,000 (below min_output=20,000,000 -- genuinely starved)");
    eprintln!("new: FALLEN gen6, fall_reason=STARVATION, redemption_deadline={}", current_daa + REDEMPTION_WINDOW_DAA);
    let receipt = run_proof(&starved_dna, &fallen_dna, 7, current_daa, Some(FALL_REASON_STARVATION), false).unwrap();
    eprintln!("PROOF VERIFIED LOCALLY. THE ORGANISM HAS FALLEN, BUT IT IS NOT YET DEAD.");

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
        "old_dna_hex": hex::encode(&starved_dna),
        "new_dna_hex": hex::encode(&fallen_dna),
        "current_daa": current_daa,
        "redemption_deadline": current_daa + REDEMPTION_WINDOW_DAA,
        "covenant_id_hex": to_hex(&covenant_id),
        "kind": "stage11_fall",
    });
    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote {out_path}");
}

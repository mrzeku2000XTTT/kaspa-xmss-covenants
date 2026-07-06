// crumble_dust.rs -- Stage 13: DUST ("ashes to ashes, the true end")
//
// Builds a fresh FOSSIL organism carrying a dust-level fee_reserve (below its
// own min_output floor -- genuinely too small to ever be spent as a real
// output again), then proves the CRUMBLE transition (transition=13):
// FOSSIL -> DUST, fee_reserve written off to exactly zero, permissionless.
//
// Negative test: attempting to crumble while fee_reserve is still ABOVE
// min_output (not yet true dust) must be rejected.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts, Receipt};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_STEM: u8 = 0;
const MODE_FOSSIL: u8 = 7;
const MODE_DUST: u8 = 9;
const FALL_REASON_STARVATION: u8 = 1;

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
fn append_hunt_zeroed(mut d: Vec<u8>) -> Vec<u8> {
    d.extend_from_slice(&[0u8; 32]); // hunt_target_id
    d.extend_from_slice(&0u64.to_le_bytes()); // hunt_wager
    d.extend_from_slice(&[0u8; 32]); // hunter_move_commit
    d.extend_from_slice(&[0u8; 32]); // prey_move_commit
    d.push(0); // hunt_state
    assert_eq!(d.len(), 846);
    d
}
fn append_dust(mut d: Vec<u8>, dusted_at_daa: u64) -> Vec<u8> {
    d.extend_from_slice(&dusted_at_daa.to_le_bytes());
    assert_eq!(d.len(), 854);
    d
}

fn run_proof(old_dna: &[u8], new_dna: &[u8], transition: u8, current_daa: u64, expect_fail: bool) -> Option<Receipt> {
    let mut b = ExecutorEnv::builder();
    b.write(&old_dna.to_vec()).unwrap();
    b.write(&new_dna.to_vec()).unwrap();
    b.write(&transition).unwrap();
    b.write(&current_daa).unwrap();
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

fn write_receipt(receipt: &Receipt, old_dna: &[u8], new_dna: &[u8], current_daa: u64, kind: &str, out_path: &str) {
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
        "old_dna_hex": hex::encode(old_dna),
        "new_dna_hex": hex::encode(new_dna),
        "current_daa": current_daa,
        "kind": kind,
    });
    fs::write(out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote {out_path}");
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env()).init();
    let args: Vec<String> = std_env::args().collect();
    let mut out_dir = "/app/kaspa_wrpc/keys".to_string();
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--out-dir=") { out_dir = v.to_string(); }
    }

    let owner_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());
    let cov_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());
    let creation_txid = hex_to_bytes(&"55".repeat(32));
    let creation_daa: u64 = 478_000_000;
    let fallen_at_daa = creation_daa;
    let redemption_deadline = fallen_at_daa + 100_000;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;

    // ══════════════════════ NEGATIVE TEST: fee_reserve NOT yet true dust ══════════════════════
    eprintln!("── negative test: crumble attempt while fee_reserve is still above min_output ──");
    let body_early = build_body(&covenant_id, 12, MODE_FOSSIL, 6, &owner_commitment);
    let mem_early = append_memory(body_early, &[0x77u8; 32], 12, creation_daa, &creation_txid);
    let esc_early = append_escrow_zeroed(mem_early);
    // fee_reserve = 25M, ABOVE min_output (20M) -- not true dust yet
    let fossil_not_dust = append_dust(
        append_hunt_zeroed(append_fall(
            append_metabolism(esc_early, 2700, 25_000_000, min_output, rate_window, rate_max_spend, creation_daa, 1),
            FALL_REASON_STARVATION, fallen_at_daa, redemption_deadline, 2, 5, 1,
        )),
        0,
    );
    let crumble_daa_early = creation_daa + 200_000;
    let mut bad_crumble = fossil_not_dust.clone();
    bad_crumble[36..40].copy_from_slice(&13u32.to_le_bytes());
    bad_crumble[40] = MODE_DUST;
    bad_crumble[41] = MODE_FOSSIL;
    let eh = sha256_bytes(&fossil_not_dust);
    bad_crumble[493..525].copy_from_slice(&eh);
    bad_crumble[525..529].copy_from_slice(&13u32.to_le_bytes());
    bad_crumble[685..693].copy_from_slice(&0u64.to_le_bytes());
    bad_crumble[709..717].copy_from_slice(&crumble_daa_early.to_le_bytes());
    bad_crumble[717..721].copy_from_slice(&1u32.to_le_bytes());
    bad_crumble[846..854].copy_from_slice(&crumble_daa_early.to_le_bytes());
    run_proof(&fossil_not_dust, &bad_crumble, 13, crumble_daa_early, true);

    // ══════════════════════ REAL PROOF: genuine dust crumbles ══════════════════════
    eprintln!("── real proof: CRUMBLE -- ashes to ashes ──");
    let body = build_body(&covenant_id, 12, MODE_FOSSIL, 6, &owner_commitment);
    let mem = append_memory(body, &[0x88u8; 32], 12, creation_daa, &creation_txid);
    let esc = append_escrow_zeroed(mem);
    // fee_reserve = 3M, well BELOW min_output (20M) -- genuinely un-spendable dust
    let old_fee_reserve: u64 = 3_000_000;
    let fossil_dust = append_dust(
        append_hunt_zeroed(append_fall(
            append_metabolism(esc, 2700, old_fee_reserve, min_output, rate_window, rate_max_spend, creation_daa, 1),
            FALL_REASON_STARVATION, fallen_at_daa, redemption_deadline, 2, 5, 1,
        )),
        0,
    );

    let crumble_daa = creation_daa + 200_000;
    let mut dust_dna = fossil_dust.clone();
    dust_dna[36..40].copy_from_slice(&13u32.to_le_bytes());
    dust_dna[40] = MODE_DUST;
    dust_dna[41] = MODE_FOSSIL;
    let oh = sha256_bytes(&fossil_dust);
    dust_dna[493..525].copy_from_slice(&oh);
    dust_dna[525..529].copy_from_slice(&13u32.to_le_bytes());
    dust_dna[685..693].copy_from_slice(&0u64.to_le_bytes()); // fee_reserve -> 0
    dust_dna[709..717].copy_from_slice(&crumble_daa.to_le_bytes());
    dust_dna[717..721].copy_from_slice(&1u32.to_le_bytes());
    dust_dna[846..854].copy_from_slice(&crumble_daa.to_le_bytes()); // dusted_at_daa

    eprintln!("old: FOSSIL, fee_reserve={} (< min_output={})", old_fee_reserve, min_output);
    eprintln!("new: DUST, fee_reserve=0, dusted_at_daa={}", crumble_daa);
    let receipt = run_proof(&fossil_dust, &dust_dna, 13, crumble_daa, false).unwrap();
    eprintln!("PROOF VERIFIED LOCALLY. THE ORGANISM HAS CRUMBLED TO DUST. THIS IS THE END.");
    write_receipt(&receipt, &fossil_dust, &dust_dna, crumble_daa, "stage13_dust_crumble", &format!("{out_dir}/scorpion_stage13_dust_receipt.json"));

    eprintln!("\n═══ STAGE 13 (DUST) COMPLETE: negative test + real crumble proof, both verified. ═══");
}

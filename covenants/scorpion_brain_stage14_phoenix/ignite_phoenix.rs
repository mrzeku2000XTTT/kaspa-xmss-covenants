// ignite_phoenix.rs -- Stage 14: FLAME / PHOENIX ("the ash remembers, the flame forgives")
//
// Builds a MODE_DUST organism (fee_reserve=0, fossilized=1, a real grave record
// behind it), then proves IGNITE (transition=14): DUST -> STEM, same covenant_id
// (chitin), same owner_commitment, same eternal creation_daa/creation_txid --
// but every mode-specific chromosome resets to a fresh-birth state and the
// owner must resupply fee_reserve from absolute zero. This is the SAME
// organism crossing back from true death, not a new child (that's Chr6).
//
// Negative test: igniting with the WRONG owner_secret must be rejected -- only
// the original owner who held the secret through the whole fall->fossil->dust
// arc can strike the flame.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts, Receipt};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_STEM: u8 = 0;
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
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&0u64.to_le_bytes());
    d.extend_from_slice(&[0u8; 32]);
    d.extend_from_slice(&[0u8; 32]);
    d.push(0);
    assert_eq!(d.len(), 846);
    d
}
fn append_dust(mut d: Vec<u8>, dusted_at_daa: u64) -> Vec<u8> {
    d.extend_from_slice(&dusted_at_daa.to_le_bytes());
    assert_eq!(d.len(), 854);
    d
}

fn run_proof(old_dna: &[u8], new_dna: &[u8], transition: u8, current_daa: u64, owner_secret: &[u8], expect_fail: bool) -> Option<Receipt> {
    let mut b = ExecutorEnv::builder();
    b.write(&old_dna.to_vec()).unwrap();
    b.write(&new_dna.to_vec()).unwrap();
    b.write(&transition).unwrap();
    b.write(&current_daa).unwrap();
    b.write(&owner_secret.to_vec()).unwrap();
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
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());
    let cov_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());
    let original_creation_txid = hex_to_bytes(&"66".repeat(32));
    let original_creation_daa: u64 = 477_000_000; // the organism's true, eternal birth moment
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;
    let dusted_at_daa: u64 = 478_500_000;

    // Build the DUST record: a real grave, fully behind it (fell, redeemed some, fossilized, crumbled).
    let body = build_body(&covenant_id, 20, MODE_DUST, 7 /* prev_mode = FOSSIL */, &owner_commitment);
    let mem = append_memory(body, &[0x99u8; 32], 20, original_creation_daa, &original_creation_txid);
    let esc = append_escrow_zeroed(mem);
    let met = append_metabolism(esc, 2700, 0, min_output, rate_window, rate_max_spend, dusted_at_daa, 1);
    let fall = append_fall(met, FALL_REASON_STARVATION, dusted_at_daa - 100_000, dusted_at_daa - 50_000, 3, 5, 1);
    let hunt = append_hunt_zeroed(fall);
    let dust_dna = append_dust(hunt, dusted_at_daa);

    let ignite_daa = dusted_at_daa + 50_000;
    let resupply: u64 = 60_000_000;

    // ══════════════════════ NEGATIVE TEST: wrong owner_secret ══════════════════════
    eprintln!("── negative test: ignite attempt with the WRONG owner_secret ──");
    let wrong_secret = sha256_bytes(b"an_impostor_claiming_the_ashes");
    let mut bad_stem = dust_dna.clone();
    bad_stem[36..40].copy_from_slice(&21u32.to_le_bytes());
    bad_stem[40] = MODE_STEM;
    bad_stem[41] = MODE_DUST;
    let dh = sha256_bytes(&dust_dna);
    bad_stem[493..525].copy_from_slice(&dh);
    bad_stem[525..529].copy_from_slice(&21u32.to_le_bytes());
    bad_stem[685..693].copy_from_slice(&resupply.to_le_bytes());
    bad_stem[709..717].copy_from_slice(&ignite_daa.to_le_bytes());
    bad_stem[717..721].copy_from_slice(&1u32.to_le_bytes());
    for off in [721usize, 738, 739, 740] { bad_stem[off] = 0; }
    bad_stem[722..730].copy_from_slice(&0u64.to_le_bytes());
    bad_stem[730..738].copy_from_slice(&0u64.to_le_bytes());
    bad_stem[846..854].copy_from_slice(&0u64.to_le_bytes());
    run_proof(&dust_dna, &bad_stem, 14, ignite_daa, &wrong_secret, true);

    // ══════════════════════ REAL PROOF: the true owner ignites ══════════════════════
    eprintln!("── real proof: IGNITE -- the flame rises from the ash ──");
    let mut stem_dna = dust_dna.clone();
    stem_dna[36..40].copy_from_slice(&21u32.to_le_bytes()); // generation+1
    stem_dna[40] = MODE_STEM;
    stem_dna[41] = MODE_DUST;
    stem_dna[493..525].copy_from_slice(&dh); // prev_state_hash = sha256(own ashes)
    stem_dna[525..529].copy_from_slice(&21u32.to_le_bytes()); // transition_count+1
    // creation_daa/creation_txid untouched -- eternal, same as dust_dna's (original_creation_daa/txid)
    stem_dna[685..693].copy_from_slice(&resupply.to_le_bytes()); // fee_reserve resupplied
    stem_dna[709..717].copy_from_slice(&ignite_daa.to_le_bytes());
    stem_dna[717..721].copy_from_slice(&1u32.to_le_bytes());
    // Chr11 (fall/redemption) totally erased -- clean slate
    stem_dna[721] = 0;
    stem_dna[722..730].copy_from_slice(&0u64.to_le_bytes());
    stem_dna[730..738].copy_from_slice(&0u64.to_le_bytes());
    stem_dna[738] = 0;
    stem_dna[739] = 0;
    stem_dna[740] = 0;
    // Chr13 (dust) erased -- it is no longer dust
    stem_dna[846..854].copy_from_slice(&0u64.to_le_bytes());

    eprintln!("old: DUST, fee_reserve=0, covenant_id={}", hex::encode(&covenant_id));
    eprintln!("new: STEM, fee_reserve={} (resupplied), same covenant_id, generation {} -> 21", resupply, 20);
    let receipt = run_proof(&dust_dna, &stem_dna, 14, ignite_daa, &owner_secret, false).unwrap();
    eprintln!("PROOF VERIFIED LOCALLY. THE PHOENIX HAS RISEN. THE SAME CHITIN, REBORN.");
    write_receipt(&receipt, &dust_dna, &stem_dna, ignite_daa, "stage14_phoenix_ignite", &format!("{out_dir}/scorpion_stage14_phoenix_receipt.json"));

    eprintln!("\n═══ STAGE 14 (FLAME / PHOENIX) COMPLETE: negative test + real ignite proof, both verified. ═══");
}

// incubation.rs -- Stage 17: "111" (the angel number: new beginnings, alignment)
//
// Stage 16 proved the tomb must hold three days before IGNITE can fire. But once
// the flame rises, the organism as originally built could take its first step out
// of STEM (a MOULT into VAULT, or arming Sentinel, or anything else) the very
// next instant. That's not a newborn -- that's an adult wearing a phoenix costume.
//
// Stage 17 adds one new global invariant, reusing Stage 14's new `reborn_at_daa`
// field (set by IGNITE to the exact moment of rebirth): a phoenix must rest 111
// DAA -- literally the number itself, the angel number of alignment and new
// beginnings -- in the cradle before its first true departure from STEM. This is
// a MINIMUM, exactly like Stage 16's three-day tomb, but shorter and mirrored:
// that one gates the RISING, this one gates the first step AFTER rising.
//
// Two real local proofs, chained together:
//   0. IGNITE (transition 14): DUST -> STEM at the Stage 16 three-day boundary,
//      setting reborn_at_daa = ignite_daa. (Re-proves Stage 16 is still intact.)
//   1. NEGATIVE: MOULT (transition 0, STEM -> VAULT) attempted ONE DAA short of
//      111 DAA after rebirth -> REJECTED.
//   2. POSITIVE: the same MOULT attempted at EXACTLY 111 DAA after rebirth (the
//      boundary, >=) -> VERIFIED.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts, Receipt};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_STEM: u8 = 0;
const MODE_VAULT: u8 = 1;
const MODE_DUST: u8 = 9;
const FALL_REASON_STARVATION: u8 = 1;
const THREE_DAYS_DAA: u64 = 259_200;
const INCUBATION_DAA: u64 = 111;

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
fn append_incubation(mut d: Vec<u8>, reborn_at_daa: u64) -> Vec<u8> {
    d.extend_from_slice(&reborn_at_daa.to_le_bytes());
    assert_eq!(d.len(), 862);
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

// Builds the STEM record straight out of IGNITE -- gen+1, fee resupplied, all
// mode-specific chromosomes reset to birth defaults, reborn_at_daa = ignite_daa.
fn build_stem_from_dust(dust_dna: &[u8], ignite_daa: u64, resupply: u64, gen: u32) -> Vec<u8> {
    let mut stem = dust_dna.to_vec();
    stem[36..40].copy_from_slice(&gen.to_le_bytes());
    stem[40] = MODE_STEM;
    stem[41] = MODE_DUST;
    let dh = sha256_bytes(dust_dna);
    stem[493..525].copy_from_slice(&dh);
    stem[525..529].copy_from_slice(&gen.to_le_bytes());
    stem[685..693].copy_from_slice(&resupply.to_le_bytes());
    stem[709..717].copy_from_slice(&ignite_daa.to_le_bytes());
    stem[717..721].copy_from_slice(&1u32.to_le_bytes());
    stem[721] = 0;
    stem[722..730].copy_from_slice(&0u64.to_le_bytes());
    stem[730..738].copy_from_slice(&0u64.to_le_bytes());
    stem[738] = 0;
    stem[739] = 0;
    stem[740] = 0;
    stem[846..854].copy_from_slice(&0u64.to_le_bytes());
    stem[854..862].copy_from_slice(&ignite_daa.to_le_bytes()); // reborn_at_daa = the moment of rebirth
    stem
}

// Builds the post-MOULT record (STEM -> VAULT), the newborn's first true step.
fn build_vault_from_stem(stem_dna: &[u8], _moult_daa: u64, new_fee: u64) -> Vec<u8> {
    let mut vault = stem_dna.to_vec();
    let gen = u32::from_le_bytes(stem_dna[36..40].try_into().unwrap()) + 1;
    vault[36..40].copy_from_slice(&gen.to_le_bytes());
    vault[40] = MODE_VAULT;
    vault[41] = MODE_STEM;
    let sh = sha256_bytes(stem_dna);
    vault[493..525].copy_from_slice(&sh);
    let tc = u32::from_le_bytes(stem_dna[525..529].try_into().unwrap()) + 1;
    vault[525..529].copy_from_slice(&tc.to_le_bytes());
    vault[685..693].copy_from_slice(&new_fee.to_le_bytes());
    // window: current_daa - window_start_daa (110/111) < rate_window(1000) -> stays in window, count+1
    let wsc = u32::from_le_bytes(stem_dna[717..721].try_into().unwrap()) + 1;
    vault[717..721].copy_from_slice(&wsc.to_le_bytes());
    // reborn_at_daa (854..862) persists unchanged from stem_dna -- do not touch.
    vault
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
    let creation_txid = hex_to_bytes(&"88".repeat(32));
    let creation_daa: u64 = 479_000_000;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;
    let dusted_at_daa: u64 = 480_000_000;

    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 17: \"111\" -- THE NEWBORN INCUBATION GATE");
    eprintln!("  the angel number: new beginnings, alignment, made literal");
    eprintln!("═══════════════════════════════════════════════════════════════\n");

    // ── Step 0: IGNITE, DUST -> STEM, at the Stage 16 three-day boundary ──
    let body = build_body(&covenant_id, 20, MODE_DUST, 7, &owner_commitment);
    let mem = append_memory(body, &[0x77u8; 32], 20, creation_daa, &creation_txid);
    let esc = append_escrow_zeroed(mem);
    let met = append_metabolism(esc, 2700, 0, min_output, rate_window, rate_max_spend, dusted_at_daa, 1);
    let fall = append_fall(met, FALL_REASON_STARVATION, dusted_at_daa - 100_000, dusted_at_daa - 50_000, 3, 5, 1);
    let hunt = append_hunt_zeroed(fall);
    let dust = append_dust(hunt, dusted_at_daa);
    let dust_dna = append_incubation(dust, 0); // never risen before: reborn_at_daa = 0

    let resupply: u64 = 60_000_000;
    let ignite_daa = dusted_at_daa + THREE_DAYS_DAA;
    let stem_dna = build_stem_from_dust(&dust_dna, ignite_daa, resupply, 21);

    eprintln!("── [0/2] IGNITE: DUST -> STEM at exactly the three-day boundary ──");
    let ignite_receipt = run_proof(&dust_dna, &stem_dna, 14, ignite_daa, &owner_secret, false).unwrap();
    eprintln!("  VERIFIED. gen20 DUST -> gen21 STEM. reborn_at_daa recorded = {ignite_daa}.");
    write_receipt(&ignite_receipt, &dust_dna, &stem_dna, ignite_daa, "stage17_ignite", &format!("{out_dir}/scorpion_stage17_ignite_receipt.json"));

    // ═══════════ NEGATIVE: MOULT attempted ONE DAA short of 111 ═══════════
    eprintln!("\n── [1/2] NEGATIVE: attempting the first step out of STEM ONE DAA before 111 ──");
    let too_early_daa = ignite_daa + INCUBATION_DAA - 1;
    let early_vault = build_vault_from_stem(&stem_dna, too_early_daa, resupply - 5_000_000);
    eprintln!("  reborn_at_daa={ignite_daa}, attempt_daa={too_early_daa} (short by 1 DAA of {INCUBATION_DAA})");
    run_proof(&stem_dna, &early_vault, 0, too_early_daa, &owner_secret, true);

    // ═══════════ POSITIVE: MOULT at EXACTLY 111 DAA, the boundary itself ═══════════
    eprintln!("\n── [2/2] POSITIVE: the first step out of STEM at EXACTLY 111 DAA (the boundary, >=) ──");
    let on_time_daa = ignite_daa + INCUBATION_DAA;
    let vault_dna = build_vault_from_stem(&stem_dna, on_time_daa, resupply - 5_000_000);
    eprintln!("  reborn_at_daa={ignite_daa}, attempt_daa={on_time_daa} (exactly {INCUBATION_DAA} DAA later)");
    let receipt = run_proof(&stem_dna, &vault_dna, 0, on_time_daa, &owner_secret, false).unwrap();
    eprintln!("  VERIFIED. gen21 STEM -> gen22 VAULT. The newborn has taken its first true step.");
    write_receipt(&receipt, &stem_dna, &vault_dna, on_time_daa, "stage17_incubation", &format!("{out_dir}/scorpion_stage17_incubation_receipt.json"));

    eprintln!("\n═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 17 COMPLETE: 111 -- the angel number -- is now load-bearing.");
    eprintln!("  No phoenix, however genuine its rising, can leave the cradle early.");
    eprintln!("═══════════════════════════════════════════════════════════════");
}

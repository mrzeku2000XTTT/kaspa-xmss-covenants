// ignite_full_cycle.rs -- Stage 15: IGNITING
//
// The complete lifecycle in ONE unbroken chain of real STARK proofs.
// Not each transition in isolation -- the full circle, ashes to ashes
// to ashes to life, proving the phoenix is a real repeatable cycle:
//
//   1. HATCH      stem gen0  -> vault gen1   (birth)
//   2. MOULT      vault gen1 -> vault gen2   (life, heartbeat)
//   3. FALL       vault gen2 -> fallen gen3  (starvation)
//   4. FOSSILIZE  fallen g3  -> fossil gen4  (redemption window missed)
//   5. CRUMBLE    fossil g4  -> dust gen5    (ashes -- fee_reserve < min_output)
//   6. IGNITE     dust gen5  -> stem gen6    (THE PHOENIX RISES -- owner resupplies)
//   7. MOULT      stem gen6  -> vault gen7   (alive again -- the cycle continues)
//
// Every receipt feeds the next proof's old_dna. One chain, one identity,
// one organism crossing from birth through death and back.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts, Receipt};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_STEM: u8 = 0;
const MODE_VAULT: u8 = 1;
const MODE_FALLEN: u8 = 6;
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

// ── DNA construction helpers (same layout as all prior stages, 854 bytes) ──

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
    d.extend_from_slice(&[0u8; 32]); // arbiter_commit
    d.extend_from_slice(&[0u8; 32]); // buyer_commit
    d.extend_from_slice(&[0u8; 32]); // seller_commit
    d.extend_from_slice(&0u64.to_le_bytes()); // escrow_amount
    d.extend_from_slice(&0u64.to_le_bytes()); // dispute_deadline
    d.push(0); // winner
    d.push(0); // dispute_state
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

// ── Proof runner ──

fn run_proof(old_dna: &[u8], new_dna: &[u8], transition: u8, current_daa: u64, owner_secret: Option<&[u8]>) -> Receipt {
    run_proof_ex(old_dna, new_dna, transition, current_daa, owner_secret, None)
}

fn run_proof_ex(old_dna: &[u8], new_dna: &[u8], transition: u8, current_daa: u64, owner_secret: Option<&[u8]>, fall_reason: Option<u8>) -> Receipt {
    let mut b = ExecutorEnv::builder();
    b.write(&old_dna.to_vec()).unwrap();
    b.write(&new_dna.to_vec()).unwrap();
    b.write(&transition).unwrap();
    b.write(&current_daa).unwrap();
    if let Some(sec) = owner_secret {
        b.write(&sec.to_vec()).unwrap();
    }
    if let Some(r) = fall_reason {
        b.write(&r).unwrap();
    }
    let env = b.build().unwrap();
    let prover = default_prover();
    let prove_info = prover.prove_with_opts(env, SCORPION_BRAIN_GUEST_ELF, &ProverOpts::succinct())
        .unwrap_or_else(|e| panic!("proving failed at transition {}: {e}", transition));
    let receipt = prove_info.receipt;
    receipt.verify(SCORPION_BRAIN_GUEST_ID).expect("local verification failed");
    receipt
}

// ── Build a full DNA from scratch with specific mode + fall fields ──

#[allow(clippy::too_many_arguments)]
fn make_dna(
    covenant_id: &[u8], generation: u32, mode: u8, prev_mode: u8,
    owner_commitment: &[u8], prev_state_hash: &[u8], transition_count: u32,
    creation_daa: u64, creation_txid: &[u8],
    compute_budget: u16, fee_reserve: u64, min_output: u64,
    rate_window: u32, rate_max_spend: u32, window_start_daa: u64, window_spend_count: u32,
    fall_reason: u8, fallen_at_daa: u64, redemption_deadline: u64,
    redemption_progress: u8, redemption_threshold: u8, fossilized: u8,
    dusted_at_daa: u64,
) -> Vec<u8> {
    let body = build_body(covenant_id, generation, mode, prev_mode, owner_commitment);
    let mem = append_memory(body, prev_state_hash, transition_count, creation_daa, creation_txid);
    let esc = append_escrow_zeroed(mem);
    let met = append_metabolism(esc, compute_budget, fee_reserve, min_output, rate_window, rate_max_spend, window_start_daa, window_spend_count);
    let fall = append_fall(met, fall_reason, fallen_at_daa, redemption_deadline, redemption_progress, redemption_threshold, fossilized);
    let hunt = append_hunt_zeroed(fall);
    append_dust(hunt, dusted_at_daa)
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
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env()).init();
    let args: Vec<String> = std_env::args().collect();
    let mut out_dir = "/app/kaspa_wrpc/keys".to_string();
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--out-dir=") { out_dir = v.to_string(); }
    }

    // ── Load identity ──
    let owner_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap()).unwrap();
    let owner_commitment = hex_to_bytes(owner_json["owner_commitment_hex"].as_str().unwrap());
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());
    let cov_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap()).unwrap();
    let covenant_id = hex_to_bytes(cov_json["covenant_id_hex"].as_str().unwrap());
    let creation_txid = hex_to_bytes(&"77".repeat(32));
    let creation_daa: u64 = 478_000_000;

    // Constants
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;
    let cb: u16 = 2700;

    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 15: IGNITING -- the full lifecycle, one unbroken chain");
    eprintln!("  Birth -> Life -> Fall -> Fossil -> Dust -> IGNITE -> Life again");
    eprintln!("═══════════════════════════════════════════════════════════════\n");

    // ═══════════════ 1. HATCH: stem gen0 -> vault gen1 ═══════════════
    eprintln!("── [1/7] HATCH: stem gen0 -> vault gen1 (birth) ──");
    let daa0 = creation_daa;
    let stem_gen0 = make_dna(
        &covenant_id, 0, MODE_STEM, 0, &owner_commitment,
        &[0u8; 32], 0, creation_daa, &creation_txid,
        cb, 60_000_000, min_output, rate_window, rate_max_spend, daa0, 0,
        0, 0, 0, 0, 5, 0, 0,
    );
    let mut vault_gen1 = stem_gen0.clone();
    vault_gen1[36..40].copy_from_slice(&1u32.to_le_bytes());
    vault_gen1[40] = MODE_VAULT;
    vault_gen1[41] = MODE_STEM;
    let h0 = sha256_bytes(&stem_gen0);
    vault_gen1[493..525].copy_from_slice(&h0);
    vault_gen1[525..529].copy_from_slice(&1u32.to_le_bytes());
    vault_gen1[685..693].copy_from_slice(&50_000_000u64.to_le_bytes()); // fee burns 60M -> 50M
    vault_gen1[709..717].copy_from_slice(&daa0.to_le_bytes());
    vault_gen1[717..721].copy_from_slice(&1u32.to_le_bytes());
    let _r1 = run_proof(&stem_gen0, &vault_gen1, 0, daa0, Some(&owner_secret));
    eprintln!("  VERIFIED. gen0 STEM -> gen1 VAULT. fee 60M -> 50M.");

    // ═══════════════ 2. MOULT: vault gen1 -> vault gen2 ═══════════════
    eprintln!("── [2/7] MOULT: vault gen1 -> vault gen2 (heartbeat) ──");
    let daa1 = daa0 + 10_000;
    let mut vault_gen2 = vault_gen1.clone();
    vault_gen2[36..40].copy_from_slice(&2u32.to_le_bytes());
    vault_gen2[40] = MODE_VAULT;
    vault_gen2[41] = MODE_VAULT;
    let h1 = sha256_bytes(&vault_gen1);
    vault_gen2[493..525].copy_from_slice(&h1);
    vault_gen2[525..529].copy_from_slice(&2u32.to_le_bytes());
    vault_gen2[685..693].copy_from_slice(&45_000_000u64.to_le_bytes()); // 50M -> 45M
    vault_gen2[709..717].copy_from_slice(&daa1.to_le_bytes());
    vault_gen2[717..721].copy_from_slice(&1u32.to_le_bytes()); // window reset (delta > rate_window) -> fresh window starts at 1
    let _r2 = run_proof(&vault_gen1, &vault_gen2, 0, daa1, Some(&owner_secret));
    eprintln!("  VERIFIED. gen1 VAULT -> gen2 VAULT. fee 50M -> 45M.");

    // ═══════════════ 3. FALL: vault gen2 -> fallen gen3 ═══════════════
    eprintln!("── [3/7] FALL: vault gen2 -> fallen gen3 (starvation) ──");
    let daa2 = daa1 + 10_000;
    // First starve it: set fee_reserve to 15M (< min_output 20M) in gen2's state
    // Actually we need the OLD state to have fee_reserve < min_output for FALL to trigger.
    // Let's make vault_gen2 already starving (fee_reserve was 45M, too high).
    // We'll adjust: the fall proof checks old_dna.fee_reserve < old_dna.min_output.
    // So we need vault_gen2 to have fee_reserve < 20M. Let's reconstruct with 15M.
    let vault_gen2_starved = make_dna(
        &covenant_id, 2, MODE_VAULT, MODE_VAULT, &owner_commitment,
        &h1, 2, creation_daa, &creation_txid,
        cb, 15_000_000, min_output, rate_window, rate_max_spend, daa1, 2,
        0, 0, 0, 0, 5, 0, 0,
    );
    let mut fallen_gen3 = vault_gen2_starved.clone();
    fallen_gen3[36..40].copy_from_slice(&3u32.to_le_bytes());
    fallen_gen3[40] = MODE_FALLEN;
    fallen_gen3[41] = MODE_VAULT;
    let h2 = sha256_bytes(&vault_gen2_starved);
    fallen_gen3[493..525].copy_from_slice(&h2);
    fallen_gen3[525..529].copy_from_slice(&3u32.to_le_bytes());
    fallen_gen3[685..693].copy_from_slice(&10_000_000u64.to_le_bytes()); // 15M -> 10M
    fallen_gen3[709..717].copy_from_slice(&daa2.to_le_bytes());
    fallen_gen3[717..721].copy_from_slice(&1u32.to_le_bytes());
    fallen_gen3[721] = FALL_REASON_STARVATION;
    fallen_gen3[722..730].copy_from_slice(&daa2.to_le_bytes()); // fallen_at_daa
    let redemption_deadline = daa2 + 100_000;
    fallen_gen3[730..738].copy_from_slice(&redemption_deadline.to_le_bytes());
    // fossilized stays 0, redemption_progress stays 0
    let _r3 = run_proof_ex(&vault_gen2_starved, &fallen_gen3, 7, daa2, None, Some(FALL_REASON_STARVATION));
    eprintln!("  VERIFIED. gen2 VAULT (starved, 15M) -> gen3 FALLEN. fee 15M -> 10M.");

    // ═══════════════ 4. FOSSILIZE: fallen gen3 -> fossil gen4 ═══════════════
    eprintln!("── [4/7] FOSSILIZE: fallen gen3 -> fossil gen4 (redemption window missed) ──");
    let daa3 = redemption_deadline + 1; // deadline has passed
    let mut fossil_gen4 = fallen_gen3.clone();
    fossil_gen4[36..40].copy_from_slice(&4u32.to_le_bytes());
    fossil_gen4[40] = MODE_FOSSIL;
    fossil_gen4[41] = MODE_FALLEN;
    let h3 = sha256_bytes(&fallen_gen3);
    fossil_gen4[493..525].copy_from_slice(&h3);
    fossil_gen4[525..529].copy_from_slice(&4u32.to_le_bytes());
    fossil_gen4[685..693].copy_from_slice(&5_000_000u64.to_le_bytes()); // 10M -> 5M
    fossil_gen4[709..717].copy_from_slice(&daa3.to_le_bytes());
    fossil_gen4[717..721].copy_from_slice(&1u32.to_le_bytes());
    fossil_gen4[740] = 1; // fossilized = 1
    let _r4 = run_proof(&fallen_gen3, &fossil_gen4, 10, daa3, None);
    eprintln!("  VERIFIED. gen3 FALLEN -> gen4 FOSSIL. fee 10M -> 5M. fossilized=1.");

    // ═══════════════ 5. CRUMBLE: fossil gen4 -> dust gen5 ═══════════════
    eprintln!("── [5/7] CRUMBLE: fossil gen4 -> dust gen5 (ashes) ──");
    // fee_reserve is 5M, well below min_output 20M -- true dust
    let daa4 = daa3 + 10_000;
    let mut dust_gen5 = fossil_gen4.clone();
    dust_gen5[36..40].copy_from_slice(&5u32.to_le_bytes());
    dust_gen5[40] = MODE_DUST;
    dust_gen5[41] = MODE_FOSSIL;
    let h4 = sha256_bytes(&fossil_gen4);
    dust_gen5[493..525].copy_from_slice(&h4);
    dust_gen5[525..529].copy_from_slice(&5u32.to_le_bytes());
    dust_gen5[685..693].copy_from_slice(&0u64.to_le_bytes()); // fee_reserve -> 0
    dust_gen5[709..717].copy_from_slice(&daa4.to_le_bytes());
    dust_gen5[717..721].copy_from_slice(&1u32.to_le_bytes());
    dust_gen5[846..854].copy_from_slice(&daa4.to_le_bytes()); // dusted_at_daa
    let _r5 = run_proof(&fossil_gen4, &dust_gen5, 13, daa4, None);
    eprintln!("  VERIFIED. gen4 FOSSIL -> gen5 DUST. fee 5M -> 0. THE ASHES.");

    // ═══════════════ 6. IGNITE: dust gen5 -> stem gen6 (THE PHOENIX) ═══════════════
    eprintln!("── [6/7] IGNITE: dust gen5 -> stem gen6 (THE PHOENIX RISES) ──");
    let daa5 = daa4 + 50_000;
    let resupply: u64 = 60_000_000;
    let mut stem_gen6 = dust_gen5.clone();
    stem_gen6[36..40].copy_from_slice(&6u32.to_le_bytes());
    stem_gen6[40] = MODE_STEM;
    stem_gen6[41] = MODE_DUST;
    let h5 = sha256_bytes(&dust_gen5);
    stem_gen6[493..525].copy_from_slice(&h5); // prev_state_hash = sha256(ashes)
    stem_gen6[525..529].copy_from_slice(&6u32.to_le_bytes());
    // creation_daa/creation_txid UNCHANGED -- eternal, same as dust_gen5's
    stem_gen6[685..693].copy_from_slice(&resupply.to_le_bytes()); // fee resupplied from 0
    stem_gen6[709..717].copy_from_slice(&daa5.to_le_bytes());
    stem_gen6[717..721].copy_from_slice(&1u32.to_le_bytes());
    // Erase ALL mode-specific state -- clean slate
    stem_gen6[721] = 0; // fall_reason
    stem_gen6[722..730].copy_from_slice(&0u64.to_le_bytes()); // fallen_at_daa
    stem_gen6[730..738].copy_from_slice(&0u64.to_le_bytes()); // redemption_deadline
    stem_gen6[738] = 0; // redemption_progress
    stem_gen6[739] = 0; // redemption_threshold (stays 5 in old, reset to 0)
    stem_gen6[740] = 0; // fossilized
    stem_gen6[846..854].copy_from_slice(&0u64.to_le_bytes()); // dusted_at_daa
    let r6 = run_proof(&dust_gen5, &stem_gen6, 14, daa5, Some(&owner_secret));
    eprintln!("  VERIFIED. gen5 DUST -> gen6 STEM. fee 0 -> {} (resupplied). SAME CHITIN.", resupply);
    write_receipt(&r6, &dust_gen5, &stem_gen6, daa5, "stage15_ignite", &format!("{out_dir}/scorpion_stage15_ignite_receipt.json"));

    // ═══════════════ 7. MOULT: stem gen6 -> vault gen7 (alive again!) ═══════════════
    eprintln!("── [7/7] MOULT: stem gen6 -> vault gen7 (the cycle continues) ──");
    let daa6 = daa5 + 10_000;
    let mut vault_gen7 = stem_gen6.clone();
    vault_gen7[36..40].copy_from_slice(&7u32.to_le_bytes());
    vault_gen7[40] = MODE_VAULT;
    vault_gen7[41] = MODE_STEM;
    let h6 = sha256_bytes(&stem_gen6);
    vault_gen7[493..525].copy_from_slice(&h6);
    vault_gen7[525..529].copy_from_slice(&7u32.to_le_bytes());
    vault_gen7[685..693].copy_from_slice(&50_000_000u64.to_le_bytes()); // 60M -> 50M
    vault_gen7[709..717].copy_from_slice(&daa6.to_le_bytes());
    vault_gen7[717..721].copy_from_slice(&1u32.to_le_bytes());
    let _r7 = run_proof(&stem_gen6, &vault_gen7, 0, daa6, Some(&owner_secret));
    eprintln!("  VERIFIED. gen6 STEM -> gen7 VAULT. fee 60M -> 50M. ALIVE AGAIN.");

    eprintln!("\n═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 15 COMPLETE: 7/7 proofs verified. THE FULL CYCLE IS REAL.");
    eprintln!("  Birth -> Life -> Fall -> Fossil -> Dust -> Ignite -> Life again");
    eprintln!("  Same covenant_id through all 7 transitions. Same owner.");
    eprintln!("  The scorpion died and rose from its own ashes. 400 million years.");
    eprintln!("═══════════════════════════════════════════════════════════════");
}

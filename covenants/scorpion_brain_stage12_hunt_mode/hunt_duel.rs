// hunt_duel.rs -- Stage 12: HUNT MODE ("you wanna be a hunter, or hunted")
//
// Builds a fresh VAULT organism, proves INITIATE_HUNT (transition=11, wager
// staked from fee_reserve), then proves RESOLVE_HUNT (transition=12) for two
// real outcomes -- a clean kill (hunter wins) and an ambush (hunter loses) --
// plus a negative test: resolving with a revealed move that doesn't match its
// sealed commitment must be rejected.

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts, Receipt};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_STEM: u8 = 0;
const MODE_VAULT: u8 = 1;
const MODE_HUNT: u8 = 8;

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
    d.extend_from_slice(&0u64.to_le_bytes()); // dispute_deadline
    d.extend_from_slice(&0u64.to_le_bytes()); // escrow_amount
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
fn append_fall_zeroed(mut d: Vec<u8>) -> Vec<u8> {
    d.push(0); // fall_reason
    d.extend_from_slice(&0u64.to_le_bytes()); // fallen_at_daa
    d.extend_from_slice(&0u64.to_le_bytes()); // redemption_deadline
    d.push(0); // redemption_progress
    d.push(0); // redemption_threshold
    d.push(0); // fossilized
    assert_eq!(d.len(), 741);
    d
}
fn append_hunt(mut d: Vec<u8>, hunt_target_id: &[u8], hunt_wager: u64, hunter_move_commit: &[u8], prey_move_commit: &[u8], hunt_state: u8) -> Vec<u8> {
    d.extend_from_slice(hunt_target_id);
    d.extend_from_slice(&hunt_wager.to_le_bytes());
    d.extend_from_slice(hunter_move_commit);
    d.extend_from_slice(prey_move_commit);
    d.push(hunt_state);
    assert_eq!(d.len(), 846);
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

fn run_proof(old_dna: &[u8], new_dna: &[u8], transition: u8, current_daa: u64, extra: &[Vec<u8>], owner_secret: Option<&[u8]>, expect_fail: bool) -> Option<Receipt> {
    let mut b = ExecutorEnv::builder();
    b.write(&old_dna.to_vec()).unwrap();
    b.write(&new_dna.to_vec()).unwrap();
    b.write(&transition).unwrap();
    b.write(&current_daa).unwrap();
    if let Some(s) = owner_secret { b.write(&s.to_vec()).unwrap(); }
    for e in extra {
        // moves are single bytes, nonces are 32-byte vecs -- write as raw bytes matching guest's env::read::<u8>/Vec<u8>
        if e.len() == 1 {
            b.write(&e[0]).unwrap();
        } else {
            b.write(e).unwrap();
        }
    }
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

#[allow(clippy::too_many_arguments)]
fn fresh_vault(covenant_id: &[u8], owner_commitment: &[u8], creation_txid: &[u8], creation_daa: u64, fee_reserve: u64, min_output: u64, rate_window: u32, rate_max_spend: u32) -> Vec<u8> {
    let body = build_body(covenant_id, 1, MODE_VAULT, MODE_STEM, owner_commitment);
    let mem = append_memory(body, &[0x00u8; 32], 1, creation_daa, creation_txid);
    let esc = append_escrow_zeroed(mem);
    let met = append_metabolism(esc, 2700, fee_reserve, min_output, rate_window, rate_max_spend, creation_daa, 1);
    let fall = append_fall_zeroed(met);
    append_hunt(fall, &[0u8; 32], 0, &[0u8; 32], &[0u8; 32], 0)
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
    let creation_txid = hex_to_bytes(&"44".repeat(32));
    let creation_daa: u64 = 477_800_000;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;

    let prey_target_id = sha256_bytes(b"prey_covenant_id_placeholder");
    let wager: u64 = 20_000_000;

    // ══════════════════════════════ DUEL 1: CLEAN KILL (hunter wins) ══════════════════════════════
    eprintln!("═══ DUEL 1: hunter HUNTs, prey FLEEs -- clean kill ═══");
    let vault1 = fresh_vault(&covenant_id, &owner_commitment, &creation_txid, creation_daa, 100_000_000, min_output, rate_window, rate_max_spend);

    let hunter_nonce1 = sha256_bytes(b"hunter_nonce_duel1");
    let prey_nonce1 = sha256_bytes(b"prey_nonce_duel1");
    let mut hbuf = vec![1u8]; hbuf.extend_from_slice(&hunter_nonce1);
    let hunter_commit1 = sha256_bytes(&hbuf);
    let mut pbuf = vec![2u8]; pbuf.extend_from_slice(&prey_nonce1);
    let prey_commit1 = sha256_bytes(&pbuf);

    let fee_after_initiate1 = 100_000_000u64 - wager;
    let mut hunt_active1 = vault1.clone();
    hunt_active1[36..40].copy_from_slice(&2u32.to_le_bytes());
    hunt_active1[40] = MODE_HUNT;
    hunt_active1[41] = MODE_VAULT;
    let vh1 = sha256_bytes(&vault1);
    hunt_active1[493..525].copy_from_slice(&vh1);
    hunt_active1[525..529].copy_from_slice(&2u32.to_le_bytes());
    hunt_active1[685..693].copy_from_slice(&fee_after_initiate1.to_le_bytes());
    apply_window(&mut hunt_active1, creation_daa, 1, rate_window as u64, creation_daa + 100);
    hunt_active1[741..773].copy_from_slice(&prey_target_id);
    hunt_active1[773..781].copy_from_slice(&wager.to_le_bytes());
    hunt_active1[781..813].copy_from_slice(&hunter_commit1);
    hunt_active1[813..845].copy_from_slice(&prey_commit1);
    hunt_active1[845] = 1;

    eprintln!("── INITIATE_HUNT (duel 1) ──");
    run_proof(&vault1, &hunt_active1, 11, creation_daa + 100, &[], Some(&owner_secret), false);
    eprintln!("  verified. fee_reserve {} -> {} (wager {} staked)", 100_000_000u64, fee_after_initiate1, wager);

    let expected_fee1 = fee_after_initiate1 + wager * 2;
    let mut vault_after1 = hunt_active1.clone();
    vault_after1[36..40].copy_from_slice(&3u32.to_le_bytes());
    vault_after1[40] = MODE_VAULT;
    vault_after1[41] = MODE_HUNT;
    let hh1 = sha256_bytes(&hunt_active1);
    vault_after1[493..525].copy_from_slice(&hh1);
    vault_after1[525..529].copy_from_slice(&3u32.to_le_bytes());
    vault_after1[685..693].copy_from_slice(&expected_fee1.to_le_bytes());
    apply_window(&mut vault_after1, creation_daa, 2, rate_window as u64, creation_daa + 200);
    vault_after1[741..773].copy_from_slice(&[0u8; 32]);
    vault_after1[773..781].copy_from_slice(&0u64.to_le_bytes());
    vault_after1[781..813].copy_from_slice(&[0u8; 32]);
    vault_after1[813..845].copy_from_slice(&[0u8; 32]);
    vault_after1[845] = 0;

    // ── Negative test: resolve with a move that doesn't match its own sealed commitment ──
    eprintln!("── negative test: resolve_hunt with a forged hunter_move (doesn't match commitment) ──");
    run_proof(
        &hunt_active1, &vault_after1, 12, creation_daa + 200,
        &[vec![2u8], hunter_nonce1.to_vec(), vec![2u8], prey_nonce1.to_vec()],
        None, true,
    );

    eprintln!("── RESOLVE_HUNT (duel 1, real): HUNT vs FLEE -- clean kill ──");
    let receipt1 = run_proof(
        &hunt_active1, &vault_after1, 12, creation_daa + 200,
        &[vec![1u8], hunter_nonce1.to_vec(), vec![2u8], prey_nonce1.to_vec()],
        None, false,
    ).unwrap();
    eprintln!("  PROOF VERIFIED. fee_reserve {} -> {}. THE HUNTER FEASTS.", fee_after_initiate1, expected_fee1);
    write_receipt(&receipt1, &hunt_active1, &vault_after1, creation_daa + 200, "stage12_hunt_win", &format!("{out_dir}/scorpion_stage12_hunt_win_receipt.json"));

    // ══════════════════════════════ DUEL 2: AMBUSHED (hunter loses) ══════════════════════════════
    eprintln!("═══ DUEL 2: hunter FLEEs, prey HUNTs -- ambushed ═══");
    let vault2 = fresh_vault(&covenant_id, &owner_commitment, &creation_txid, creation_daa + 10_000, 80_000_000, min_output, rate_window, rate_max_spend);

    let hunter_nonce2 = sha256_bytes(b"hunter_nonce_duel2");
    let prey_nonce2 = sha256_bytes(b"prey_nonce_duel2");
    let mut hbuf2 = vec![2u8]; hbuf2.extend_from_slice(&hunter_nonce2);
    let hunter_commit2 = sha256_bytes(&hbuf2);
    let mut pbuf2 = vec![1u8]; pbuf2.extend_from_slice(&prey_nonce2);
    let prey_commit2 = sha256_bytes(&pbuf2);

    let fee_after_initiate2 = 80_000_000u64 - wager;
    let mut hunt_active2 = vault2.clone();
    hunt_active2[36..40].copy_from_slice(&2u32.to_le_bytes());
    hunt_active2[40] = MODE_HUNT;
    hunt_active2[41] = MODE_VAULT;
    let vh2 = sha256_bytes(&vault2);
    hunt_active2[493..525].copy_from_slice(&vh2);
    hunt_active2[525..529].copy_from_slice(&2u32.to_le_bytes());
    hunt_active2[685..693].copy_from_slice(&fee_after_initiate2.to_le_bytes());
    apply_window(&mut hunt_active2, creation_daa + 10_000, 1, rate_window as u64, creation_daa + 10_100);
    hunt_active2[741..773].copy_from_slice(&prey_target_id);
    hunt_active2[773..781].copy_from_slice(&wager.to_le_bytes());
    hunt_active2[781..813].copy_from_slice(&hunter_commit2);
    hunt_active2[813..845].copy_from_slice(&prey_commit2);
    hunt_active2[845] = 1;

    eprintln!("── INITIATE_HUNT (duel 2) ──");
    run_proof(&vault2, &hunt_active2, 11, creation_daa + 10_100, &[], Some(&owner_secret), false);
    eprintln!("  verified. fee_reserve {} -> {} (wager {} staked)", 80_000_000u64, fee_after_initiate2, wager);

    let expected_fee2 = fee_after_initiate2; // FLEE/HUNT -- hunter recovers nothing
    let mut vault_after2 = hunt_active2.clone();
    vault_after2[36..40].copy_from_slice(&3u32.to_le_bytes());
    vault_after2[40] = MODE_VAULT;
    vault_after2[41] = MODE_HUNT;
    let hh2 = sha256_bytes(&hunt_active2);
    vault_after2[493..525].copy_from_slice(&hh2);
    vault_after2[525..529].copy_from_slice(&3u32.to_le_bytes());
    vault_after2[685..693].copy_from_slice(&expected_fee2.to_le_bytes());
    apply_window(&mut vault_after2, creation_daa + 10_000, 2, rate_window as u64, creation_daa + 10_200);
    vault_after2[741..773].copy_from_slice(&[0u8; 32]);
    vault_after2[773..781].copy_from_slice(&0u64.to_le_bytes());
    vault_after2[781..813].copy_from_slice(&[0u8; 32]);
    vault_after2[813..845].copy_from_slice(&[0u8; 32]);
    vault_after2[845] = 0;

    eprintln!("── RESOLVE_HUNT (duel 2, real): FLEE vs HUNT -- ambushed, wager gone ──");
    let receipt2 = run_proof(
        &hunt_active2, &vault_after2, 12, creation_daa + 10_200,
        &[vec![2u8], hunter_nonce2.to_vec(), vec![1u8], prey_nonce2.to_vec()],
        None, false,
    ).unwrap();
    eprintln!("  PROOF VERIFIED. fee_reserve {} -> {} (unchanged -- no refund). THE HUNTER BECAME THE HUNTED.", fee_after_initiate2, expected_fee2);
    write_receipt(&receipt2, &hunt_active2, &vault_after2, creation_daa + 10_200, "stage12_hunt_lose", &format!("{out_dir}/scorpion_stage12_hunt_lose_receipt.json"));

    eprintln!("\n═══ STAGE 12 (HUNT MODE) COMPLETE: initiate + win + lose + forged-move rejection, all proven. ═══");
}

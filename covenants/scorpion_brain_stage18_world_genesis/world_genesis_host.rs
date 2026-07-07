// world_genesis.rs -- Stage 18: "A SCORPION WHO GENERATES WORLDS"
//
// Reproduction (Stage 6/7, transition 5) spawns more scorpions: same DNA shape, same
// rules, a new instance of the same species. Stage 18 is categorically different --
// the scorpion authorizes a brand-new covenant genesis output that does NOT have to
// be a scorpion at all. Any redeem-script template, any rule set, any species: a
// zktimelock, a sentinel, a zkgate, another organism entirely, or something that has
// never existed before. The scorpion doesn't reproduce itself here -- it seeds an
// entire independent world and lets it go free.
//
// What stays permanently, provably verifiable is PROVENANCE: every world this scorpion
// has ever seeded is linked into one forward hash chain (`world_chain_hash`), the same
// memory-chain technique as Chromosome 7, but for children of a different kind. It is
// append-only, extended by exactly one transition (15, GENERATE_WORLD), and it never
// resets -- not even through death and rebirth (Chromosome 14/17). The scorpion's
// memory of what it created is eternal, even if the scorpion itself is not.
//
// Four real local proofs, chained together:
//   0. HATCH  (transition 0):  STEM  -> VAULT   (birth, max_worlds=2 set at genesis)
//   1. GENERATE_WORLD #1 (transition 15): VAULT -> VAULT, worlds_spawned 0 -> 1
//   2. GENERATE_WORLD #2 (transition 15): VAULT -> VAULT, worlds_spawned 1 -> 2 (== max_worlds, the cap boundary)
//   3. NEGATIVE: GENERATE_WORLD #3 attempted -> worlds_spawned would be 3 > max_worlds=2 -> REJECTED

use methods::{SCORPION_BRAIN_GUEST_ELF, SCORPION_BRAIN_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts, Receipt};
use serde_json::Value;
use std::env as std_env;
use std::fs;

const MAGIC: &[u8; 4] = b"GPDL";
const MODE_STEM: u8 = 0;
const MODE_VAULT: u8 = 1;

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
fn append_fall_zeroed(mut d: Vec<u8>) -> Vec<u8> {
    d.push(0);
    d.extend_from_slice(&0u64.to_le_bytes());
    d.extend_from_slice(&0u64.to_le_bytes());
    d.push(0);
    d.push(0);
    d.push(0);
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
fn append_world_genesis(mut d: Vec<u8>, world_chain_hash: &[u8], worlds_spawned: u32, max_worlds: u32) -> Vec<u8> {
    d.extend_from_slice(world_chain_hash);
    d.extend_from_slice(&worlds_spawned.to_le_bytes());
    d.extend_from_slice(&max_worlds.to_le_bytes());
    assert_eq!(d.len(), 902);
    d
}

fn run_proof(old_dna: &[u8], new_dna: &[u8], transition: u8, current_daa: u64, extra_witness: Vec<Vec<u8>>, expect_fail: bool) -> Option<Receipt> {
    let mut b = ExecutorEnv::builder();
    b.write(&old_dna.to_vec()).unwrap();
    b.write(&new_dna.to_vec()).unwrap();
    b.write(&transition).unwrap();
    b.write(&current_daa).unwrap();
    for w in &extra_witness {
        b.write(w).unwrap();
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

fn build_vault_from_stem(stem_dna: &[u8], new_fee: u64) -> Vec<u8> {
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
    let wsc = u32::from_le_bytes(stem_dna[717..721].try_into().unwrap()) + 1;
    vault[717..721].copy_from_slice(&wsc.to_le_bytes());
    vault
}

// Builds the post-GENERATE_WORLD record (VAULT -> VAULT), extending world_chain_hash.
fn build_vault_after_world(prev_dna: &[u8], new_fee: u64, world_covenant_id: &[u8; 32], world_script_hash: &[u8; 32]) -> Vec<u8> {
    let mut next = prev_dna.to_vec();
    let gen = u32::from_le_bytes(prev_dna[36..40].try_into().unwrap()) + 1;
    next[36..40].copy_from_slice(&gen.to_le_bytes());
    next[40] = MODE_VAULT;
    next[41] = MODE_VAULT;
    let ph = sha256_bytes(prev_dna);
    next[493..525].copy_from_slice(&ph);
    let tc = u32::from_le_bytes(prev_dna[525..529].try_into().unwrap()) + 1;
    next[525..529].copy_from_slice(&tc.to_le_bytes());
    next[685..693].copy_from_slice(&new_fee.to_le_bytes());
    let wsc = u32::from_le_bytes(prev_dna[717..721].try_into().unwrap()) + 1;
    next[717..721].copy_from_slice(&wsc.to_le_bytes());

    // Chromosome 18 fields (offsets 862..902):
    let old_chain_hash = &prev_dna[862..894];
    let mut buf = Vec::with_capacity(96);
    buf.extend_from_slice(old_chain_hash);
    buf.extend_from_slice(world_covenant_id);
    buf.extend_from_slice(world_script_hash);
    let new_chain_hash = sha256_bytes(&buf);
    next[862..894].copy_from_slice(&new_chain_hash);

    let worlds_spawned = u32::from_le_bytes(prev_dna[894..898].try_into().unwrap()) + 1;
    next[894..898].copy_from_slice(&worlds_spawned.to_le_bytes());
    // max_worlds (898..902) unchanged
    next
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
    let creation_txid = hex_to_bytes(&"99".repeat(32));
    let creation_daa: u64 = 480_500_000;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 1000;
    let rate_max_spend: u32 = 5;
    let hatch_daa = creation_daa;

    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 18: \"A SCORPION WHO GENERATES WORLDS\"");
    eprintln!("  world genesis -- Chromosome 18");
    eprintln!("═══════════════════════════════════════════════════════════════\n");

    // gen0: STEM, born with max_worlds = 2 already set (birth-time constant)
    let max_worlds: u32 = 2;
    let body0 = build_body(&covenant_id, 0, MODE_STEM, MODE_STEM, &owner_commitment);
    let mem0 = append_memory(body0, &[0u8; 32], 0, creation_daa, &creation_txid);
    let esc0 = append_escrow_zeroed(mem0);
    let met0 = append_metabolism(esc0, 2700, 100_000_000, min_output, rate_window, rate_max_spend, hatch_daa, 0);
    let fall0 = append_fall_zeroed(met0);
    let hunt0 = append_hunt_zeroed(fall0);
    let dust0 = append_dust(hunt0, 0);
    let incub0 = append_incubation(dust0, 0);
    let stem_dna = append_world_genesis(incub0, &[0u8; 32], 0, max_worlds);

    eprintln!("── [0/3] HATCH: STEM -> VAULT (birth, max_worlds={max_worlds}) ──");
    let vault1 = build_vault_from_stem(&stem_dna, 90_000_000);
    let hatch_receipt = run_proof(&stem_dna, &vault1, 0, hatch_daa, vec![owner_secret.clone()], false).unwrap();
    eprintln!("  VERIFIED. gen0 STEM -> gen1 VAULT.");
    write_receipt(&hatch_receipt, &stem_dna, &vault1, hatch_daa, "stage18_hatch", &format!("{out_dir}/scorpion_stage18_hatch_receipt.json"));

    // ── [1/3] GENERATE_WORLD #1: seed a zktimelock-shaped world ──
    let world1_covenant_id: [u8; 32] = sha256_bytes(b"world-1: zktimelock, customer deploy, gen0 seed");
    let world1_script_hash: [u8; 32] = sha256_bytes(b"world-1: redeem_script bytes (opaque to the scorpion)");
    let vault2 = build_vault_after_world(&vault1, 85_000_000, &world1_covenant_id, &world1_script_hash);
    eprintln!("\n── [1/3] GENERATE_WORLD #1: seed a brand-new, unrelated covenant ──");
    let w1_daa = hatch_daa + 100;
    let w1_receipt = run_proof(
        &vault1, &vault2, 15, w1_daa,
        vec![world1_covenant_id.to_vec(), world1_script_hash.to_vec()],
        false,
    ).unwrap();
    eprintln!("  VERIFIED. gen1 VAULT -> gen2 VAULT. worlds_spawned 0 -> 1. world_chain_hash extended.");
    write_receipt(&w1_receipt, &vault1, &vault2, w1_daa, "stage18_world1", &format!("{out_dir}/scorpion_stage18_world1_receipt.json"));

    // ── [2/3] GENERATE_WORLD #2: seed a second, totally different world -- reaches max_worlds ──
    let world2_covenant_id: [u8; 32] = sha256_bytes(b"world-2: sentinel dead-man switch, independent species");
    let world2_script_hash: [u8; 32] = sha256_bytes(b"world-2: an entirely different redeem_script shape");
    let vault3 = build_vault_after_world(&vault2, 80_000_000, &world2_covenant_id, &world2_script_hash);
    eprintln!("\n── [2/3] GENERATE_WORLD #2: a second, unrelated world -- reaches the max_worlds={max_worlds} cap ──");
    let w2_daa = w1_daa + 100;
    let w2_receipt = run_proof(
        &vault2, &vault3, 15, w2_daa,
        vec![world2_covenant_id.to_vec(), world2_script_hash.to_vec()],
        false,
    ).unwrap();
    eprintln!("  VERIFIED. gen2 VAULT -> gen3 VAULT. worlds_spawned 1 -> 2 (== max_worlds). world_chain_hash extended again.");
    write_receipt(&w2_receipt, &vault2, &vault3, w2_daa, "stage18_world2", &format!("{out_dir}/scorpion_stage18_world2_receipt.json"));

    // ── NEGATIVE: GENERATE_WORLD #3 -- cap already reached, must be rejected ──
    let world3_covenant_id: [u8; 32] = sha256_bytes(b"world-3: should never be born, cap exceeded");
    let world3_script_hash: [u8; 32] = sha256_bytes(b"world-3: rejected script");
    let vault4_attempt = build_vault_after_world(&vault3, 75_000_000, &world3_covenant_id, &world3_script_hash);
    eprintln!("\n── [3/3] NEGATIVE: GENERATE_WORLD #3 attempted -- worlds_spawned would be 3 > max_worlds={max_worlds} ──");
    let w3_daa = w2_daa + 100;
    run_proof(
        &vault3, &vault4_attempt, 15, w3_daa,
        vec![world3_covenant_id.to_vec(), world3_script_hash.to_vec()],
        true,
    );

    eprintln!("\n═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 18 COMPLETE: the scorpion has generated 2 worlds, each");
    eprintln!("  unrelated in shape to itself, both permanently chained into its");
    eprintln!("  own eternal memory. A third was refused -- even a god of creation");
    eprintln!("  has a limit its own DNA set for itself, at birth, forever.");
    eprintln!("═══════════════════════════════════════════════════════════════");
}

// moult.rs -- THE SPIRIT: a generic, repeatable heartbeat/moult prover.
//
// Unlike hatch.rs (which only proves the ONE literal genesis transition),
// this takes ANY previous receipt (with its full new_dna_hex saved) and
// proves the NEXT transition=0 (owner-authorized) moult on top of it,
// advancing generation by 1. Run it again on its own output and it keeps
// going -- this is what makes the organism keep living instead of hatching
// once and stopping.
//
// Usage: moult --prev-receipt=<path to prior receipt.json> --to-mode=<u8> --out=<path>
// Reads the same owner_secret/covenant_id key files as hatch.rs (mandatory key-backup rule).

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
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}
fn u32le(b: &[u8]) -> u32 { u32::from_le_bytes(b.try_into().unwrap()) }
fn u64le(b: &[u8]) -> u64 { u64::from_le_bytes(b.try_into().unwrap()) }
fn u16le(b: &[u8]) -> u16 { u16::from_le_bytes(b.try_into().unwrap()) }

// Exact byte-offset layout of the 721-byte DNA blob (matches build_body +
// append_memory + append_escrow_zeroed + append_metabolism in hatch.rs).
struct Dna {
    raw: Vec<u8>,
}
impl Dna {
    fn covenant_id(&self) -> &[u8] { &self.raw[4..36] }
    fn generation(&self) -> u32 { u32le(&self.raw[36..40]) }
    fn mode(&self) -> u8 { self.raw[40] }
    fn owner_commitment(&self) -> &[u8] { &self.raw[42..74] }
    // memory chromosome starts at 493
    fn transition_count(&self) -> u32 { u32le(&self.raw[525..529]) }
    fn creation_daa(&self) -> u64 { u64le(&self.raw[529..537]) }
    fn creation_txid(&self) -> &[u8] { &self.raw[537..569] }
    // metabolism chromosome starts at 683
    fn compute_budget(&self) -> u16 { u16le(&self.raw[683..685]) }
    fn fee_reserve(&self) -> u64 { u64le(&self.raw[685..693]) }
    fn min_output(&self) -> u64 { u64le(&self.raw[693..701]) }
    fn rate_window(&self) -> u32 { u32le(&self.raw[701..705]) }
    fn rate_max_spend(&self) -> u32 { u32le(&self.raw[705..709]) }
    fn window_start_daa(&self) -> u64 { u64le(&self.raw[709..717]) }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std_env::args().collect();
    let mut prev_receipt_path = String::new();
    let mut out_path = "/tmp/scorpion_moult_receipt.json".to_string();
    let mut to_mode: u8 = 255; // 255 = stay in same mode (heartbeat)
    let mut heartbeat_cost: u64 = 5_000_000;
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--prev-receipt=") { prev_receipt_path = v.to_string(); }
        if let Some(v) = a.strip_prefix("--out=") { out_path = v.to_string(); }
        if let Some(v) = a.strip_prefix("--to-mode=") { to_mode = v.parse().unwrap(); }
        if let Some(v) = a.strip_prefix("--cost=") { heartbeat_cost = v.parse().unwrap(); }
    }
    assert!(!prev_receipt_path.is_empty(), "need --prev-receipt=<path>");

    let prev: Value = serde_json::from_str(&fs::read_to_string(&prev_receipt_path).unwrap()).unwrap();
    let old_dna_raw = hex_to_bytes(prev["new_dna_hex"].as_str().unwrap());
    assert_eq!(old_dna_raw.len(), 721, "prev receipt's new_dna_hex must be the full 721-byte DNA");
    let old_dna = Dna { raw: old_dna_raw.clone() };
    let old_hash = sha256_bytes(&old_dna.raw);

    let owner_json: Value = serde_json::from_str(
        &fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap(),
    ).unwrap();
    let owner_secret = hex_to_bytes(owner_json["secret_hex"].as_str().unwrap());

    let target_mode = if to_mode == 255 { old_dna.mode() } else { to_mode };
    let new_generation = old_dna.generation() + 1;
    let new_fee_reserve = old_dna.fee_reserve().checked_sub(heartbeat_cost)
        .expect("fee_reserve would go negative -- organism would starve, refusing to prove");
    let current_daa = old_dna.creation_daa() + (new_generation as u64) * 1000; // fake DAA progression for local test

    // Rebuild the untouched chromosomes byte-for-byte from old_dna (transition=0 only
    // ever touches identity/generation/mode + memory + metabolism.fee_reserve).
    let mut new_dna = old_dna.raw.clone();
    new_dna[36..40].copy_from_slice(&new_generation.to_le_bytes());
    new_dna[40] = target_mode;
    new_dna[41] = old_dna.mode(); // prev_mode
    // memory chromosome: prev_state_hash + transition_count+1 (creation_daa/txid unchanged)
    new_dna[493..525].copy_from_slice(&old_hash);
    new_dna[525..529].copy_from_slice(&(old_dna.transition_count() + 1).to_le_bytes());
    // metabolism: fee_reserve decreases; rate-window must reset-or-advance per Chr5 rules
    new_dna[685..693].copy_from_slice(&new_fee_reserve.to_le_bytes());
    let rate_window = u32::from_le_bytes(old_dna.raw[701..705].try_into().unwrap()) as u64;
    let window_start_daa = u64::from_le_bytes(old_dna.raw[709..717].try_into().unwrap());
    let window_spend_count = u32::from_le_bytes(old_dna.raw[717..721].try_into().unwrap());
    if current_daa - window_start_daa > rate_window {
        new_dna[709..717].copy_from_slice(&current_daa.to_le_bytes());
        new_dna[717..721].copy_from_slice(&1u32.to_le_bytes());
    } else {
        new_dna[709..717].copy_from_slice(&window_start_daa.to_le_bytes());
        new_dna[717..721].copy_from_slice(&(window_spend_count + 1).to_le_bytes());
    }

    eprintln!("MOULT (the spirit) -- generation {} -> {}, mode {} -> {}",
        old_dna.generation(), new_generation, old_dna.mode(), target_mode);
    eprintln!("fee_reserve {} -> {}", old_dna.fee_reserve(), new_fee_reserve);

    let env = ExecutorEnv::builder()
        .write(&old_dna.raw).unwrap()
        .write(&new_dna).unwrap()
        .write(&0u8).unwrap() // transition = 0, owner-secret authorized
        .write(&current_daa).unwrap()
        .write(&owner_secret).unwrap()
        .build().unwrap();

    let prover = default_prover();
    eprintln!("Proving the moult... this may take a while.");
    let prove_info = prover
        .prove_with_opts(env, SCORPION_BRAIN_GUEST_ELF, &ProverOpts::succinct())
        .expect("proving failed");
    let receipt = prove_info.receipt;
    receipt.verify(SCORPION_BRAIN_GUEST_ID).expect("local verification failed");
    eprintln!("PROOF VERIFIED LOCALLY. Generation {new_generation} is alive.");

    let journal_bytes = receipt.journal.bytes.clone();
    let journal_digest = receipt.journal.digest();
    let succinct = receipt.inner.succinct().expect("receipt is not a succinct receipt");
    let image_id_bytes: Vec<u8> = SCORPION_BRAIN_GUEST_ID.iter().flat_map(|w: &u32| w.to_le_bytes()).collect();
    let claim_digest = succinct.claim.digest();
    let control_index = succinct.control_inclusion_proof.index;
    let control_digests_bytes: Vec<u8> = succinct.control_inclusion_proof.digests.iter()
        .flat_map(|d| d.as_bytes().to_vec()).collect();
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
        "old_dna_hex": hex::encode(&old_dna.raw),
        "new_dna_hex": hex::encode(&new_dna),
        "old_hash_hex": hex::encode(&old_hash),
        "generation": new_generation,
        "mode": target_mode,
        "new_fee_reserve": new_fee_reserve,
        "covenant_id_hex": to_hex(old_dna.covenant_id()),
        "kind": "moult_generic",
    });
    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote {out_path}");
}

// arma.rs -- Stage 19: "ARMA" (Chromosome 19, assembly/consent separation)
//
// Named for the real academic BFT design (Arma: Byzantine Fault Tolerant Consensus with
// Horizontal Scalability) that splits a pipeline into many parallel ASSEMBLERS -- who
// batch and chain data, scaling with raw bandwidth/CPU -- and one thin CONSENTER, who
// only orders/finalizes batch commitments, never touching individual items one at a time.
//
// Here: an off-chain assembler pre-computes an entire internal chain of ordinary
// VAULT->VAULT heartbeats (each hop obeying the exact same rules as a normal moult).
// ONE on-chain proof -- the consenter -- verifies the whole internal chain in a single
// shot and finalizes only the LAST state. generation and transition_count advance by
// batch_size instead of a fixed 1: the organism's own throughput scales horizontally
// with how much its assembler prepared, while the on-chain footprint stays exactly one
// transition, same as every other kind.
//
// Three real local proofs:
//   0. HATCH (transition 0): STEM -> VAULT (birth)
//   1. ARMA_CONSENT (transition 16): one batch of 5 assembled heartbeats consented to
//      in a SINGLE proof -- generation advances by 5, transition_count by 5, in one shot.
//   2. NEGATIVE: an attempted ARMA batch where one inner hop's fee_reserve does NOT
//      strictly decrease (a tampered/invalid assembled hop) -- correctly rejected.

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
fn append_arma(mut d: Vec<u8>, last_batch_size: u32, total_batched_transitions: u64) -> Vec<u8> {
    d.extend_from_slice(&last_batch_size.to_le_bytes());
    d.extend_from_slice(&total_batched_transitions.to_le_bytes());
    assert_eq!(d.len(), 914);
    d
}

// write_rest must write, IN ORDER: [batch_size if transition==16], current_daa,
// [transition-specific witness]. This mirrors the guest's exact env::read() sequence --
// batch_size is read before current_daa (right after `transition`), current_daa is read
// in Phase 1c, and any transition-specific witness (owner_secret, inner_hops, ...) is
// read last, inside that transition's own branch.
fn run_proof<F: FnOnce(&mut risc0_zkvm::ExecutorEnvBuilder)>(old_dna: &[u8], new_dna: &[u8], transition: u8, write_rest: F, expect_fail: bool) -> Option<Receipt> {
    let mut b = ExecutorEnv::builder();
    b.write(&old_dna.to_vec()).unwrap();
    b.write(&new_dna.to_vec()).unwrap();
    b.write(&transition).unwrap();
    write_rest(&mut b);
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

// Builds ONE ordinary internal hop (VAULT -> VAULT): generation+1, transition_count+1,
// fee_reserve strictly decreasing, memory-linked. Everything else stays byte-identical.
fn build_hop(prev_dna: &[u8], new_fee: u64) -> Vec<u8> {
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
    let creation_txid = hex_to_bytes(&"aa".repeat(32));
    let creation_daa: u64 = 480_600_000;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 10_000;
    let rate_max_spend: u32 = 20;
    let hatch_daa = creation_daa;

    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 19: \"ARMA\" -- assembly/consent separation");
    eprintln!("  Chromosome 19, transition 16");
    eprintln!("═══════════════════════════════════════════════════════════════\n");

    let body0 = build_body(&covenant_id, 0, MODE_STEM, MODE_STEM, &owner_commitment);
    let mem0 = append_memory(body0, &[0u8; 32], 0, creation_daa, &creation_txid);
    let esc0 = append_escrow_zeroed(mem0);
    let met0 = append_metabolism(esc0, 2700, 100_000_000, min_output, rate_window, rate_max_spend, hatch_daa, 0);
    let fall0 = append_fall_zeroed(met0);
    let hunt0 = append_hunt_zeroed(fall0);
    let dust0 = append_dust(hunt0, 0);
    let incub0 = append_incubation(dust0, 0);
    let wg0 = append_world_genesis(incub0, &[0u8; 32], 0, 0);
    let stem_dna = append_arma(wg0, 0, 0);

    eprintln!("── [0/2] HATCH: STEM -> VAULT (birth) ──");
    let vault1 = build_vault_from_stem(&stem_dna, 90_000_000);
    let owner_secret_clone = owner_secret.clone();
    let hatch_receipt = run_proof(&stem_dna, &vault1, 0, |b| {
        b.write(&hatch_daa).unwrap();
        b.write(&owner_secret_clone).unwrap();
    }, false).unwrap();
    eprintln!("  VERIFIED. gen0 STEM -> gen1 VAULT.");
    write_receipt(&hatch_receipt, &stem_dna, &vault1, hatch_daa, "stage19_hatch", &format!("{out_dir}/scorpion_stage19_hatch_receipt.json"));

    // ── [1/2] ARMA_CONSENT: assemble 5 ordinary heartbeats, consent to all 5 in ONE proof ──
    const BATCH_SIZE: u32 = 5;
    let mut hops: Vec<Vec<u8>> = Vec::new();
    let mut prev = vault1.clone();
    let mut fee = 85_000_000u64;
    for _ in 0..BATCH_SIZE {
        let next = build_hop(&prev, fee);
        hops.push(next.clone());
        prev = next;
        fee -= 5_000_000;
    }
    // The metabolism rate-window bookkeeping (window_start_daa, window_spend_count) is
    // frozen across every INTERNAL hop -- it only advances ONCE, for the single on-chain
    // footprint this whole batch produces, exactly like an ordinary heartbeat. So we patch
    // it in only on the very last hop, which is the one submitted as new_dna.
    let mut final_vault = hops.last().unwrap().clone();
    let vault1_wsc = u32::from_le_bytes(vault1[717..721].try_into().unwrap());
    final_vault[717..721].copy_from_slice(&(vault1_wsc + 1).to_le_bytes());
    // Chromosome 19's own bookkeeping (last_batch_size, total_batched_transitions) also
    // only advances once, on the final committed state -- the internal hops don't touch it.
    let vault1_total_batched = u64::from_le_bytes(vault1[906..914].try_into().unwrap());
    final_vault[902..906].copy_from_slice(&BATCH_SIZE.to_le_bytes());
    final_vault[906..914].copy_from_slice(&(vault1_total_batched + BATCH_SIZE as u64).to_le_bytes());
    // inner_hops witness = hops[0..batch_size-1] (everything except the LAST, which is new_dna itself)
    let inner_hops: Vec<Vec<u8>> = hops[..(BATCH_SIZE as usize - 1)].to_vec();

    eprintln!("\n── [1/2] ARMA_CONSENT: assemble {BATCH_SIZE} heartbeats, consent to all in ONE proof ──");
    let arma_daa = hatch_daa + 100;
    let inner_hops_clone = inner_hops.clone();
    let arma_receipt = run_proof(
        &vault1, &final_vault, 16,
        |b| {
            b.write(&BATCH_SIZE).unwrap();
            b.write(&arma_daa).unwrap();
            b.write(&inner_hops_clone).unwrap();
        },
        false,
    ).unwrap();
    eprintln!("  VERIFIED. gen1 -> gen{} in ONE proof. transition_count +{BATCH_SIZE}. total_batched_transitions 0 -> {BATCH_SIZE}.", 1 + BATCH_SIZE);
    write_receipt(&arma_receipt, &vault1, &final_vault, arma_daa, "stage19_arma_consent", &format!("{out_dir}/scorpion_stage19_arma_receipt.json"));

    // ── NEGATIVE: tamper with one inner hop's fee_reserve (does not strictly decrease) ──
    eprintln!("\n── [2/2] NEGATIVE: an assembled hop with a non-decreasing fee_reserve -- must be rejected ──");
    let mut bad_prev = vault1.clone();
    let mut bad_hops: Vec<Vec<u8>> = Vec::new();
    let mut bad_fee = 85_000_000u64;
    let mut last_hop_fee = u64::from_le_bytes(vault1[685..693].try_into().unwrap()); // vault1's own fee_reserve
    for i in 0..BATCH_SIZE {
        // tamper: at i==2, force fee STRICTLY ABOVE the actual previous hop's fee --
        // guarantees a genuine strict-decrease violation regardless of step size.
        let this_fee = if i == 2 { last_hop_fee + 1_000_000 } else { bad_fee };
        let next = build_hop(&bad_prev, this_fee);
        bad_hops.push(next.clone());
        bad_prev = next;
        last_hop_fee = this_fee;
        bad_fee -= 5_000_000;
    }
    let mut bad_final = bad_hops.last().unwrap().clone();
    bad_final[717..721].copy_from_slice(&(vault1_wsc + 1).to_le_bytes());
    bad_final[902..906].copy_from_slice(&BATCH_SIZE.to_le_bytes());
    bad_final[906..914].copy_from_slice(&(vault1_total_batched + BATCH_SIZE as u64).to_le_bytes());
    let bad_inner_hops: Vec<Vec<u8>> = bad_hops[..(BATCH_SIZE as usize - 1)].to_vec();
    let neg_daa = arma_daa + 100;
    run_proof(
        &vault1, &bad_final, 16,
        |b| {
            b.write(&BATCH_SIZE).unwrap();
            b.write(&neg_daa).unwrap();
            b.write(&bad_inner_hops).unwrap();
        },
        true,
    );

    eprintln!("\n═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 19 COMPLETE: the scorpion assembled {BATCH_SIZE} heartbeats off-chain");
    eprintln!("  and consented to the whole batch in ONE on-chain proof. Throughput");
    eprintln!("  scales with the assembler; the footprint stays exactly one transition.");
    eprintln!("═══════════════════════════════════════════════════════════════");
}

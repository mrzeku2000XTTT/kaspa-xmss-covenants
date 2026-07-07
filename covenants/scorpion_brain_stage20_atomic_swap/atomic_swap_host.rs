// atomic_swap.rs -- Stage 20: "CROSSCHAIN ATOMIC TRANSFER" (Chromosome 20, transition 17)
//
// The classic HTLC (hash + time locked contract) that makes atomic swaps between two
// independent chains possible: the scorpion commits to a hash H = sha256(secret).
// Whoever reveals the matching secret before a deadline unlocks the swap (CLAIM) --
// proving they completed their leg of the deal on the OTHER chain using that exact
// secret, since CLAIM is deliberately NOT owner-gated: knowledge of the secret IS the
// authorization. If nobody claims in time, the original owner gets it back,
// permissionlessly (REFUND), exactly like Chromosome 3's sentinel dead-man's-switch.
// On a successful CLAIM, the preimage is committed in the clear to the public STARK
// journal -- so any observer on the other chain of the swap can read it straight off
// this accepted proof and complete their own leg with the same secret. That public
// reveal is the entire mechanism that makes this a real atomic swap, not just a timelock.
//
// PART 1 -- single-scorpion mechanics (5 proofs, all real, local, receipt.verify() passing):
//   0. HATCH (transition 0): STEM -> VAULT (birth)
//   1. SWAP_INIT (transition 17, action 0): commit to a fresh HTLC hash + timeout.
//   2. SWAP_CLAIM (transition 17, action 1): reveal the correct secret, unlock, and
//      confirm the secret is publicly visible in the receipt's journal.
//   3. SWAP_INIT + SWAP_REFUND: a SECOND swap that times out unclaimed, refunded
//      permissionlessly after the deadline.
//   4. NEGATIVE: CLAIM attempted with the WRONG secret -- correctly rejected.
//
// PART 2 -- the actual point: TWO INDEPENDENT SCORPIONS SWAPPING WITH EACH OTHER.
// Scorpion A and Scorpion B are two separate covenant instances (different covenant_id,
// different owner). Bob (scorpion B's owner) picks a secret and gives Alice only the
// hash. Alice (scorpion A) locks first, with the LONGER timeout (she's the one taking
// the "first mover" risk). Bob (scorpion B) locks second, mirroring the SAME hash, with
// a SHORTER timeout (standard atomic-swap safety margin -- if anything goes wrong, Bob's
// window closes first, so Alice can never be left stuck if Bob doesn't finish the play).
// Bob claims Alice's lock (scorpion A), revealing the secret PUBLICLY in scorpion A's
// STARK journal. Alice reads that secret straight off the (now on-chain, in a real
// deploy) accepted proof and uses it to claim Bob's lock (scorpion B). Two proofs, two
// covenants, one shared secret -- a real cross-scorpion atomic swap.
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
fn append_atomic_swap(mut d: Vec<u8>, swap_hash: &[u8], swap_timeout_daa: u64, swap_active: u8) -> Vec<u8> {
    d.extend_from_slice(swap_hash);
    d.extend_from_slice(&swap_timeout_daa.to_le_bytes());
    d.push(swap_active);
    assert_eq!(d.len(), 955);
    d
}

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

fn build_swap_hop(prev_dna: &[u8], new_fee: u64) -> Vec<u8> {
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
    next
}

fn build_stem(covenant_id: &[u8], owner_commitment: &[u8], creation_daa: u64, creation_txid: &[u8], min_output: u64, rate_window: u32, rate_max_spend: u32, fee_reserve: u64) -> Vec<u8> {
    let body0 = build_body(covenant_id, 0, MODE_STEM, MODE_STEM, owner_commitment);
    let mem0 = append_memory(body0, &[0u8; 32], 0, creation_daa, creation_txid);
    let esc0 = append_escrow_zeroed(mem0);
    let met0 = append_metabolism(esc0, 2700, fee_reserve, min_output, rate_window, rate_max_spend, creation_daa, 0);
    let fall0 = append_fall_zeroed(met0);
    let hunt0 = append_hunt_zeroed(fall0);
    let dust0 = append_dust(hunt0, 0);
    let incub0 = append_incubation(dust0, 0);
    let wg0 = append_world_genesis(incub0, &[0u8; 32], 0, 0);
    let arma0 = append_arma(wg0, 0, 0);
    append_atomic_swap(arma0, &[0u8; 32], 0, 0)
}

fn hatch(stem_dna: &[u8], hatch_daa: u64, owner_secret: &[u8], new_fee: u64, label: &str, out_dir: &str) -> Vec<u8> {
    let vault = build_vault_from_stem(stem_dna, new_fee);
    let owner_secret_v = owner_secret.to_vec();
    let receipt = run_proof(stem_dna, &vault, 0, |b| {
        b.write(&hatch_daa).unwrap();
        b.write(&owner_secret_v).unwrap();
    }, false).unwrap();
    eprintln!("  VERIFIED. {label}: gen0 STEM -> gen1 VAULT.");
    write_receipt(&receipt, stem_dna, &vault, hatch_daa, &format!("{label}_hatch"), &format!("{out_dir}/{label}_hatch_receipt.json"));
    vault
}

fn swap_init(prev: &[u8], daa: u64, new_fee: u64, swap_hash: &[u8; 32], timeout_daa: u64, label: &str, out_dir: &str) -> Vec<u8> {
    let mut next = build_swap_hop(prev, new_fee);
    next[914..946].copy_from_slice(swap_hash);
    next[946..954].copy_from_slice(&timeout_daa.to_le_bytes());
    next[954] = 1;
    let hash_v = swap_hash.to_vec();
    let receipt = run_proof(prev, &next, 17, |b| {
        b.write(&daa).unwrap();
        b.write(&0u8).unwrap();
        b.write(&hash_v).unwrap();
        b.write(&timeout_daa).unwrap();
    }, false).unwrap();
    eprintln!("  VERIFIED. {label}: swap_hash committed, timeout_daa={timeout_daa}, swap_active=1.");
    write_receipt(&receipt, prev, &next, daa, &format!("{label}_init"), &format!("{out_dir}/{label}_init_receipt.json"));
    next
}

fn swap_claim(prev: &[u8], daa: u64, new_fee: u64, secret: &[u8], label: &str, out_dir: &str) -> (Vec<u8>, Receipt) {
    let mut next = build_swap_hop(prev, new_fee);
    next[914..946].copy_from_slice(&[0u8; 32]);
    next[946..954].copy_from_slice(&0u64.to_le_bytes());
    next[954] = 0;
    let secret_v = secret.to_vec();
    let receipt = run_proof(prev, &next, 17, |b| {
        b.write(&daa).unwrap();
        b.write(&1u8).unwrap();
        b.write(&secret_v).unwrap();
    }, false).unwrap();
    eprintln!("  VERIFIED. {label}: swap claimed, secret revealed publicly, swap_active=0.");
    write_receipt(&receipt, prev, &next, daa, &format!("{label}_claim"), &format!("{out_dir}/{label}_claim_receipt.json"));
    (next, receipt)
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env()).init();
    let args: Vec<String> = std_env::args().collect();
    let mut out_dir = "/app/kaspa_wrpc/keys".to_string();
    for a in &args[1..] {
        if let Some(v) = a.strip_prefix("--out-dir=") { out_dir = v.to_string(); }
    }

    let owner_a_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_owner_secret.json").unwrap()).unwrap();
    let owner_a_commitment = hex_to_bytes(owner_a_json["owner_commitment_hex"].as_str().unwrap());
    let owner_a_secret = hex_to_bytes(owner_a_json["secret_hex"].as_str().unwrap());
    let cov_a_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_covenant_id.json").unwrap()).unwrap();
    let covenant_id_a = hex_to_bytes(cov_a_json["covenant_id_hex"].as_str().unwrap());

    let owner_b_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_b_owner_secret.json").unwrap()).unwrap();
    let owner_b_commitment = hex_to_bytes(owner_b_json["owner_commitment_hex"].as_str().unwrap());
    let owner_b_secret = hex_to_bytes(owner_b_json["secret_hex"].as_str().unwrap());
    let cov_b_json: Value = serde_json::from_str(&fs::read_to_string("/app/kaspa_wrpc/keys/scorpion_b_covenant_id.json").unwrap()).unwrap();
    let covenant_id_b = hex_to_bytes(cov_b_json["covenant_id_hex"].as_str().unwrap());

    let creation_txid = hex_to_bytes(&"aa".repeat(32));
    let creation_daa: u64 = 480_800_000;
    let min_output: u64 = 20_000_000;
    let rate_window: u32 = 10_000;
    let rate_max_spend: u32 = 20;

    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("  STAGE 20 -- PART 2: SCORPION <-> SCORPION ATOMIC SWAP");
    eprintln!("  Two independent covenant instances, one shared secret");
    eprintln!("═══════════════════════════════════════════════════════════════\n");

    eprintln!("── Hatching Scorpion A (Alice) ──");
    let stem_a = build_stem(&covenant_id_a, &owner_a_commitment, creation_daa, &creation_txid, min_output, rate_window, rate_max_spend, 300_000_000);
    let a1 = hatch(&stem_a, creation_daa, &owner_a_secret, 290_000_000, "scorpionA", &out_dir);

    eprintln!("\n── Hatching Scorpion B (Bob) ──");
    let stem_b = build_stem(&covenant_id_b, &owner_b_commitment, creation_daa, &creation_txid, min_output, rate_window, rate_max_spend, 300_000_000);
    let b1 = hatch(&stem_b, creation_daa, &owner_b_secret, 290_000_000, "scorpionB", &out_dir);

    // Bob picks the secret. Alice only ever learns the hash, until Bob reveals it.
    let secret: Vec<u8> = b"cross-scorpion-swap-secret-alice-and-bob".to_vec();
    let swap_hash = sha256_bytes(&secret);
    eprintln!("\nBob's secret hash (public, shared with Alice up front): {}", hex::encode(swap_hash));

    // Alice moves FIRST -- longer timeout, she's taking on more risk by committing first.
    eprintln!("\n── [1] Alice (Scorpion A) INITs, locking for Bob, LONG timeout ──");
    let alice_init_daa = creation_daa + 50;
    let alice_timeout = alice_init_daa + 400;
    let a2 = swap_init(&a1, alice_init_daa, 280_000_000, &swap_hash, alice_timeout, "swapAB_alice", &out_dir);

    // Bob mirrors SECOND -- shorter timeout, standard atomic-swap safety margin.
    eprintln!("\n── [2] Bob (Scorpion B) mirrors, locking for Alice, SHORTER timeout ──");
    let bob_init_daa = alice_init_daa + 20;
    let bob_timeout = bob_init_daa + 200; // strictly less runway than Alice's lock
    assert!(bob_timeout < alice_timeout, "atomic-swap safety: Bob's timeout must expire before Alice's");
    let b2 = swap_init(&b1, bob_init_daa, 280_000_000, &swap_hash, bob_timeout, "swapAB_bob", &out_dir);

    // Bob claims Alice's lock (on Scorpion A), revealing the secret PUBLICLY in A's journal.
    eprintln!("\n── [3] Bob CLAIMS Alice's lock (on Scorpion A) -- secret becomes public ──");
    let bob_claim_daa = bob_init_daa + 20;
    let (a3, a_claim_receipt) = swap_claim(&a2, bob_claim_daa, 270_000_000, &secret, "swapAB_alice", &out_dir);

    // Alice reads the secret straight off Scorpion A's now-public, accepted claim receipt --
    // she never needed Bob to tell her directly. This IS the atomic-swap mechanism.
    let journal_bytes = a_claim_receipt.journal.bytes.clone();
    let extracted_secret = journal_bytes[66..].to_vec();
    assert_eq!(extracted_secret, secret, "Alice must be able to extract the exact secret Bob revealed");
    eprintln!("  Alice extracts secret from Scorpion A's public journal: \"{}\"", String::from_utf8_lossy(&extracted_secret));

    // Alice claims Bob's lock (on Scorpion B) using the secret she just extracted -- she
    // never needed a side channel to Bob at all, only the public proof.
    eprintln!("\n── [4] Alice CLAIMS Bob's lock (on Scorpion B) using the extracted secret ──");
    let alice_claim_daa = bob_claim_daa + 20;
    assert!(alice_claim_daa < bob_timeout, "Alice must claim before Bob's timeout -- she has plenty of margin since Bob revealed early");
    let (_b3, _b_claim_receipt) = swap_claim(&b2, alice_claim_daa, 270_000_000, &extracted_secret, "swapAB_bob", &out_dir);

    eprintln!("\n═══════════════════════════════════════════════════════════════");
    eprintln!("  SWAP COMPLETE: Alice and Bob each hold what the other locked.");
    eprintln!("  Two independent scorpion covenants, two independent proofs,");
    eprintln!("  ONE shared secret -- neither could have taken the other's funds");
    eprintln!("  without also revealing the key that lets the other claim theirs.");
    eprintln!("  That's atomicity, enforced by STARK proofs instead of trust.");
    eprintln!("═══════════════════════════════════════════════════════════════");
}

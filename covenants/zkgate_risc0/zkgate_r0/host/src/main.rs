// Host program: proves knowledge of `secret` such that sha256(secret_le_bytes) == committed hash.
// Generates a SUCCINCT receipt (STARK-based, Poseidon2 hashfn) suitable for Kaspa's
// OpZkPrecompile tag 0x21 (RISC0-Succinct) verification.
use methods::{ZKGATE_GUEST_ELF, ZKGATE_GUEST_ID};
use risc0_zkvm::{default_prover, sha::Digestible, ExecutorEnv, ProverOpts};
use std::env as std_env;
use std::fs;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std_env::args().collect();
    let secret: u64 = if args.len() > 1 {
        args[1].parse().expect("secret must be a u64")
    } else {
        999999u64
    };
    let out_path = if args.len() > 2 { args[2].clone() } else { "/tmp/r0_receipt.json".to_string() };

    let env = ExecutorEnv::builder()
        .write(&secret)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();

    eprintln!("Proving with secret={secret} ... this may take a while.");
    let prove_info = prover
        .prove_with_opts(env, ZKGATE_GUEST_ELF, &ProverOpts::succinct())
        .unwrap();

    let receipt = prove_info.receipt;
    receipt.verify(ZKGATE_GUEST_ID).expect("local verification failed");

    let journal_bytes = receipt.journal.bytes.clone();
    let journal_digest = receipt.journal.digest();

    let succinct = receipt
        .inner
        .succinct()
        .expect("receipt is not a succinct receipt");

    let image_id_bytes: Vec<u8> = ZKGATE_GUEST_ID.iter().flat_map(|w: &u32| w.to_le_bytes()).collect();
    let claim_digest = succinct.claim.digest();
    let control_index = succinct.control_inclusion_proof.index;
    let control_digests_bytes: Vec<u8> = succinct
        .control_inclusion_proof
        .digests
        .iter()
        .flat_map(|d| d.as_bytes().to_vec())
        .collect();
    let seal_bytes: Vec<u8> = succinct.seal.iter().flat_map(|w| w.to_le_bytes()).collect();

    eprintln!("image_id = {:?}", ZKGATE_GUEST_ID);
    eprintln!("journal_bytes (hex) = {}", hex::encode(&journal_bytes));
    eprintln!("journal_digest = {:?}", journal_digest);
    eprintln!("control_id = {:?}", succinct.control_id);
    eprintln!("hashfn = {:?}", succinct.hashfn);
    eprintln!("claim_digest = {:?}", claim_digest);
    eprintln!("control_index = {control_index}");
    eprintln!("control_digests count = {}", succinct.control_inclusion_proof.digests.len());
    eprintln!("seal len (u32 words) = {}", succinct.seal.len());

    let out = serde_json::json!({
        "secret": secret,
        "image_id_hex": hex::encode(image_id_bytes.as_slice()),
        "journal_bytes_hex": hex::encode(&journal_bytes),
        "journal_digest_hex": hex::encode(journal_digest.as_bytes()),
        "seal_hex": hex::encode(&seal_bytes),
        "control_id_hex": hex::encode(succinct.control_id.as_bytes()),
        "claim_digest_hex": hex::encode(claim_digest.as_bytes()),
        "control_index": control_index,
        "control_digests_hex": hex::encode(&control_digests_bytes),
        "hashfn": format!("{:?}", succinct.hashfn),
    });

    fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("Wrote receipt data to {out_path}");
}

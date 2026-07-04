mod error;
mod merkle;
mod rcpt;
mod receipt_claim;

use risc0_zkp::core::digest::Digest;
use serde::Deserialize;
use std::fs;

use crate::merkle::MerkleProof;
use crate::rcpt::{HashFnId, SuccinctReceipt};
use crate::receipt_claim::compute_assert_claim;

#[derive(Deserialize)]
struct ReceiptData {
    image_id_hex: String,
    journal_digest_hex: String,
    seal_hex: String,
    control_id_hex: String,
    claim_digest_hex: String,
    control_index: u32,
    control_digests_hex: String,
    hashfn: String,
}

fn digest_from_hex(s: &str) -> Digest {
    let bytes = hex::decode(s).expect("bad hex");
    Digest::try_from(bytes.as_slice()).expect("bad digest length")
}

fn seal_from_hex(s: &str) -> Vec<u32> {
    let bytes = hex::decode(s).expect("bad hex");
    bytes.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
}

fn digest_list_from_hex(s: &str) -> Vec<Digest> {
    let bytes = hex::decode(s).expect("bad hex");
    bytes.chunks_exact(32).map(|c| Digest::try_from(c).unwrap()).collect()
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "/tmp/r0_receipt2.json".to_string());
    let raw = fs::read_to_string(&path).expect("read receipt json");
    let data: ReceiptData = serde_json::from_str(&raw).expect("parse json");

    assert_eq!(data.hashfn.to_lowercase().trim_matches('"'), "poseidon2", "only poseidon2 supported");

    let image_id = digest_from_hex(&data.image_id_hex);
    let journal_digest = digest_from_hex(&data.journal_digest_hex);
    let seal = seal_from_hex(&data.seal_hex);
    let control_id = digest_from_hex(&data.control_id_hex);
    let claim_digest = digest_from_hex(&data.claim_digest_hex);
    let control_digests = digest_list_from_hex(&data.control_digests_hex);

    let control_inclusion_proof = MerkleProof { index: data.control_index, digests: control_digests };

    let rcpt = SuccinctReceipt::new(seal, control_id, claim_digest, HashFnId::Poseidon2, control_inclusion_proof);

    println!("Step 1: verify_integrity() (checks seal is a valid STARK proof + control_id merkle inclusion)...");
    rcpt.verify_integrity().expect("verify_integrity FAILED");
    println!("  -> PASSED");

    println!("Step 2: compute_assert_claim() (checks claim == hash(image_id, journal))...");
    compute_assert_claim(rcpt.claim(), image_id, journal_digest).expect("compute_assert_claim FAILED");
    println!("  -> PASSED");

    println!("\n✅ ALL CHECKS PASSED — this receipt would verify under Kaspa's OpZkPrecompile tag 0x21 (RISC0-Succinct).");
}

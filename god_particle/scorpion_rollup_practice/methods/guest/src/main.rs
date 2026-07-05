// Rollup practice guest — proves a BATCH of N chained state transitions with a
// SINGLE STARK, the same pattern real rollups/Based Apps use: many off-chain
// state transitions collapse into one proof that only commits the batch's
// start state, end state, and length. No intermediate state ever touches L1.
//
// Simplified DNA (70 bytes) — identity + ownership only, reusing the same
// "owner-secret authorizes stem<->vault" logic proven in the scorpion brain's
// Stage 1, just looped N times inside one guest execution instead of one
// transition per proof.
#![no_main]

use risc0_zkvm::guest::env;
use risc0_zkvm::sha::rust_crypto::{Digest as _, Sha256};

risc0_zkvm::guest::entry!(main);

const DNA_LEN: usize = 70;
const MODE_STEM: u8 = 0;
const MODE_VAULT: u8 = 1;

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

struct Dna {
    covenant_id: [u8; 32],
    generation: u32,
    mode: u8,
    prev_mode: u8,
    owner_commitment: [u8; 32],
}

fn parse_dna(b: &[u8]) -> Dna {
    assert_eq!(b.len(), DNA_LEN, "DNA blob must be exactly {DNA_LEN} bytes");
    let mut covenant_id = [0u8; 32];
    covenant_id.copy_from_slice(&b[0..32]);
    let generation = u32::from_le_bytes(b[32..36].try_into().unwrap());
    let mode = b[36];
    let prev_mode = b[37];
    let mut owner_commitment = [0u8; 32];
    owner_commitment.copy_from_slice(&b[38..70]);
    Dna {
        covenant_id,
        generation,
        mode,
        prev_mode,
        owner_commitment,
    }
}

fn mode_transition_allowed(from: u8, to: u8) -> bool {
    matches!(
        (from, to),
        (MODE_STEM, MODE_STEM) | (MODE_STEM, MODE_VAULT) | (MODE_VAULT, MODE_VAULT) | (MODE_VAULT, MODE_STEM)
    )
}

fn main() {
    let batch_size: u32 = env::read();
    assert!(batch_size > 0, "batch must contain at least 1 transition");

    let first_old_dna_bytes: Vec<u8> = env::read();
    let mut prev_new_dna_bytes: Option<Vec<u8>> = None;
    let mut last_new_dna_bytes: Vec<u8> = Vec::new();

    for i in 0..batch_size {
        // The first link's old_dna was already read above; subsequent links read
        // their own old_dna, which must byte-for-byte equal the previous link's
        // new_dna (this is what makes it a CHAIN, not just N independent proofs).
        let old_dna_bytes: Vec<u8> = if i == 0 {
            first_old_dna_bytes.clone()
        } else {
            let b: Vec<u8> = env::read();
            assert_eq!(
                Some(&b),
                prev_new_dna_bytes.as_ref(),
                "batch link {i} does not chain from the previous link's successor state"
            );
            b
        };
        let new_dna_bytes: Vec<u8> = env::read();
        let owner_secret: Vec<u8> = env::read();

        let old_dna = parse_dna(&old_dna_bytes);
        let new_dna = parse_dna(&new_dna_bytes);

        assert_eq!(old_dna.covenant_id, new_dna.covenant_id, "covenant_id mutated at link {i}");
        assert_eq!(new_dna.generation, old_dna.generation + 1, "generation must advance by 1 at link {i}");
        assert_eq!(new_dna.prev_mode, old_dna.mode, "prev_mode mismatch at link {i}");
        assert!(mode_transition_allowed(old_dna.mode, new_dna.mode), "illegal mode transition at link {i}");

        assert_eq!(owner_secret.len(), 32, "owner_secret must be 32 bytes at link {i}");
        assert_eq!(sha256(&owner_secret), old_dna.owner_commitment, "owner_secret mismatch at link {i}");
        assert_eq!(
            old_dna.owner_commitment, new_dna.owner_commitment,
            "owner_commitment must persist at link {i}"
        );

        prev_new_dna_bytes = Some(new_dna_bytes.clone());
        last_new_dna_bytes = new_dna_bytes;
    }

    // ── Commit only the batch's boundary — the whole point of a rollup ──
    let batch_start_hash = sha256(&first_old_dna_bytes);
    let batch_end_hash = sha256(&last_new_dna_bytes);

    let mut journal = Vec::with_capacity(32 + 32 + 4);
    journal.extend_from_slice(&batch_start_hash);
    journal.extend_from_slice(&batch_end_hash);
    journal.extend_from_slice(&batch_size.to_le_bytes());

    env::commit_slice(&journal);
}

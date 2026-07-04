use risc0_zkvm::guest::env;
use risc0_zkvm::sha::{Impl, Sha256};

fn main() {
    // Private witness: the secret value only the owner knows.
    let secret: u64 = env::read();

    // Compute a public commitment to the secret (little-endian bytes -> SHA256).
    let secret_bytes = secret.to_le_bytes();
    let hash = *Impl::hash_bytes(&secret_bytes);

    // Commit only the hash (public) to the journal -- the secret itself stays private.
    env::commit(&hash);
}

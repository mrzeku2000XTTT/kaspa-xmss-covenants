// Vendored (adapted) from kaspanet/rusty-kaspa crypto/txscript/src/zk_precompiles/risc0/merkle.rs
use risc0_zkp::core::{digest::Digest, hash::HashFn};
use risc0_core::field::baby_bear::BabyBear;
use anyhow::{ensure, Result};

#[derive(Clone, Debug)]
pub struct MerkleProof {
    pub index: u32,
    pub digests: Vec<Digest>,
}

impl MerkleProof {
    pub fn verify(&self, leaf: &Digest, root: &Digest, hashfn: &dyn HashFn<BabyBear>) -> Result<()> {
        ensure!(self.root(leaf, hashfn) == *root, "merkle proof verify failed");
        Ok(())
    }

    pub fn root(&self, leaf: &Digest, hashfn: &dyn HashFn<BabyBear>) -> Digest {
        let mut cur = *leaf;
        let mut cur_index = self.index;
        for sibling in &self.digests {
            cur = if cur_index & 1 == 0 {
                *hashfn.hash_pair(&cur, sibling)
            } else {
                *hashfn.hash_pair(sibling, &cur)
            };
            cur_index >>= 1;
        }
        cur
    }
}

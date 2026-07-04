// Vendored (adapted) from kaspanet/rusty-kaspa crypto/txscript/src/zk_precompiles/risc0/rcpt.rs
use std::collections::VecDeque;

use risc0_binfmt::read_sha_halfs;
use risc0_circuit_recursion::{control_id::ALLOWED_CONTROL_ROOT, CircuitImpl, CIRCUIT};
use risc0_core::field::baby_bear::{BabyBear, BabyBearElem};
use risc0_zkp::adapter::CircuitInfo;
use risc0_zkp::core::hash::{blake2b::Blake2bCpuHashSuite, poseidon2::Poseidon2HashSuite, sha::Sha256HashSuite, HashSuite};
use risc0_zkp::core::digest::Digest;
use risc0_zkp::verify::VerificationError;

use crate::error::R0Error;
use crate::merkle::MerkleProof;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HashFnId {
    Blake2b = 0,
    Poseidon2 = 1,
    Sha256 = 2,
}

impl TryFrom<u8> for HashFnId {
    type Error = R0Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(HashFnId::Blake2b),
            1 => Ok(HashFnId::Poseidon2),
            2 => Ok(HashFnId::Sha256),
            _ => Err(R0Error::InvalidHashFnId(value)),
        }
    }
}

#[derive(Debug)]
pub struct SuccinctReceipt {
    seal: Vec<u32>,
    control_id: Digest,
    claim: Digest,
    hashfn: HashFnId,
    control_inclusion_proof: MerkleProof,
}

impl SuccinctReceipt {
    pub fn new(seal: Vec<u32>, control_id: Digest, claim: Digest, hashfn: HashFnId, control_inclusion_proof: MerkleProof) -> Self {
        Self { seal, control_id, claim, hashfn, control_inclusion_proof }
    }

    pub fn claim(&self) -> &Digest {
        &self.claim
    }

    pub fn verify_integrity(&self) -> Result<(), R0Error> {
        let suite: HashSuite<BabyBear> = match self.hashfn {
            HashFnId::Blake2b => Blake2bCpuHashSuite::new_suite(),
            HashFnId::Poseidon2 => Poseidon2HashSuite::new_suite(),
            HashFnId::Sha256 => Sha256HashSuite::new_suite(),
        };

        let check_code = |_: u32, control_id: &Digest| -> Result<(), VerificationError> {
            if *control_id != self.control_id {
                return Err(VerificationError::ControlVerificationError { control_id: *control_id });
            }
            self.control_inclusion_proof
                .verify(control_id, &ALLOWED_CONTROL_ROOT, suite.hashfn.as_ref())
                .map_err(|_| VerificationError::ControlVerificationError { control_id: *control_id })
        };

        risc0_zkp::verify::verify(&CIRCUIT, &suite, &self.seal, check_code)?;

        let output_elems = self.seal[..CircuitImpl::OUTPUT_SIZE].iter().copied().map(BabyBearElem::new_raw);
        let mut seal_claim: VecDeque<u32> = VecDeque::from_iter(output_elems.map(|elem| elem.as_u32()));

        let control_root: Digest = seal_claim
            .drain(0..16)
            .enumerate()
            .filter_map(|(i, word)| (i & 1 == 0).then_some(word))
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| VerificationError::ReceiptFormatError)?;

        if control_root != ALLOWED_CONTROL_ROOT {
            return Err(VerificationError::ControlVerificationError { control_id: control_root })?;
        }

        let output_hash = read_sha_halfs(&mut seal_claim).map_err(|_| VerificationError::ReceiptFormatError)?;
        if output_hash != self.claim {
            return Err(VerificationError::JournalDigestMismatch)?;
        }

        Ok(())
    }
}

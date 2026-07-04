// Vendored (adapted) from kaspanet/rusty-kaspa crypto/txscript/src/zk_precompiles/risc0/receipt_claim.rs
use risc0_binfmt::{tagged_struct, Digestible, ExitCode, SystemState};
use risc0_zkp::core::{digest::Digest, hash::sha::Sha256};

use crate::error::R0Error;

#[derive(Clone, Debug)]
pub struct ReceiptClaim {
    pub pre: Digest,
    pub post: SystemState,
    pub exit_code: ExitCode,
    pub input: Digest,
    pub output: Output,
}

pub fn compute_assert_claim(claim: &Digest, image_id: Digest, journal_hash: Digest) -> Result<(), R0Error> {
    let computed_claim = ReceiptClaim {
        pre: image_id,
        post: SystemState { pc: 0, merkle_root: Digest::ZERO },
        exit_code: ExitCode::Halted(0),
        input: Digest::ZERO,
        output: Output { journal: journal_hash, assumptions: Digest::ZERO },
    }
    .digest::<risc0_zkp::core::hash::sha::Impl>();

    if *claim != computed_claim {
        return Err(R0Error::VerificationFailed);
    }
    Ok(())
}

impl Digestible for ReceiptClaim {
    fn digest<S: Sha256>(&self) -> Digest {
        let (sys_exit, user_exit) = self.exit_code.into_pair();
        tagged_struct::<S>(
            "risc0.ReceiptClaim",
            &[self.input, self.pre, self.post.digest::<S>(), self.output.digest::<S>()],
            &[sys_exit, user_exit],
        )
    }
}

#[derive(Clone, Debug)]
pub struct Output {
    pub journal: Digest,
    pub assumptions: Digest,
}

impl Digestible for Output {
    fn digest<S: Sha256>(&self) -> Digest {
        tagged_struct::<S>("risc0.Output", &[self.journal, self.assumptions], &[])
    }
}

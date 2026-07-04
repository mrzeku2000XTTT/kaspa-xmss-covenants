pragma circom 2.0.0;

include "node_modules/circomlib/circuits/poseidon.circom";

/*
 * ZKGate — Secret Knowledge Proof
 * 
 * Proves you know a secret preimage of a Poseidon hash
 * WITHOUT revealing the secret.
 * 
 * Private inputs: secret (never leaves your machine)
 * Public inputs:  hash   (stored in the covenant on-chain)
 * 
 * The Kaspa Toccata node verifies this Groth16 proof
 * directly in script via OpZkPrecompile tag 0x20.
 */
template SecretGate() {
    // Private: the secret you know (never revealed)
    signal input secret;

    // Public: the hash commitment stored in the covenant
    signal input hash;

    // Compute Poseidon hash of the secret
    component hasher = Poseidon(1);
    hasher.inputs[0] <== secret;

    // Constraint: computed hash must equal the public commitment
    hash === hasher.out;
}

component main {public [hash]} = SecretGate();

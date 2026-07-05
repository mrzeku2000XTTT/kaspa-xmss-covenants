pragma circom 2.0.0;

include "/app/zkgate/node_modules/circomlib/circuits/poseidon.circom";

// Proves: I know (nullifier, secret) such that Poseidon(nullifier,secret) is a leaf
// in the Merkle tree with the given root, WITHOUT revealing which leaf.
// nullifierHash = Poseidon(nullifier) is revealed publicly to prevent double-spend,
// but on its own doesn't link back to which commitment/leaf was used.
template MerkleTreeChecker(levels) {
    signal input leaf;
    signal input root;
    signal input pathElements[levels];
    signal input pathIndices[levels];

    signal levelHashes[levels+1];
    levelHashes[0] <== leaf;

    component hashers[levels];
    signal left[levels];
    signal right[levels];

    for (var i = 0; i < levels; i++) {
        pathIndices[i] * (1 - pathIndices[i]) === 0;

        left[i]  <== levelHashes[i] + pathIndices[i] * (pathElements[i] - levelHashes[i]);
        right[i] <== pathElements[i] + pathIndices[i] * (levelHashes[i] - pathElements[i]);

        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== left[i];
        hashers[i].inputs[1] <== right[i];
        levelHashes[i+1] <== hashers[i].out;
    }

    root === levelHashes[levels];
}

template Withdraw(levels) {
    signal input root;            // public
    signal input nullifierHash;   // public
    signal input nullifier;       // private
    signal input secret;          // private
    signal input pathElements[levels]; // private
    signal input pathIndices[levels];  // private

    component commitmentHasher = Poseidon(2);
    commitmentHasher.inputs[0] <== nullifier;
    commitmentHasher.inputs[1] <== secret;

    component nullifierHasher = Poseidon(1);
    nullifierHasher.inputs[0] <== nullifier;
    nullifierHasher.out === nullifierHash;

    component tree = MerkleTreeChecker(levels);
    tree.leaf <== commitmentHasher.out;
    tree.root <== root;
    for (var i = 0; i < levels; i++) {
        tree.pathElements[i] <== pathElements[i];
        tree.pathIndices[i] <== pathIndices[i];
    }
}

component main {public [root, nullifierHash]} = Withdraw(2);

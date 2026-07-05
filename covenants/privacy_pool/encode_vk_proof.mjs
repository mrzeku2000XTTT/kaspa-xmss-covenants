import { encodeProof, compressG1, compressG2 } from '/app/kaspa_wrpc/zk_encode.mjs';
import fs from 'fs';

const proof = JSON.parse(fs.readFileSync('/app/privacy_pool/proof.json'));
const pub = JSON.parse(fs.readFileSync('/app/privacy_pool/public.json'));
const vkey = JSON.parse(fs.readFileSync('/app/privacy_pool/verification_key.json'));

function bigToLE32(x) {
  const b = Buffer.alloc(32); let v = BigInt(x);
  for (let i = 0; i < 32; i++) { b[i] = Number(v & 0xffn); v >>= 8n; }
  return b;
}

const PROOF = encodeProof(proof);

// Public inputs in the SAME order as publicSignals (root, nullifierHash)
const PUB_INPUTS = pub.map(bigToLE32); // array of 32B buffers

// VK encoding: compressG1(alpha) + compressG2(beta) + compressG2(gamma) + compressG2(delta)
//   + u64LE(len IC) + compressG1(IC[0..n])
const alpha = compressG1(vkey.vk_alpha_1);
const beta = compressG2(vkey.vk_beta_2);
const gamma = compressG2(vkey.vk_gamma_2);
const delta = compressG2(vkey.vk_delta_2);
const icLen = vkey.IC.length;
const icLenBuf = Buffer.alloc(8);
icLenBuf.writeBigUInt64LE(BigInt(icLen));
const icBufs = vkey.IC.map(compressG1);

const VK = Buffer.concat([alpha, beta, gamma, delta, icLenBuf, ...icBufs]);

console.log('PROOF_HEX:' + PROOF.toString('hex'));
console.log('VK_HEX:' + VK.toString('hex'));
console.log('VK_LEN:' + VK.length);
console.log('PROOF_LEN:' + PROOF.length);
console.log('NUM_PUBLIC:' + PUB_INPUTS.length);
PUB_INPUTS.forEach((b, i) => console.log(`PUBLIC_${i}_HEX:` + b.toString('hex')));

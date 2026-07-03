// build_scripts.mjs — assembles the 3 chained redeem scripts for the XMSS workflow demo.
// Built in REVERSE dependency order (hop2 first) since each hop's script must hardcode
// the NEXT hop's P2SH scriptPubKey, which requires hashing that hop's script first.
import fs from 'fs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('../node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('../node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from '../node_modules/@noble/hashes/blake2b.js';
const { ScriptPublicKey, addressFromScriptPublicKey } = kaspa;

const AGENT_SPK = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

function pd(b) {
  const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  const lb = Buffer.alloc(2); lb.writeUInt16LE(len);
  return Buffer.concat([Buffer.from([0x4d]), lb, b]);
}

const OP_EQUAL = 0x87, OP_EQUALVERIFY = 0x88, OP_0 = 0x00, OP_TX_OUTPUT_SPK = 0xc3;

const build = JSON.parse(fs.readFileSync('./workflow_build.json', 'utf8'));
const ROOT = Buffer.from(build.master_root_hex, 'hex');
const hopVerify = { 0: Buffer.from(build.hop_verify_script_hex['0'], 'hex'),
                    1: Buffer.from(build.hop_verify_script_hex['1'], 'hex'),
                    2: Buffer.from(build.hop_verify_script_hex['2'], 'hex') };

function p2shSpkOf(redeemScript) {
  const scriptHash = blake2b(redeemScript, { dkLen: 32 });
  return Buffer.concat([Buffer.from([0xaa, 0x20]), Buffer.from(scriptHash), Buffer.from([0x87])]);
}
function addrOf(spkBuf) {
  return addressFromScriptPublicKey(new ScriptPublicKey(0, spkBuf.toString('hex')), 'mainnet').toString();
}

// hop2 (final): output[0] must pay the agent wallet directly (workflow complete -> payout)
const finalDestSpk = Buffer.concat([Buffer.from([0x00,0x00]), Buffer.from(AGENT_SPK, 'hex')]); // version(u16LE) + script pairing used by OpTxOutputSpk comparisons... see note below
// NOTE: OpTxOutputSpk compares against the RAW scriptPublicKey (script bytes only, no version pairing) based on prior ZKGate usage (OWNER_SPK there matched raw pushed AGENT_SPK).
// Re-check: deploy_zkgate_v4 pushed OWNER_SPK = version(u16LE=0)+script as the comparison value. We mirror that exactly.

function hopScript(verifyScript, destSpkForCompare) {
  return Buffer.concat([
    verifyScript,
    pd(ROOT), Buffer.from([OP_EQUALVERIFY]),
    pd(destSpkForCompare), Buffer.from([OP_0]), Buffer.from([OP_TX_OUTPUT_SPK]), Buffer.from([OP_EQUAL]),
  ]);
}

// --- hop2: script proves leaf2, requires output[0] == agent wallet (final payout) ---
const hop2Script = hopScript(hopVerify[2], finalDestSpk);
const hop2Spk = p2shSpkOf(hop2Script);
const hop2Addr = addrOf(hop2Spk);

// --- hop1: script proves leaf1, requires output[0] == hop2's covenant address ---
const hop2DestForCompare = Buffer.concat([Buffer.from([0x00,0x00]), hop2Spk]);
const hop1Script = hopScript(hopVerify[1], hop2DestForCompare);
const hop1Spk = p2shSpkOf(hop1Script);
const hop1Addr = addrOf(hop1Spk);

// --- hop0: script proves leaf0, requires output[0] == hop1's covenant address ---
const hop1DestForCompare = Buffer.concat([Buffer.from([0x00,0x00]), hop1Spk]);
const hop0Script = hopScript(hopVerify[0], hop1DestForCompare);
const hop0Spk = p2shSpkOf(hop0Script);
const hop0Addr = addrOf(hop0Spk);

console.log('hop0 (DEPLOY/DEPOSIT address):', hop0Addr, '-', hop0Script.length, 'bytes');
console.log('hop1 (intermediate)         :', hop1Addr, '-', hop1Script.length, 'bytes');
console.log('hop2 (intermediate)         :', hop2Addr, '-', hop2Script.length, 'bytes');
console.log('final payout                :', AGENT_ADDR);

fs.writeFileSync('./scripts_out.json', JSON.stringify({
  hop0: { script_hex: hop0Script.toString('hex'), spk_hex: hop0Spk.toString('hex'), addr: hop0Addr },
  hop1: { script_hex: hop1Script.toString('hex'), spk_hex: hop1Spk.toString('hex'), addr: hop1Addr },
  hop2: { script_hex: hop2Script.toString('hex'), spk_hex: hop2Spk.toString('hex'), addr: hop2Addr },
  final_addr: AGENT_ADDR,
}, null, 2));
console.log('Written scripts_out.json');

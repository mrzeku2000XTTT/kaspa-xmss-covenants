// build_scripts.mjs — appends the output-enforcement suffix (OpTxOutputSpk + OpTxOutputAmount)
// to the game_script from build_stag_hunt.py, producing the final deployable redeem script.
import fs from 'fs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('../node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('../node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from '../node_modules/@noble/hashes/blake2b.js';
const { ScriptPublicKey, addressFromScriptPublicKey, Address, payToAddressScript } = kaspa;

const AGENT_SPK = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

function pd(b) {
  const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  const lb = Buffer.alloc(2); lb.writeUInt16LE(len);
  return Buffer.concat([Buffer.from([0x4d]), lb, b]);
}
function encodeNum(n) {
  // minimal CScriptNum encoding (Bitcoin-style little-endian, sign bit on last byte)
  if (n === 0) return Buffer.alloc(0);
  const neg = n < 0; n = Math.abs(n);
  const bytes = [];
  while (n > 0) { bytes.push(n & 0xff); n = Math.floor(n / 256); }
  if (bytes[bytes.length-1] & 0x80) bytes.push(neg ? 0x80 : 0x00);
  else if (neg) bytes[bytes.length-1] |= 0x80;
  return Buffer.from(bytes);
}

const OP_EQUAL = 0x87, OP_EQUALVERIFY = 0x88;
const OP_TX_OUTPUT_AMOUNT = 0xc2, OP_TX_OUTPUT_SPK = 0xc3;

const build = JSON.parse(fs.readFileSync('./stag_hunt_build.json', 'utf8'));
const gameScript = Buffer.from(build.game_script_hex, 'hex');

// --- Player addresses come from CLI args (real users, not agent-controlled) ---
function arg(name) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : undefined; }
const ADDR_A = arg('addrA') || AGENT_ADDR;  // default: demo uses the agent's own wallet for both, for a controlled self-test
const ADDR_B = arg('addrB') || AGENT_ADDR;

const spkA = Buffer.from(payToAddressScript(new Address(ADDR_A)).script, 'hex');
const spkB = Buffer.from(payToAddressScript(new Address(ADDR_B)).script, 'hex');
const spkAgent = Buffer.from(AGENT_SPK, 'hex');

function checkOutput(idx, spk, isLast) {
  // proven format (validated on zkgate mainnet deploy): version(u16LE=0) + script
  const versionedSpk = Buffer.concat([Buffer.from([0x00,0x00]), spk]);
  let s = Buffer.concat([]);
  s = Buffer.concat([s, pd(encodeNum(idx)), Buffer.from([OP_TX_OUTPUT_SPK]), pd(versionedSpk), Buffer.from([OP_EQUALVERIFY])]);
  return s;
}

let suffix = Buffer.concat([]);
// SPK checks for outputs 0,1,2 (destinations fixed regardless of branch outcome)
suffix = Buffer.concat([suffix, checkOutput(0, spkA), checkOutput(1, spkB), checkOutput(2, spkAgent)]);
// amount checks -- stack is [amtA, amtB, amtAgent] with amtAgent on top
suffix = Buffer.concat([suffix, pd(encodeNum(2)), Buffer.from([OP_TX_OUTPUT_AMOUNT]), Buffer.from([OP_EQUALVERIFY])]); // consumes amtAgent
suffix = Buffer.concat([suffix, pd(encodeNum(1)), Buffer.from([OP_TX_OUTPUT_AMOUNT]), Buffer.from([OP_EQUALVERIFY])]); // consumes amtB
suffix = Buffer.concat([suffix, pd(encodeNum(0)), Buffer.from([OP_TX_OUTPUT_AMOUNT]), Buffer.from([OP_EQUAL])]);       // final check -> leaves bool

const fullScript = Buffer.concat([gameScript, suffix]);
console.log('Full redeem script:', fullScript.length, 'bytes');

const scriptHash = blake2b(fullScript, { dkLen: 32 });
const spkBuf = Buffer.concat([Buffer.from([0xaa, 0x20]), Buffer.from(scriptHash), Buffer.from([0x87])]);
const covAddr = addressFromScriptPublicKey(new ScriptPublicKey(0, spkBuf.toString('hex')), 'mainnet').toString();

console.log('Covenant (deposit) address:', covAddr);
console.log('addrA (player A payout):', ADDR_A);
console.log('addrB (player B payout):', ADDR_B);
console.log('agent (house/unclaimed):', AGENT_ADDR);

fs.writeFileSync('./scripts_out.json', JSON.stringify({
  redeem_script_hex: fullScript.toString('hex'),
  spk_hex: spkBuf.toString('hex'),
  covenant_address: covAddr,
  addrA: ADDR_A, addrB: ADDR_B, agent_addr: AGENT_ADDR,
}, null, 2));
console.log('Written scripts_out.json');

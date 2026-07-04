/**
 * deploy_zkgate_risc0.mjs — RISC0-Succinct (tag 0x21) variant of ZKGate.
 *
 * Destination-locked ZK covenant: funds can only move to --owner (script-enforced via
 * OpTxOutputSpk), either via a valid RISC Zero STARK receipt proving knowledge of the
 * secret behind the committed hash, OR the owner's own signature fallback (OP_ELSE).
 *
 * Reads a receipt JSON produced by risc0gate/zkgate_r0/host (fields: image_id_hex,
 * journal_digest_hex, seal_hex, control_id_hex, claim_digest_hex, control_index,
 * control_digests_hex, hashfn).
 *
 * Usage: node deploy_zkgate_risc0.mjs --owner=<kaspa:address> --deposit=<KAS> --receipt=<path.json> [--dry-run]
 * Env: KASPA_AGENT_PRIVKEY
 */
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from './node_modules/@noble/hashes/blake2b.js';
import fs from 'fs';

const {
  RpcClient, Resolver, createTransactions, PrivateKey, createInputSignature,
  addressFromScriptPublicKey, payToAddressScript, Address, ScriptPublicKey,
} = kaspa;

function arg(name, def) {
  const a = process.argv.find(x => x.startsWith(`--${name}=`));
  return a ? a.split('=').slice(1).join('=') : def;
}

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK  = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

const OWNER_ADDR = arg('owner');
const DEPOSIT_KAS = parseFloat(arg('deposit', '0.5'));
const RECEIPT_PATH = arg('receipt', '/tmp/r0_receipt2.json');
const DRY_RUN = process.argv.includes('--dry-run');

if (!AGENT_PRIV_HEX) { console.log('RESULT_ERROR: Missing KASPA_AGENT_PRIVKEY'); process.exit(1); }
if (!OWNER_ADDR || !OWNER_ADDR.startsWith('kaspa:')) { console.log('RESULT_ERROR: Missing/invalid --owner address'); process.exit(1); }

let ownerSpkObj, OWNER_PK;
try {
  ownerSpkObj = payToAddressScript(new Address(OWNER_ADDR));
  OWNER_PK = Buffer.from(ownerSpkObj.script.slice(2, -2), 'hex');
  if (ownerSpkObj.script.length !== 68) throw new Error('not a plain pubkey address');
} catch (e) {
  console.log('RESULT_ERROR: Could not decode owner address as a plain pubkey address: ' + e.message);
  process.exit(1);
}
const OWNER_SPK = Buffer.concat([Buffer.from([0x00, 0x00]), Buffer.from(ownerSpkObj.script, 'hex')]);

// Push-data helper — supports up to 1,000,000-byte elements (MAX_SCRIPT_ELEMENT_SIZE_POST_TOCCATA)
// via OpData1..75 / OpPushData1(0x4c,u8) / OpPushData2(0x4d,u16) / OpPushData4(0x4e,u32).
function pd(b) {
  const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  if (len <= 65535) {
    const lb = Buffer.alloc(2); lb.writeUInt16LE(len);
    return Buffer.concat([Buffer.from([0x4d]), lb, b]);
  }
  const lb = Buffer.alloc(4); lb.writeUInt32LE(len);
  return Buffer.concat([Buffer.from([0x4e]), lb, b]);
}
function pi(n) {
  if (n === 0) return Buffer.from([0x00]);
  if (n >= 1 && n <= 16) return Buffer.from([0x50 + n]);
  const bytes = []; let abs = Math.abs(n);
  while (abs > 0) { bytes.push(abs & 0xff); abs >>= 8; }
  if (bytes[bytes.length - 1] & 0x80) bytes.push(0x00);
  return pd(Buffer.from(bytes));
}
function u32le(n) { const b = Buffer.alloc(4); b.writeUInt32LE(n); return b; }

const OP_IF = 0x63, OP_ELSE = 0x67, OP_ENDIF = 0x68;
const OP_CHECKSIG = 0xac, OP_ZK = 0xa6, OP_TX_OUTPUT_SPK = 0xc3, OP_EQUALVERIFY = 0x88;

async function main() {
  if (!fs.existsSync(RECEIPT_PATH)) { console.log('RESULT_ERROR: receipt file not found: ' + RECEIPT_PATH); process.exit(1); }
  const r = JSON.parse(fs.readFileSync(RECEIPT_PATH, 'utf8'));
  const hashfnStr = String(r.hashfn).toLowerCase().replace(/"/g, '');
  if (hashfnStr !== 'poseidon2') { console.log('RESULT_ERROR: only poseidon2 hashfn supported, got ' + hashfnStr); process.exit(1); }

  const claim = Buffer.from(r.claim_digest_hex, 'hex');
  const controlIndex = u32le(r.control_index);
  const controlDigests = Buffer.from(r.control_digests_hex, 'hex');
  const seal = Buffer.from(r.seal_hex, 'hex');
  const journal = Buffer.from(r.journal_digest_hex, 'hex');
  const imageId = Buffer.from(r.image_id_hex, 'hex');
  const controlId = Buffer.from(r.control_id_hex, 'hex');
  const hashfnByte = Buffer.from([0x01]); // Poseidon2 = 1

  if (claim.length !== 32 || journal.length !== 32 || imageId.length !== 32 || controlId.length !== 32) {
    console.log('RESULT_ERROR: one of claim/journal/image_id/control_id is not exactly 32 bytes');
    process.exit(1);
  }
  if (controlDigests.length % 32 !== 0) { console.log('RESULT_ERROR: control_digests not a multiple of 32 bytes'); process.exit(1); }
  if (seal.length % 4 !== 0) { console.log('RESULT_ERROR: seal length not a multiple of 4 bytes'); process.exit(1); }

  // Push order (verified against rusty-kaspa zk_precompiles/risc0/mod.rs destructure order
  // AND independently cross-checked against the SilverScript RFC's documented push order):
  // claim, control_index, control_digests, seal, journal, image_id, control_id, hashfn, tag
  const redeemScript = Buffer.concat([
    Buffer.from([OP_IF]),
      pd(OWNER_SPK), pi(0), Buffer.from([OP_TX_OUTPUT_SPK]), Buffer.from([OP_EQUALVERIFY]),
      pd(claim), pd(controlIndex), pd(controlDigests), pd(seal), pd(journal),
      pd(imageId), pd(controlId), pd(hashfnByte), pd(Buffer.from([0x21])),
      Buffer.from([OP_ZK]),
    Buffer.from([OP_ELSE]),
      pd(OWNER_PK), Buffer.from([OP_CHECKSIG]),
    Buffer.from([OP_ENDIF]),
  ]);

  const scriptHash = blake2b(redeemScript, { dkLen: 32 });
  const spkBuf = Buffer.concat([Buffer.from([0xaa, 0x20]), Buffer.from(scriptHash), Buffer.from([0x87])]);
  const G_SPK = spkBuf.toString('hex');
  const G_ADDR = addressFromScriptPublicKey(new ScriptPublicKey(0, G_SPK), 'mainnet').toString();

  console.log('Redeem script:', redeemScript.length, 'bytes');
  console.log('Covenant addr:', G_ADDR);

  if (DRY_RUN) {
    console.log('RESULT_JSON:', JSON.stringify({ dryRun: true, covenantAddress: G_ADDR, redeemScriptBytes: redeemScript.length }));
    return;
  }

  const DEPOSIT = BigInt(Math.round(DEPOSIT_KAS * 1e8));
  const FEE = 300_000n;

  const utxos = await (await fetch(`https://api.kaspa.org/addresses/${AGENT_ADDR}/utxos`)).json();
  utxos.sort((a, b) => Number(BigInt(b.utxoEntry.amount) - BigInt(a.utxoEntry.amount)));
  const u = utxos[0];
  if (!u) { console.log('RESULT_ERROR: No UTXOs available'); process.exit(1); }
  const IN_TXID = u.outpoint.transactionId;
  const IN_IDX = u.outpoint.index;
  const IN_AMT = BigInt(u.utxoEntry.amount);
  const IN_DAA = BigInt(u.utxoEntry.blockDaaScore);
  const CHANGE = IN_AMT - DEPOSIT - FEE;
  if (CHANGE < 0n) { console.log('RESULT_ERROR: Insufficient funds'); process.exit(1); }

  const privKey = new PrivateKey(AGENT_PRIV_HEX);
  const result = await createTransactions({
    entries: [{
      address: AGENT_ADDR,
      outpoint: { transactionId: IN_TXID, index: IN_IDX },
      amount: IN_AMT,
      scriptPublicKey: { version: 0, script: AGENT_SPK },
      blockDaaScore: IN_DAA,
      isCoinbase: false,
    }],
    outputs: [{ address: G_ADDR, amount: DEPOSIT }],
    changeAddress: AGENT_ADDR,
    priorityFee: FEE,
    networkId: 'mainnet',
  });
  const tx = result.transactions[0].transaction;
  tx.version = 1;

  const sigResult = createInputSignature(tx, 0, privKey, 1);
  const sigScriptHex = Buffer.from(sigResult).toString('ascii');

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const out0val = tx.outputs[0].value;
  const out1val = tx.outputs[1]?.value;

  const txObj = {
    version: 1,
    inputs: [{
      previousOutpoint: { transactionId: IN_TXID, index: IN_IDX },
      signatureScript: sigScriptHex,
      sequence: 0, sigOpCount: 0, computeBudget: 10,
    }],
    outputs: [
      { value: Number(out0val), scriptPublicKey: { script: G_SPK, version: 0 } },
      ...(out1val ? [{ value: Number(out1val), scriptPublicKey: { script: AGENT_SPK, version: 0 } }] : []),
    ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  const resp = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});

  console.log('RESULT_JSON:', JSON.stringify({
    txId: resp.transactionId, covenantAddress: G_ADDR,
    redeemScriptHex: redeemScript.toString('hex'),
    lockedKas: DEPOSIT_KAS,
    explorerUrl: 'https://kaspa.stream/transactions/' + resp.transactionId,
  }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 500) || String(e))); process.exit(1); });

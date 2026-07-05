/**
 * deploy_pool_demo.mjs — Deploy the Privacy Pool proof-of-mechanism covenant.
 * Locks funds behind: OP_IF branch = valid Groth16 proof of Merkle-membership
 * (root fixed, nullifierHash+proof embedded from our real 4-leaf tree withdrawal),
 * OP_ELSE = owner sig fallback (escape hatch for this test).
 */
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('/app/kaspa_wrpc/node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('/app/kaspa_wrpc/node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from '/app/kaspa_wrpc/node_modules/@noble/hashes/blake2b.js';
import fs from 'fs';

const { RpcClient, Resolver, createTransactions, PrivateKey, createInputSignature,
  addressFromScriptPublicKey, ScriptPublicKey } = kaspa;

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK  = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';
const OWNER_PK = Buffer.from('6d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60', 'hex');

function pd(b) {
  const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  const lb = Buffer.alloc(2); lb.writeUInt16LE(len);
  return Buffer.concat([Buffer.from([0x4d]), lb, b]);
}
function pi(n) {
  if (n === 0) return Buffer.from([0x00]);
  if (n >= 1 && n <= 16) return Buffer.from([0x50 + n]);
  const bytes = []; let abs = Math.abs(n);
  while (abs > 0) { bytes.push(abs & 0xff); abs >>= 8; }
  if (bytes[bytes.length - 1] & 0x80) bytes.push(0x00);
  return pd(Buffer.from(bytes));
}
const OP_IF = 0x63, OP_ELSE = 0x67, OP_ENDIF = 0x68;
const OP_CHECKSIG = 0xac, OP_ZK = 0xa6;

function bigToLE32(x) {
  const b = Buffer.alloc(32); let v = BigInt(x);
  for (let i = 0; i < 32; i++) { b[i] = Number(v & 0xffn); v >>= 8n; }
  return b;
}

const tree = JSON.parse(fs.readFileSync('/app/privacy_pool/tree_data.json'));
const proof = JSON.parse(fs.readFileSync('/app/privacy_pool/proof.json'));
const pub = JSON.parse(fs.readFileSync('/app/privacy_pool/public.json'));
const encLines = fs.readFileSync('/tmp/enc_out.txt', 'utf8').split('\n');
const get = (k) => encLines.find(l => l.startsWith(k)).split(':')[1];

const PROOF = Buffer.from(get('PROOF_HEX'), 'hex');
const VK = Buffer.from(get('VK_HEX'), 'hex');
const ROOT = bigToLE32(pub[0]);
const NULLIFIER_HASH = bigToLE32(pub[1]);

const redeemScript = Buffer.concat([
  Buffer.from([OP_IF]),
    pd(NULLIFIER_HASH), pd(ROOT), pi(2), pd(PROOF), pd(VK), pd(Buffer.from([0x20])),
    Buffer.from([OP_ZK]),
  Buffer.from([OP_ELSE]),
    pd(OWNER_PK), Buffer.from([OP_CHECKSIG]),
  Buffer.from([OP_ENDIF]),
]);

const scriptHash = blake2b(redeemScript, { dkLen: 32 });
const spkBuf = Buffer.concat([Buffer.from([0xaa, 0x20]), Buffer.from(scriptHash), Buffer.from([0x87])]);
const G_SPK = spkBuf.toString('hex');
const G_ADDR = addressFromScriptPublicKey(new ScriptPublicKey(0, G_SPK), 'mainnet').toString();

console.log('Redeem script bytes:', redeemScript.length);
console.log('Covenant address:', G_ADDR);
fs.writeFileSync('/app/privacy_pool/redeem_script.hex', redeemScript.toString('hex'));
fs.writeFileSync('/app/privacy_pool/covenant_addr.txt', G_ADDR);

const DRY_RUN = process.argv.includes('--dry-run');
if (DRY_RUN) { console.log('DRY RUN - stopping before broadcast'); process.exit(0); }

const DEPOSIT_KAS = parseFloat(process.argv.find(a=>a.startsWith('--deposit='))?.split('=')[1] || '0.4');

async function main() {
  const DEPOSIT = BigInt(Math.round(DEPOSIT_KAS * 1e8));
  const FEE = 3_000_000n;

  const utxos = await (await fetch(`https://api.kaspa.org/addresses/${AGENT_ADDR}/utxos`)).json();
  utxos.sort((a, b) => Number(BigInt(b.utxoEntry.amount) - BigInt(a.utxoEntry.amount)));
  // use as many UTXOs as needed to cover deposit+fee with change either 0 or comfortably above the KIP-9 floor
  let picked = [];
  let sum = 0n;
  for (const u of utxos) {
    picked.push(u);
    sum += BigInt(u.utxoEntry.amount);
    if (sum >= DEPOSIT + FEE + 20_000_000n || sum === DEPOSIT + FEE) break;
  }
  if (sum < DEPOSIT + FEE) { console.log('RESULT_ERROR: Insufficient funds'); process.exit(1); }

  const entries = picked.map(u => ({
    address: AGENT_ADDR,
    outpoint: { transactionId: u.outpoint.transactionId, index: u.outpoint.index },
    amount: BigInt(u.utxoEntry.amount),
    scriptPublicKey: { version: 0, script: AGENT_SPK },
    blockDaaScore: BigInt(u.utxoEntry.blockDaaScore),
    isCoinbase: false,
  }));

  const privKey = new PrivateKey(AGENT_PRIV_HEX);
  let result = await createTransactions({
      entries,
      outputs: [{ address: G_ADDR, amount: DEPOSIT }],
      changeAddress: AGENT_ADDR,
      priorityFee: FEE,
      networkId: 'mainnet',
    });

  const tx = result.transactions[0].transaction;
  tx.version = 1;
  const inputSigs = entries.map((_, i) => Buffer.from(createInputSignature(tx, i, privKey, 1)).toString('ascii'));

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const out0val = tx.outputs[0].value;
  const out1val = tx.outputs[1]?.value;

  const txObj = {
    version: 1,
    inputs: entries.map((e, i) => ({
      previousOutpoint: { transactionId: e.outpoint.transactionId, index: e.outpoint.index },
      signatureScript: inputSigs[i],
      sequence: 0, sigOpCount: 0, computeBudget: 10,
    })),
    outputs: [
      { value: Number(out0val), scriptPublicKey: { script: G_SPK, version: 0 } },
      ...(out1val ? [{ value: Number(out1val), scriptPublicKey: { script: AGENT_SPK, version: 0 } }] : []),
    ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});

  console.log('RESULT_JSON:', JSON.stringify({
    txId: r.transactionId, covenantAddress: G_ADDR,
    redeemScriptHex: redeemScript.toString('hex'),
    lockedKas: DEPOSIT_KAS,
    explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId,
  }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 500) || String(e))); process.exit(1); });

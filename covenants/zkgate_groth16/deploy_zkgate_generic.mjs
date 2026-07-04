/**
 * deploy_zkgate_generic.mjs — Reusable ZKGate deployer for the SuperZK pipeline.
 *
 * Destination-locked ZK covenant: funds can only ever move to --owner (script-enforced
 * via OpTxOutputSpk), either via a valid Groth16 proof-of-secret OR the owner's own
 * signature fallback (OP_ELSE branch) — same proven pattern as ZKGate v4.
 *
 * Usage: node deploy_zkgate_generic.mjs --owner=<kaspa:address> --deposit=<KAS> [--secret=<int>] [--dry-run]
 * Env: KASPA_AGENT_PRIVKEY
 *
 * Outputs "RESULT_JSON: {...}" with:
 *   { txId, covenantAddress, redeemScriptHex, secret, publicHash, lockedKas, explorerUrl }
 * on success, or "RESULT_ERROR: <message>" on failure.
 */
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from './node_modules/@noble/hashes/blake2b.js';
import { encodeProof } from './zk_encode.mjs';
import crypto from 'crypto';

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
const DEPOSIT_KAS = parseFloat(arg('deposit', '0.12'));
const SECRET_ARG = arg('secret', null);
const DRY_RUN = process.argv.includes('--dry-run');

if (!AGENT_PRIV_HEX) { console.log('RESULT_ERROR: Missing KASPA_AGENT_PRIVKEY'); process.exit(1); }
if (!OWNER_ADDR || !OWNER_ADDR.startsWith('kaspa:')) { console.log('RESULT_ERROR: Missing/invalid --owner address'); process.exit(1); }

let ownerSpkObj, OWNER_PK;
try {
  ownerSpkObj = payToAddressScript(new Address(OWNER_ADDR));
  OWNER_PK = Buffer.from(ownerSpkObj.script.slice(2, -2), 'hex'); // strip push-len byte + OP_CHECKSIG
  if (ownerSpkObj.script.length !== 68) throw new Error('not a plain pubkey address');
} catch (e) {
  console.log('RESULT_ERROR: Could not decode owner address as a plain pubkey address: ' + e.message);
  process.exit(1);
}
const OWNER_SPK = Buffer.concat([Buffer.from([0x00, 0x00]), Buffer.from(ownerSpkObj.script, 'hex')]); // version u16LE + script

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
const OP_CHECKSIG = 0xac, OP_ZK = 0xa6, OP_TX_OUTPUT_SPK = 0xc3, OP_EQUALVERIFY = 0x88;

const VK = Buffer.from('654c99a0160cad4ac289f9ce9ef821c52939f0f4f98e7c43ee404b891fb37d83a8c33af5a4654c2da203c2114e9183bdbb4a65a60185f1ced88238462d69df229039570ff7206ea185e3fa1e3b55b8fb19df09e665d5a5490bfb0595c353ef93edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e192418664450746c69b36dda1c5d21e218f4fce1464d89a2142fcdacee8bc8613088393b94673f20cdc6f4bf57c9cab418489228a3f14da4339a186b3d163ab80802000000000000008f67ed07859f5b07c04e247e2ed2d2dd588a480059a437453dd06527c3370d14b2ad8d3749f0ef2cc5c7d4af130221cb1f1793905a28e3928f4f46b2a2e6658b', 'hex');

async function main() {
  const secret = SECRET_ARG ? BigInt(SECRET_ARG) : (BigInt('0x' + crypto.randomBytes(16).toString('hex')));

  const circomlibjs = require('/app/zkgate/node_modules/circomlibjs');
  const snarkjs = require('/app/zkgate/node_modules/snarkjs');
  const poseidon = await circomlibjs.buildPoseidon();
  const F = poseidon.F;
  const hashOut = F.toString(poseidon([secret]));

  const { proof, publicSignals } = await snarkjs.groth16.fullProve(
    { secret: secret.toString(), hash: hashOut },
    '/app/zkgate/secret_gate_js/secret_gate.wasm',
    '/app/zkgate/secret_gate_final.zkey'
  );

  function bigToLE32(x) {
    const b = Buffer.alloc(32); let v = x;
    for (let i = 0; i < 32; i++) { b[i] = Number(v & 0xffn); v >>= 8n; }
    return b;
  }
  const PROOF = encodeProof(proof);
  const INPUT = bigToLE32(BigInt(publicSignals[0]));

  const redeemScript = Buffer.concat([
    Buffer.from([OP_IF]),
      pd(OWNER_SPK), pi(0), Buffer.from([OP_TX_OUTPUT_SPK]), Buffer.from([OP_EQUALVERIFY]),
      pd(INPUT), pi(1), pd(PROOF), pd(VK), pd(Buffer.from([0x20])),
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
  console.log('Secret:', secret.toString(), ' Public hash:', hashOut);

  if (DRY_RUN) {
    console.log('RESULT_JSON:', JSON.stringify({ dryRun: true, covenantAddress: G_ADDR, secret: secret.toString(), publicHash: hashOut }));
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

  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});

  console.log('RESULT_JSON:', JSON.stringify({
    txId: r.transactionId, covenantAddress: G_ADDR,
    redeemScriptHex: redeemScript.toString('hex'),
    secret: secret.toString(), publicHash: hashOut,
    lockedKas: DEPOSIT_KAS,
    explorerUrl: 'https://kaspa.stream/tx/' + r.transactionId,
  }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 500) || String(e))); process.exit(1); });

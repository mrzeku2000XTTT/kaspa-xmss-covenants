/**
 * spend_zkgate_risc0.mjs — Spend a RISC0-Succinct ZKGate covenant via the ZK proof branch
 * (OP_IF: destination-lock + OP_ZK tag 0x21). Requires NO private key for the covenant input —
 * the redeem script itself, once revealed, is the only "witness" needed.
 *
 * Usage: node spend_zkgate_risc0.mjs --cov-txid=... --cov-idx=0 --cov-amt=<sompi> \
 *          --fee-txid=... --fee-idx=0 --fee-amt=<sompi> --redeem=<hex or @file> \
 *          --out-addr=<kaspa:addr> [--compute-budget=3000] [--fee=50000000]
 * Env: KASPA_AGENT_PRIVKEY (signs the fee input)
 */
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import fs from 'fs';

const { RpcClient, Resolver, PrivateKey, signScriptHash, TransactionSigningHash } = kaspa;

function arg(name, def) {
  const a = process.argv.find(x => x.startsWith(`--${name}=`));
  return a ? a.split('=').slice(1).join('=') : def;
}

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';

const COV_TXID = arg('cov-txid');
const COV_IDX = parseInt(arg('cov-idx', '0'));
const COV_AMT = BigInt(arg('cov-amt'));
const COV_SPK = arg('cov-spk');
const FEE_TXID = arg('fee-txid');
const FEE_IDX = parseInt(arg('fee-idx', '0'));
const FEE_AMT = BigInt(arg('fee-amt'));
const OUT_SPK = arg('out-spk', AGENT_SPK);
const FEE = BigInt(arg('fee', '50000000'));
const COMPUTE_BUDGET_ZK = parseInt(arg('compute-budget', '3000'));
const DRY_RUN = process.argv.includes('--dry-run');

let redeemArg = arg('redeem');
if (redeemArg.startsWith('@')) redeemArg = fs.readFileSync(redeemArg.slice(1), 'utf8').trim();
const redeemScript = Buffer.from(redeemArg, 'hex');

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

function u16le(n) { const b = Buffer.alloc(2); b.writeUInt16LE(Number(n)); return b; }
function u32le(n) { const b = Buffer.alloc(4); b.writeUInt32LE(Number(n)); return b; }
function u64le(n) { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; }
function varBytes(buf) { return Buffer.concat([u64le(buf.length), buf]); }
function kh(data) { const h = new TransactionSigningHash(); h.update(data); return Buffer.from(h.finalize(), 'hex'); }

async function main() {
  const OUT_AMT = COV_AMT + FEE_AMT - FEE;
  if (OUT_AMT < 0n) { console.log('RESULT_ERROR: fee exceeds available funds'); process.exit(1); }

  const inputs = [
    { txid: COV_TXID, idx: COV_IDX, value: COV_AMT, spk: Buffer.from(COV_SPK, 'hex') },
    { txid: FEE_TXID, idx: FEE_IDX, value: FEE_AMT, spk: Buffer.from(AGENT_SPK, 'hex') },
  ];
  const outputs = [{ value: OUT_AMT, spk: Buffer.from(OUT_SPK, 'hex') }];

  const prevOutpointsBuf = Buffer.concat(inputs.map(i => Buffer.concat([Buffer.from(i.txid, 'hex'), u32le(i.idx)])));
  const h_prevAll = kh(prevOutpointsBuf);
  const h_seqAll = kh(Buffer.concat(inputs.map(() => u64le(0n))));
  const outBuf = Buffer.concat(outputs.map(o => Buffer.concat([u64le(o.value), u16le(0), varBytes(o.spk), Buffer.from([0])])));
  const h_out = kh(outBuf);

  function buildSighash(idx) {
    const inp = inputs[idx];
    const preimage = Buffer.concat([
      u16le(1), h_prevAll, h_seqAll,
      Buffer.from(inp.txid, 'hex'), u32le(inp.idx),
      u16le(0), varBytes(inp.spk),
      u64le(inp.value), u64le(0n),
      h_out,
      u64le(0n), Buffer.alloc(20), u64le(0n), Buffer.alloc(32),
      Buffer.from([0x01]),
    ]);
    return kh(preimage);
  }

  const selectorTrue = pd(Buffer.from([0x01]));
  const scriptSig0 = Buffer.concat([selectorTrue, pd(redeemScript)]).toString('hex');

  const privKey = new PrivateKey(AGENT_PRIV_HEX);
  const sighash1Hex = buildSighash(1).toString('hex');
  const scriptSig1 = signScriptHash(sighash1Hex, privKey);

  const txObj = {
    version: 1,
    inputs: [
      { previousOutpoint: { transactionId: COV_TXID, index: COV_IDX }, signatureScript: scriptSig0, sequence: 0, sigOpCount: 0, computeBudget: COMPUTE_BUDGET_ZK },
      { previousOutpoint: { transactionId: FEE_TXID, index: FEE_IDX }, signatureScript: scriptSig1, sequence: 0, sigOpCount: 0, computeBudget: 10 },
    ],
    outputs: [{ value: Number(OUT_AMT), scriptPublicKey: { script: OUT_SPK, version: 0 } }],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  console.log('Redeem script:', redeemScript.length, 'bytes');
  console.log('scriptSig0 length:', scriptSig0.length / 2, 'bytes');
  console.log('Out amount:', Number(OUT_AMT) / 1e8, 'KAS');

  if (DRY_RUN) {
    console.log('RESULT_JSON:', JSON.stringify({ dryRun: true, outAmount: Number(OUT_AMT) / 1e8 }));
    return;
  }

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  try {
    await rpc.connect();
    const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
    console.log('RESULT_JSON:', JSON.stringify({ txId: r.transactionId, explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId }));
  } catch (e) {
    console.log('RESULT_ERROR: ' + (e.message?.slice(0, 800) || String(e)));
  } finally {
    await rpc.disconnect().catch(() => {});
    process.exit(0);
  }
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 500) || String(e))); process.exit(1); });

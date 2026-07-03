/**
 * xmssmt_spend.mjs — Spend the 5-layer XMSS^MT-style covenant by revealing all 255
 * WOTS+ witnesses (51 per layer x 5 layers). ~2,165 SHA256 hash operations executed
 * on-chain, zero signatures for this input, zero private key.
 */
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });

const { RpcClient, Resolver, PrivateKey, TransactionSigningHash, signScriptHash } = kaspa;

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK  = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

const COV_TXID = '92e384f3134c18a7ca1958e4cdfd42c5af89908ac4325f72a3b546890ac9f413';
const COV_IDX = 0;
const COV_AMT = 15000000n;
const COV_SPK_HEX = 'aa20091a187f7c7d3b6e16253dbf450555cedef0445cdcadb1cecaa642a2de70965487';
const REDEEM_SCRIPT_HEX = fs.readFileSync('./multisig_2of2_script.hex', 'utf8').trim();
const WITNESS = JSON.parse(fs.readFileSync('./multisig_2of2_witness.json', 'utf8'));

function pd(b) {
  const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  if (len <= 65535) { const lb = Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]), lb, b]); }
  const lb = Buffer.alloc(4); lb.writeUInt32LE(len);
  return Buffer.concat([Buffer.from([0x4e]), lb, b]);
}
function u16le(n) { const b = Buffer.alloc(2); b.writeUInt16LE(Number(n)); return b; }
function u32le(n) { const b = Buffer.alloc(4); b.writeUInt32LE(Number(n)); return b; }
function u64le(n) { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; }
function varBytes(buf) { return Buffer.concat([u64le(buf.length), buf]); }
function kh(data) { const h = new TransactionSigningHash(); h.update(data); return Buffer.from(h.finalize(), 'hex'); }

async function main() {
  const utxos = await (await fetch(`https://api.kaspa.org/addresses/${AGENT_ADDR}/utxos`)).json();
  utxos.sort((a,b)=>Number(BigInt(b.utxoEntry.amount)-BigInt(a.utxoEntry.amount)));
  const feeUtxo = utxos[0];
  if (!feeUtxo) throw new Error('No agent UTXOs for fee input');
  const FEE_TXID = feeUtxo.outpoint.transactionId;
  const FEE_IDX = feeUtxo.outpoint.index;
  const FEE_AMT = BigInt(feeUtxo.utxoEntry.amount);

  const FEE = 32_000_000n; // ~130KB scriptSig
  const OUT_AMT = COV_AMT + FEE_AMT - FEE;

  const redeemScript = Buffer.from(REDEEM_SCRIPT_HEX, 'hex');

  const inputs = [
    { txid: COV_TXID, idx: COV_IDX, value: COV_AMT, spk: Buffer.from(COV_SPK_HEX, 'hex') },
    { txid: FEE_TXID, idx: FEE_IDX, value: FEE_AMT, spk: Buffer.from(AGENT_SPK, 'hex') },
  ];
  const outputs = [ { value: OUT_AMT, spk: Buffer.from(AGENT_SPK, 'hex') } ];

  const prevOutpointsBuf = Buffer.concat(inputs.map(i => Buffer.concat([Buffer.from(i.txid,'hex'), u32le(i.idx)])));
  const h_prevAll = kh(prevOutpointsBuf);
  const h_seqAll = kh(Buffer.concat(inputs.map(() => u64le(0n))));
  const outBuf = Buffer.concat(outputs.map(o => Buffer.concat([u64le(o.value), u16le(0), varBytes(o.spk), Buffer.from([0])])));
  const h_out = kh(outBuf);

  function buildSighash(idx) {
    const inp = inputs[idx];
    const preimage = Buffer.concat([
      u16le(1), h_prevAll, h_seqAll,
      Buffer.from(inp.txid,'hex'), u32le(inp.idx),
      u16le(0), varBytes(inp.spk),
      u64le(inp.value), u64le(0n),
      h_out,
      u64le(0n), Buffer.alloc(20), u64le(0n), Buffer.alloc(32),
      Buffer.from([0x01]),
    ]);
    return kh(preimage);
  }

  const privKey = new PrivateKey(AGENT_PRIV_HEX);

  const witnessParts = WITNESS.map(h => pd(Buffer.from(h, 'hex')));
  const scriptSig0 = Buffer.concat([ ...witnessParts, pd(redeemScript) ]);

  const sighash1 = buildSighash(1);
  const scriptSig1 = signScriptHash(sighash1.toString('hex'), privKey);

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const txObj = {
    version: 1,
    inputs: [
      { previousOutpoint: { transactionId: COV_TXID, index: COV_IDX }, signatureScript: scriptSig0.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 1400 },
      { previousOutpoint: { transactionId: FEE_TXID, index: FEE_IDX }, signatureScript: scriptSig1.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 10 },
    ],
    outputs: [
      { value: Number(OUT_AMT), scriptPublicKey: { script: AGENT_SPK, version: 0 } },
    ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  console.log('scriptSig0 bytes:', scriptSig0.length);

  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});

  console.log('RESULT_JSON:', JSON.stringify({
    txId: r.transactionId,
    explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId,
  }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 1200) || String(e))); process.exit(1); });

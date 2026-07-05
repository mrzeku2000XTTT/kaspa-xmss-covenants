import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import fs from 'fs';
const { RpcClient, Resolver, PrivateKey, signScriptHash, TransactionSigningHash } = kaspa;

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

const COV_TXID = '756f85560a59183b8ad6afa50fe4e14777863baeabe537ea4210171a63e9ff14';
const COV_IDX = 0;
const COV_AMT = 60000000n;
const COV_SPK = Buffer.from('aa2073abf907e36a511745ae3697981ba26a8bc5ebeb326a16df97c6741dae84d09d87', 'hex');
const redeemScript = Buffer.from(fs.readFileSync('/tmp/hatch_redeem.hex','utf8').trim(), 'hex');
const FEE = 46000000n;
const OUT_AMT = COV_AMT - FEE;

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
  const inputs = [{ txid: COV_TXID, idx: COV_IDX, value: COV_AMT, spk: COV_SPK }];
  const outputs = [{ value: OUT_AMT, spk: Buffer.from(AGENT_SPK, 'hex') }];

  const prevOutpointsBuf = Buffer.concat(inputs.map(i => Buffer.concat([Buffer.from(i.txid, 'hex'), u32le(i.idx)])));
  const h_prevAll = kh(prevOutpointsBuf);
  const h_seqAll = kh(Buffer.concat(inputs.map(() => u64le(0n))));
  const outBuf = Buffer.concat(outputs.map(o => Buffer.concat([u64le(o.value), u16le(0), varBytes(o.spk), Buffer.from([0])])));
  const h_out = kh(outBuf);

  const preimage = Buffer.concat([
    u16le(1), h_prevAll, h_seqAll,
    Buffer.from(COV_TXID, 'hex'), u32le(COV_IDX),
    u16le(0), varBytes(COV_SPK),
    u64le(COV_AMT), u64le(0n),
    h_out,
    u64le(0n), Buffer.alloc(20), u64le(0n), Buffer.alloc(32),
    Buffer.from([0x01]),
  ]);
  const sighash = kh(preimage).toString('hex');

  const privKey = new PrivateKey(AGENT_PRIV_HEX);
  const sig = signScriptHash(sighash, privKey);

  const selectorFalse = Buffer.from([0x00]); // OP_0 -> false, takes OP_ELSE branch
  const scriptSig = Buffer.concat([selectorFalse, pd(redeemScript)]);
  // sig already includes push of signature; need to prepend BEFORE selector+redeem per ELSE branch stack order:
  // ELSE branch expects: <sig> pushed, then selector(0), then redeem. Stack consumed bottom-up: OWNER_PK OP_CHECKSIG needs sig on stack already.
  const sigBuf = Buffer.from(sig, 'hex');
  const finalScriptSig = Buffer.concat([sigBuf, selectorFalse, pd(redeemScript)]).toString('hex');

  const txObj = {
    version: 1,
    inputs: [
      { previousOutpoint: { transactionId: COV_TXID, index: COV_IDX }, signatureScript: finalScriptSig, sequence: 0, sigOpCount: 0, computeBudget: 60 },
    ],
    outputs: [{ value: Number(OUT_AMT), scriptPublicKey: { script: AGENT_SPK, version: 0 } }],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();
  try {
    const resp = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
    console.log('RESULT_JSON:', JSON.stringify({ txId: resp.transactionId }));
  } catch (e) {
    console.log('RESULT_ERROR: ' + (e.message || String(e)));
  }
  await rpc.disconnect().catch(() => {});
}
main();

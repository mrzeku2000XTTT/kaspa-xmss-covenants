// spend_hop.mjs --step=0|1|2 --txid=<cov txid> --idx=<cov idx> --amt=<cov amount sompi>
// Spends one hop of the XMSS workflow chain: reveals that leaf's WOTS+ witness (no signature),
// and the script itself enforces where output[0] must go (next hop, or final payout on step 2).
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('../node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('../node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
const { RpcClient, Resolver, PrivateKey, TransactionSigningHash, signScriptHash } = kaspa;

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK  = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';
if (!AGENT_PRIV_HEX) { console.log('RESULT_ERROR: Missing KASPA_AGENT_PRIVKEY'); process.exit(1); }

function arg(name) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : undefined; }
const STEP = parseInt(arg('step'));
const COV_TXID = arg('txid');
const COV_IDX = parseInt(arg('idx'));
const COV_AMT = BigInt(arg('amt'));

const scripts = JSON.parse(fs.readFileSync('./scripts_out.json', 'utf8'));
const build = JSON.parse(fs.readFileSync('./workflow_build.json', 'utf8'));

const hopKey = `hop${STEP}`;
const REDEEM_SCRIPT_HEX = scripts[hopKey].script_hex;
const COV_SPK_HEX = scripts[hopKey].spk_hex;
const WITNESS = build.witnesses_hex[String(STEP)];

const nextSpkHex = STEP < 2 ? scripts[`hop${STEP+1}`].spk_hex : AGENT_SPK;
const nextAddr = STEP < 2 ? scripts[`hop${STEP+1}`].addr : AGENT_ADDR;

function pd(b) { const len=b.length; if (len<=75) return Buffer.concat([Buffer.from([len]),b]); if (len<=255) return Buffer.concat([Buffer.from([0x4c,len]),b]); const lb=Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]),lb,b]); }
function u16le(n){const b=Buffer.alloc(2);b.writeUInt16LE(Number(n));return b;}
function u32le(n){const b=Buffer.alloc(4);b.writeUInt32LE(Number(n));return b;}
function u64le(n){const b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(n));return b;}
function varBytes(buf){return Buffer.concat([u64le(buf.length),buf]);}
function kh(data){const h=new TransactionSigningHash();h.update(data);return Buffer.from(h.finalize(),'hex');}

async function main() {
  const utxos = await (await fetch(`https://api.kaspa.org/addresses/${AGENT_ADDR}/utxos`)).json();
  utxos.sort((a,b)=>Number(BigInt(b.utxoEntry.amount)-BigInt(a.utxoEntry.amount)));
  const feeUtxo = utxos[0];
  if (!feeUtxo) throw new Error('No agent UTXOs for fee input');
  const FEE_TXID = feeUtxo.outpoint.transactionId, FEE_IDX = feeUtxo.outpoint.index;
  const FEE_AMT = BigInt(feeUtxo.utxoEntry.amount);
  const FEE = 16_000_000n; // bumped after mainnet fee rejection (needed ~13.85M for mass 138508)

  const redeemScript = Buffer.from(REDEEM_SCRIPT_HEX, 'hex');
  const inputs = [
    { txid: COV_TXID, idx: COV_IDX, value: COV_AMT, spk: Buffer.from(COV_SPK_HEX, 'hex') },
    { txid: FEE_TXID, idx: FEE_IDX, value: FEE_AMT, spk: Buffer.from(AGENT_SPK, 'hex') },
  ];
  const changeAmt = FEE_AMT - FEE;
  const outputs = [
    { value: COV_AMT, spk: Buffer.from(nextSpkHex, 'hex') },
    { value: changeAmt, spk: Buffer.from(AGENT_SPK, 'hex') },
  ];

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
      { previousOutpoint: { transactionId: COV_TXID, index: COV_IDX }, signatureScript: scriptSig0.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 700 },
      { previousOutpoint: { transactionId: FEE_TXID, index: FEE_IDX }, signatureScript: scriptSig1.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 10 },
    ],
    outputs: [
      { value: Number(outputs[0].value), scriptPublicKey: { script: nextSpkHex, version: 0 } },
      { value: Number(outputs[1].value), scriptPublicKey: { script: AGENT_SPK, version: 0 } },
    ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});
  console.log('RESULT_JSON:', JSON.stringify({
    step: STEP, txId: r.transactionId, nextAddr, forwardedSompi: COV_AMT.toString(),
    explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId,
  }));
}
main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0,1500) || String(e))); process.exit(1); });

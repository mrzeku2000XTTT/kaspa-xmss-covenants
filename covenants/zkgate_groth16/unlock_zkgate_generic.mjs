/**
 * unlock_zkgate_generic.mjs — Generic ZKGate unlock via the ZK-proof branch.
 * Proof/VK/public-input are already baked into the redeem script, so the covenant
 * input's scriptSig is just [selector=1 push][redeem script reveal] — no signature needed.
 *
 * Usage:
 *   node unlock_zkgate_generic.mjs --covenant_address=<addr> --redeem_script_hex=<hex> --owner_address=<addr> [--fee=35000000]
 *
 * Fetches covenant UTXO + agent fee UTXO live from api.kaspa.org, builds+signs+submits.
 * Wrap with ./with_lock.sh for wallet-safety.
 */
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });

const { RpcClient, Resolver, PrivateKey, TransactionSigningHash, signScriptHash, payToAddressScript, Address } = kaspa;

function arg(name, def) {
  const m = process.argv.find(a => a.startsWith(`--${name}=`));
  return m ? m.split('=').slice(1).join('=') : def;
}

const COVENANT_ADDRESS = arg('covenant_address');
const REDEEM_SCRIPT_HEX = arg('redeem_script_hex');
const OWNER_ADDRESS = arg('owner_address');
const FEE = BigInt(arg('fee', '35000000'));
const COMPUTE_BUDGET_ZK = Number(arg('compute_budget', '2000'));
const COMPUTE_BUDGET_P2PK = 10;

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK  = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

if (!COVENANT_ADDRESS || !REDEEM_SCRIPT_HEX || !OWNER_ADDRESS) {
  console.log('RESULT_ERROR: missing required args (covenant_address, redeem_script_hex, owner_address)');
  process.exit(1);
}

function pd(b) {
  const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  const lb = Buffer.alloc(2); lb.writeUInt16LE(len);
  return Buffer.concat([Buffer.from([0x4d]), lb, b]);
}
function u16le(n) { const b = Buffer.alloc(2); b.writeUInt16LE(Number(n)); return b; }
function u32le(n) { const b = Buffer.alloc(4); b.writeUInt32LE(Number(n)); return b; }
function u64le(n) { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; }
function varBytes(buf) { return Buffer.concat([u64le(buf.length), buf]); }
function kh(data) { const h = new TransactionSigningHash(); h.update(data); return Buffer.from(h.finalize(), 'hex'); }

async function fetchUtxos(addr) {
  const res = await fetch(`https://api.kaspa.org/addresses/${addr}/utxos`);
  return res.json();
}

async function main() {
  const [covUtxos, agentUtxos] = await Promise.all([
    fetchUtxos(COVENANT_ADDRESS),
    fetchUtxos(AGENT_ADDR),
  ]);

  if (!covUtxos.length) { console.log('RESULT_ERROR: no UTXO found at covenant address (already spent?)'); process.exit(1); }
  if (!agentUtxos.length) { console.log('RESULT_ERROR: no fee UTXO available in agent wallet'); process.exit(1); }

  const covUtxo = covUtxos[0];
  const feeUtxo = agentUtxos.reduce((a, b) => BigInt(a.utxoEntry.amount) > BigInt(b.utxoEntry.amount) ? a : b);

  const COV_TXID = covUtxo.outpoint.transactionId;
  const COV_IDX = covUtxo.outpoint.index;
  const COV_AMT = BigInt(covUtxo.utxoEntry.amount);
  const G_SPK = covUtxo.utxoEntry.scriptPublicKey.scriptPublicKey;

  const FEE_TXID = feeUtxo.outpoint.transactionId;
  const FEE_IDX = feeUtxo.outpoint.index;
  const FEE_AMT = BigInt(feeUtxo.utxoEntry.amount);

  const OUT_AMT = COV_AMT + FEE_AMT - FEE;

  const ownerSpkObj = payToAddressScript(new Address(OWNER_ADDRESS));
  const OWNER_SPK_SCRIPT = ownerSpkObj.script;

  const redeemScript = Buffer.from(REDEEM_SCRIPT_HEX, 'hex');

  const inputs = [
    { txid: COV_TXID, idx: COV_IDX, value: COV_AMT, spk: Buffer.from(G_SPK, 'hex') },
    { txid: FEE_TXID, idx: FEE_IDX, value: FEE_AMT, spk: Buffer.from(AGENT_SPK, 'hex') },
  ];
  const outputs = [ { value: OUT_AMT, spk: Buffer.from(OWNER_SPK_SCRIPT, 'hex') } ];

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

  const selectorTrue = pd(Buffer.from([0x01]));
  const scriptSig0 = Buffer.concat([selectorTrue, pd(redeemScript)]).toString('hex');

  const privKey = new PrivateKey(AGENT_PRIV_HEX);
  const sighash1Hex = buildSighash(1).toString('hex');
  const scriptSig1 = signScriptHash(sighash1Hex, privKey);

  const txObj = {
    version: 1,
    inputs: [
      { previousOutpoint: { transactionId: COV_TXID, index: COV_IDX }, signatureScript: scriptSig0, sequence: 0, sigOpCount: 0, computeBudget: COMPUTE_BUDGET_ZK },
      { previousOutpoint: { transactionId: FEE_TXID, index: FEE_IDX }, signatureScript: scriptSig1, sequence: 0, sigOpCount: 0, computeBudget: COMPUTE_BUDGET_P2PK },
    ],
    outputs: [ { value: Number(OUT_AMT), scriptPublicKey: { script: OWNER_SPK_SCRIPT, version: 0 } } ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch(_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  try {
    await rpc.connect();
    const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
    console.log('RESULT_JSON:', JSON.stringify({ txId: r.transactionId, ownerAddr: OWNER_ADDRESS, unlockedKas: Number(OUT_AMT)/1e8, explorerUrl: 'https://kaspa.stream/tx/' + r.transactionId }));
  } catch (e) {
    console.log('RESULT_ERROR: ' + (e.message || e));
  } finally {
    await rpc.disconnect().catch(()=>{});
    process.exit(0);
  }
}

main();

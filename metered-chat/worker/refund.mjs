// refund.mjs — Covenants++ metered AI chat: CLTV timeout refund (the trustless
// clawback). Adapted from the mainnet-proven x402-kaspa/sentinel-x402/refund_x402.mjs.
//
// After a hop's deadline passes, ANYONE can trigger this — no signature exists.
// The guest's funds return to the guest's own address, split across 2 outputs.
// In production the guest app can build this tx itself (permissionless);
// in the sandbox this is the operator/demo path.
//
// Usage:
//   node refund.mjs --session=mc_xxx [--hop=<i>] --txid=<covenant outpoint txid> --idx=<vout>
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
const { RpcClient, Resolver, Address, payToAddressScript } = kaspa;

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function pd(b) { const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  if (len <= 65535) { const lb = Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]), lb, b]); }
  const lb4 = Buffer.alloc(4); lb4.writeUInt32LE(len); return Buffer.concat([Buffer.from([0x4e]), lb4, b]); }
function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }

async function main() {
  const sessionId = arg('session');
  if (!sessionId) { console.log('RESULT_ERROR: --session is required'); process.exit(1); }
  const chain = JSON.parse(fs.readFileSync(path.join(__dirname, 'out', `chain_out_${sessionId}.json`), 'utf8'));

  const hopIdx = arg('hop') !== undefined ? parseInt(arg('hop', '0')) : (chain.current_hop ?? 0);
  const hop = chain.hops[hopIdx];
  if (!hop) { console.log(`RESULT_ERROR: no hop ${hopIdx}`); process.exit(1); }

  const txid = arg('txid') || chain.hop_utxo?.txid;
  const idx = arg('idx') !== undefined ? parseInt(arg('idx', '0')) : (chain.hop_utxo?.idx ?? 0);
  if (!txid) { console.log('RESULT_ERROR: pass --txid (the covenant outpoint) and --idx'); process.exit(1); }

  const redeemScript = Buffer.from(hop.script_hex, 'hex');
  const selector = Buffer.from([0x00]); // OP_0 — ELSE branch: timeout/refund
  const scriptSig = Buffer.concat([ selector, pd(redeemScript) ]);

  const custSpk = Buffer.from(payToAddressScript(new Address(chain.customer)).script, 'hex').toString('hex');
  const REFUND_A = BigInt(hop.refund_a);
  const REFUND_B = BigInt(hop.refund_b);

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const tipInfo = await (await fetch('https://api.kaspa.org/info/blockdag')).json();
  const currentDaa = Number(tipInfo.virtualDaaScore);
  if (currentDaa < hop.deadline_daa) {
    console.log(`RESULT_ERROR: hop${hopIdx} not expired (now ${currentDaa} < deadline ${hop.deadline_daa})`);
    await rpc.disconnect().catch(() => {});
    process.exit(1);
  }

  const txObj = {
    version: 1,
    inputs: [ { previousOutpoint: { transactionId: txid, index: idx }, signatureScript: scriptSig.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 2500 } ],
    outputs: [
      { value: Number(REFUND_A), scriptPublicKey: { script: custSpk, version: 0 } },
      { value: Number(REFUND_B), scriptPublicKey: { script: custSpk, version: 0 } },
    ],
    lockTime: currentDaa, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };
  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});

  const WORKER_SECRET = process.env.METERED_WORKER_SECRET;
  if (WORKER_SECRET) {
    await fetch(process.env.METERED_REGISTER_URL ||
      'https://base44.app/api/apps/6a444b036408e68ec8d6f2a6/functions/meteredChatWorker', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ secret: WORKER_SECRET, action: 'abort', sessionId, reason: `CLTV refund hop${hopIdx}` }),
    }).catch(() => {});
  }
  console.log('RESULT_JSON:', JSON.stringify({ txId: r.transactionId, refundA: REFUND_A.toString(),
    refundB: REFUND_B.toString(), customer: chain.customer,
    explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 2000) || String(e))); process.exit(1); });

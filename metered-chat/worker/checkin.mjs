// checkin.mjs — Covenants++ metered AI chat: worker check-in on the current hop.
// Adapted from the mainnet-proven x402-kaspa/sentinel-x402/checkin_x402.mjs.
//
// One check-in = one response-bundle consumed: pays INCREMENT to the treasury,
// relocks the remainder into the next hop (or returns it to the guest on the
// final hop). Signs with the server-held XMSS witness — the guest's key never
// touches this path.
//
// Usage:
//   node checkin.mjs --session=mc_xxx [--txid=<funding or previous checkin txid>] [--idx=<vout>]
// The current hop's spendable outpoint is tracked in chain_out_<session>.json
// (set by `mark_funded.mjs` after funding, or --txid/--idx manually).
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
const { RpcClient, Resolver } = kaspa;

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
  const outDir = path.join(__dirname, 'out');
  const chainPath = path.join(outDir, `chain_out_${sessionId}.json`);
  const buildPath = path.join(outDir, `chain_build_${sessionId}.json`);
  const chain = JSON.parse(fs.readFileSync(chainPath, 'utf8'));
  const build = JSON.parse(fs.readFileSync(buildPath, 'utf8'));

  const hopIdx = chain.current_hop ?? 0;
  const hop = chain.hops[hopIdx];
  if (!hop) { console.log(`RESULT_ERROR: no hop ${hopIdx} (chain complete)`); process.exit(1); }
  const nextHop = chain.hops[hopIdx + 1] || null;
  const hopBuild = build.hops[hopIdx];

  // Spendable outpoint: tracked after funding, or passed explicitly.
  let txid = arg('txid') || chain.hop_utxo?.txid;
  let idx = arg('idx') !== undefined ? parseInt(arg('idx', '0')) : (chain.hop_utxo?.idx ?? 0);
  if (!txid) { console.log('RESULT_ERROR: no spendable outpoint known. Pass --txid/--idx or run mark_funded.mjs first.'); process.exit(1); }

  // ScriptSig: witnesses (bottom-up), checkin msg, OP_1 selector, redeem script.
  const redeemScript = Buffer.from(hop.script_hex, 'hex');
  const witnesses = hopBuild.witness_hex.map(h => pd(Buffer.from(h, 'hex'))).reverse();
  const checkinMsg = pd(Buffer.from(build.checkin_msg_hex, 'hex'));
  const selector = Buffer.from([0x51]); // OP_1 — IF-branch: check-in/pay
  const scriptSig = Buffer.concat([ ...witnesses, checkinMsg, selector, pd(redeemScript) ]);

  // The script enforces exact output amounts: [increment -> treasury,
  // remainder -> next hop (or the guest on the final hop)]. Fee is implicit
  // (input value - outputs), exactly like the mainnet-proven flow.
  const INCREMENT = BigInt(chain.increment);
  const remainder = BigInt(hop.remainder);
  const { Address, payToAddressScript } = kaspa;
  const providerSpk = Buffer.from(payToAddressScript(new Address(chain.provider)).script, 'hex').toString('hex');
  const finalSpk = Buffer.from(payToAddressScript(new Address(chain.customer)).script, 'hex').toString('hex');
  const nextSpk = nextHop ? nextHop.spk_hex : finalSpk;

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const outputs = [
    { value: Number(INCREMENT), scriptPublicKey: { script: providerSpk, version: 0 } },
    { value: Number(remainder), scriptPublicKey: { script: nextSpk, version: 0 } },
  ];

  const txObj = {
    version: 1,
    inputs: [ { previousOutpoint: { transactionId: txid, index: idx }, signatureScript: scriptSig.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 2500 } ],
    outputs,
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };
  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});

  // Track the next hop's spendable outpoint: output index 1 (the relock), or
  // none on the final hop (remainder went to the guest).
  chain.current_hop = hopIdx + 1;
  chain.hop_utxo = nextHop ? { txid: r.transactionId, idx: 1 } : null;
  chain.last_checkin_tx = r.transactionId;
  fs.writeFileSync(chainPath, JSON.stringify(chain, null, 2));

  console.log('RESULT_JSON:', JSON.stringify({ txId: r.transactionId, treasuryPaid: INCREMENT.toString(),
    remainder: remainder.toString(), wentTo: nextHop ? `hop${hopIdx + 1}` : 'customer (final)',
    currentHop: chain.current_hop,
    explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId }));

  // Settle the backend bundle meter (refills replies if bundles remain).
  const WORKER_SECRET = process.env.METERED_WORKER_SECRET;
  const REGISTER_URL = process.env.METERED_REGISTER_URL ||
    'https://base44.app/api/apps/6a444b036408e68ec8d6f2a6/functions/meteredChatWorker';
  if (WORKER_SECRET) {
    const res = await fetch(REGISTER_URL, { method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ secret: WORKER_SECRET, action: 'settle', sessionId, txId: r.transactionId }) });
    const j = await res.json().catch(() => ({}));
    console.log('SETTLE:', res.status, JSON.stringify(j));
  } else {
    console.log('(METERED_WORKER_SECRET not set — backend settle skipped)');
  }
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 2000) || String(e))); process.exit(1); });

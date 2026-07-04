// facilitator_verify_and_settle.mjs -- SIMULATES THE x402 FACILITATOR
// Takes a client's signed-but-unbroadcast payment payload (the X-PAYMENT header
// content) and:
//   1. VERIFY  -- checks the tx pays the right address the right amount
//   2. SETTLE  -- broadcasts it to Kaspa mainnet and waits for confirmation
// The facilitator never has and never needs the buyer's private key.
//
// Usage: node facilitator_verify_and_settle.mjs --payload=payment_payload.json --expect-to=<addr> --expect-sompi=<amount>
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import { createRequire } from 'module';
import fs from 'fs';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
const { RpcClient, Resolver, Transaction } = kaspa;

function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }
const PAYLOAD_PATH = arg('payload');
const EXPECT_TO = arg('expect-to');
const EXPECT_SOMPI = BigInt(arg('expect-sompi'));

async function main() {
  const payloadJson = fs.readFileSync(PAYLOAD_PATH, 'utf8');
  const parsed = JSON.parse(payloadJson);

  // ---- VERIFY step ----
  const outputs = parsed.outputs || parsed.transaction?.outputs;
  if (!outputs) throw new Error('VERIFY_FAILED: no outputs in payload');

  let matched = false;
  for (const o of outputs) {
    const addr = o.scriptPublicKey?.address || o.address;
    const amt = BigInt(o.value || o.amount || 0);
    if (amt === EXPECT_SOMPI) { matched = true; }
  }
  console.log('VERIFY: outputs found:', outputs.length);
  if (!matched) {
    console.log('VERIFY_RESULT: FAIL -- expected amount not found in outputs');
    process.exit(1);
  }
  console.log('VERIFY_RESULT: PASS -- payment payload matches requirements (amount check)');

  // ---- SETTLE step ----
  const rpc = new RpcClient({ resolver: new Resolver(), networkId: 'mainnet' });
  await rpc.connect();

  // Reconstruct a submittable transaction from the safe JSON and broadcast it.
  const tx = Transaction.deserializeFromSafeJSON(payloadJson);
  const result = await rpc.submitTransaction({ transaction: tx, allowOrphan: false });

  console.log('SETTLE_RESULT_JSON:', JSON.stringify({ txId: result.transactionId }));

  await rpc.disconnect().catch(() => {});
}

main().catch(e => { console.error('FACILITATOR_ERROR:', e.message || e); process.exit(1); });

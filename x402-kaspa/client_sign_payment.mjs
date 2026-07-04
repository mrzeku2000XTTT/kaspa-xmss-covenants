// client_sign_payment.mjs -- SIMULATES THE BUYER/CLIENT SIDE OF x402
// Builds and signs a real Kaspa payment transaction but does NOT broadcast it.
// Outputs the signed-but-unbroadcast tx as JSON -- this is the "X-PAYMENT" payload
// that would be sent in the HTTP header to the resource server / facilitator.
//
// Usage: node client_sign_payment.mjs --to=<addr> --sompi=<amount> --from-priv=<hex> --from-addr=<addr>
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import { createRequire } from 'module';
import fs from 'fs';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
const { PrivateKey, RpcClient, Resolver, createTransactions } = kaspa;

function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }
const TO = arg('to');
const SOMPI = BigInt(arg('sompi'));
const FROM_PRIV = arg('from-priv');
const FROM_ADDR = arg('from-addr');
const OUT = arg('out', 'payment_payload.json');

async function main() {
  const rpc = new RpcClient({ resolver: new Resolver(), networkId: 'mainnet' });
  await rpc.connect();

  const { entries } = await rpc.getUtxosByAddresses([FROM_ADDR]);
  if (!entries || entries.length === 0) throw new Error('No UTXOs found for buyer address');

  const priorityFee = BigInt(20000);
  const { transactions } = await createTransactions({
    entries,
    outputs: [{ address: TO, amount: SOMPI }],
    changeAddress: FROM_ADDR,
    priorityFee,
    networkId: 'mainnet',
  });

  const priv = new PrivateKey(FROM_PRIV);
  const pending = transactions[0];
  pending.sign([priv]);

  // This is the key trick: serializeToSafeJSON() gives us a fully-signed,
  // ready-to-broadcast transaction as plain JSON -- NO private key inside it.
  // This is exactly what an x402 X-PAYMENT header payload would carry.
  const payload = pending.serializeToSafeJSON();

  fs.writeFileSync(OUT, payload);
  console.log('PAYMENT_PAYLOAD_WRITTEN:', OUT);
  console.log('payload preview:', payload.slice(0, 300));

  await rpc.disconnect().catch(() => {});
}

main().catch(e => { console.error('CLIENT_SIGN_ERROR:', e.message || e); process.exit(1); });

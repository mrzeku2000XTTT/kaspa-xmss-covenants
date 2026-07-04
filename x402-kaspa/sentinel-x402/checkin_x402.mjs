// checkin_x402.mjs -- performs a "check in and pay" against hop0, releasing INCREMENT to
// the provider and relocking the remainder into hop1.
// Usage: node checkin_x402.mjs --txid=<hop0 deploy txid> --idx=0
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
const { RpcClient, Resolver } = kaspa;

function pd(b) { const len=b.length; if (len<=75) return Buffer.concat([Buffer.from([len]),b]); if (len<=255) return Buffer.concat([Buffer.from([0x4c,len]),b]); if (len<=65535) { const lb=Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]),lb,b]); } const lb4=Buffer.alloc(4); lb4.writeUInt32LE(len); return Buffer.concat([Buffer.from([0x4e]),lb4,b]); }
function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }

async function main() {
  const build = JSON.parse(fs.readFileSync('sentinel_x402_build.json', 'utf8'));
  const deployed = JSON.parse(fs.readFileSync('scripts_out_x402.json', 'utf8'));
  const COV_TXID = arg('txid');
  const COV_IDX = parseInt(arg('idx', '0'));

  const hop0 = deployed.hop0;
  const hop1 = deployed.hop1;
  const redeemScript = Buffer.from(hop0.script_hex, 'hex');
  const witnesses = build.witness_checkin_leaf0_hex.map(h => pd(Buffer.from(h, 'hex'))).reverse();
  const checkinMsg = pd(Buffer.from(build.checkin_msg_hex, 'hex'));
  const selector = Buffer.from([0x51]); // OP_1 -- MINIMALDATA true (IF-branch: check-in/pay)
  const scriptSig = Buffer.concat([ ...witnesses, checkinMsg, selector, pd(redeemScript) ]);

  const providerSpk = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
  const INCREMENT = BigInt(deployed.increment);
  const V1 = BigInt(hop1.value);

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const txObj = {
    version: 1,
    inputs: [ { previousOutpoint: { transactionId: COV_TXID, index: COV_IDX }, signatureScript: scriptSig.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 2500 } ],
    outputs: [
      { value: Number(INCREMENT), scriptPublicKey: { script: providerSpk, version: 0 } },
      { value: Number(V1), scriptPublicKey: { script: hop1.spk_hex, version: 0 } },
    ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };
  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});
  console.log('RESULT_JSON:', JSON.stringify({ txId: r.transactionId, providerPaid: INCREMENT.toString(), lockedNext: V1.toString(), hop1Addr: hop1.addr, explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId }));
}
main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0,2000) || String(e))); process.exit(1); });

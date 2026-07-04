// checkin_sentinel_generic.mjs -- performs a proof-of-life "check-in" (renewal)
// against a Sentinel hop, using the customer's kit_deployed.json.
// Usage: node checkin_sentinel_generic.mjs --deployed=kit_deployed.json --hop=0 --txid=<current utxo txid> --idx=0
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('../node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('../node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
const { RpcClient, Resolver } = kaspa;

function pd(b) { const len=b.length; if (len<=75) return Buffer.concat([Buffer.from([len]),b]); if (len<=255) return Buffer.concat([Buffer.from([0x4c,len]),b]); if (len<=65535) { const lb=Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]),lb,b]); } const lb4=Buffer.alloc(4); lb4.writeUInt32LE(len); return Buffer.concat([Buffer.from([0x4e]),lb4,b]); }
function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }

async function main() {
  const deployed = JSON.parse(fs.readFileSync(arg('deployed'), 'utf8'));
  const kit = JSON.parse(fs.readFileSync(arg('kit'), 'utf8'));
  const hopIdx = parseInt(arg('hop'));
  const COV_TXID = arg('txid');
  const COV_IDX = parseInt(arg('idx', '0'));

  const hop = deployed.hops[hopIdx];
  const nextHop = deployed.hops[hopIdx + 1];
  if (!nextHop) throw new Error('This is the last hop -- no further check-ins possible, generate a new kit and re-deploy before this runs out.');

  const leaf = kit.leaves[hopIdx];
  const redeemScript = Buffer.from(hop.script_hex, 'hex');
  const witnesses = leaf.witness_hex.map(h => pd(Buffer.from(h, 'hex'))).reverse();
  const renewMsg = pd(Buffer.from(deployed.renew_msg_hex, 'hex'));
  const selector = Buffer.from([0x51]); // OP_1 -- MINIMALDATA true
  const scriptSig0 = Buffer.concat([ ...witnesses, renewMsg, selector, pd(redeemScript) ]);
  const OUT_AMT = BigInt(nextHop.value);

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();
  const txObj = {
    version: 1,
    inputs: [ { previousOutpoint: { transactionId: COV_TXID, index: COV_IDX }, signatureScript: scriptSig0.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 2500 } ],
    outputs: [ { value: Number(OUT_AMT), scriptPublicKey: { script: Buffer.from(nextHop.spk_hex,'hex').toString('hex'), version: 0 } } ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };
  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});
  console.log('RESULT_JSON:', JSON.stringify({ txId: r.transactionId, nextHopAddr: nextHop.addr, explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId }));
}
main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0,1500) || String(e))); process.exit(1); });

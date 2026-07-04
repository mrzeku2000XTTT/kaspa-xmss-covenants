// refund_x402.mjs -- triggers the ELSE/timeout branch on hop1: permissionless, zero-signature
// refund of the remaining locked balance back to the CUSTOMER, split across 2 outputs.
// Usage: node refund_x402.mjs --txid=<hop0->hop1 checkin txid> --idx=1
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
const { RpcClient, Resolver, Address, payToAddressScript } = kaspa;

function pd(b) { const len=b.length; if (len<=75) return Buffer.concat([Buffer.from([len]),b]); if (len<=255) return Buffer.concat([Buffer.from([0x4c,len]),b]); if (len<=65535) { const lb=Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]),lb,b]); } const lb4=Buffer.alloc(4); lb4.writeUInt32LE(len); return Buffer.concat([Buffer.from([0x4e]),lb4,b]); }
function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }

async function main() {
  const deployed = JSON.parse(fs.readFileSync('scripts_out_x402.json', 'utf8'));
  const COV_TXID = arg('txid');
  const COV_IDX = parseInt(arg('idx', '1'));

  const hop1 = deployed.hop1;
  const redeemScript = Buffer.from(hop1.script_hex, 'hex');
  const selector = Buffer.from([0x00]); // OP_0 opcode -- pushes empty array (falsy), MINIMALDATA-correct
  const scriptSig = Buffer.concat([ selector, pd(redeemScript) ]);

  const custSpk = Buffer.from(payToAddressScript(new Address(deployed.customer.addr)).script, 'hex').toString('hex');
  const REFUND_A = BigInt(deployed.refund_a);
  const REFUND_B = BigInt(deployed.refund_b);

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const tipInfo = await (await fetch('https://api.kaspa.org/info/blockdag')).json();
  const currentDaa = Number(tipInfo.virtualDaaScore);

  const txObj = {
    version: 1,
    inputs: [ { previousOutpoint: { transactionId: COV_TXID, index: COV_IDX }, signatureScript: scriptSig.toString('hex'), sequence: 0, sigOpCount: 0, computeBudget: 2500 } ],
    outputs: [
      { value: Number(REFUND_A), scriptPublicKey: { script: custSpk, version: 0 } },
      { value: Number(REFUND_B), scriptPublicKey: { script: custSpk, version: 0 } },
    ],
    lockTime: currentDaa, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };
  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});
  console.log('RESULT_JSON:', JSON.stringify({ txId: r.transactionId, refundA: REFUND_A.toString(), refundB: REFUND_B.toString(), customerAddr: deployed.customer.addr, explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId }));
}
main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0,2000) || String(e))); process.exit(1); });

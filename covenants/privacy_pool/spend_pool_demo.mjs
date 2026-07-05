/**
 * spend_pool_demo.mjs — Spend the Privacy Pool covenant via the ZK membership branch.
 * Reveals the redeem script (root, nullifierHash, Groth16 proof, VK) — consensus verifies
 * the proof via OpZkPrecompile. Observer sees: "someone proved membership in the 4-leaf
 * set and nullifier X hasn't been used" — NOT which of the 4 deposits was spent.
 */
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import fs from 'fs';
import crypto from 'crypto';

const { RpcClient, Resolver, PrivateKey, payToAddressScript, Address } = kaspa;

function pd(b) {
  const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  const lb = Buffer.alloc(2); lb.writeUInt16LE(len);
  return Buffer.concat([Buffer.from([0x4d]), lb, b]);
}

const REDEEM_SCRIPT_HEX = fs.readFileSync('/app/privacy_pool/redeem_script.hex', 'utf8').trim();
const redeemScript = Buffer.from(REDEEM_SCRIPT_HEX, 'hex');

const COV_TXID = 'da609f48fcdb22e50747ce120281959356cb454019810228914c184604c1e3a8';
const COV_IDX = 0;
const COV_AMT = 50_000_000n;

// A brand-new, never-before-seen destination address — proving the withdrawal is unlinkable
// to any specific one of the 4 deposits (that's the whole point of the pool).
const freshPriv = crypto.randomBytes(32);
const freshPk = new PrivateKey(freshPriv.toString('hex'));
const freshAddr = freshPk.toKeypair().toAddress('mainnet').toString();

const FEE = 22_000_000n; // ~0.03 KAS, small ~570 byte script
const OUT_AMT = COV_AMT - FEE;

async function main() {
  // scriptSig = OP_1 (take the ZK/IF branch) + push(redeemScript)
  const scriptSig = Buffer.concat([Buffer.from([0x51]), pd(redeemScript)]);

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const destSpk = payToAddressScript(new Address(freshAddr));

  const txObj = {
    version: 1,
    inputs: [{
      previousOutpoint: { transactionId: COV_TXID, index: COV_IDX },
      signatureScript: scriptSig.toString('hex'),
      sequence: 0, sigOpCount: 0, computeBudget: 2000,
    }],
    outputs: [
      { value: Number(OUT_AMT), scriptPublicKey: { script: destSpk.script, version: 0 } },
    ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  console.log('Fresh withdrawal address:', freshAddr);
  console.log('scriptSig length:', scriptSig.length);

  const DRY_RUN = process.argv.includes('--dry-run');
  if (DRY_RUN) { console.log('DRY RUN'); await rpc.disconnect().catch(()=>{}); return; }

  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});

  console.log('RESULT_JSON:', JSON.stringify({
    txId: r.transactionId,
    withdrawnTo: freshAddr,
    amountKas: Number(OUT_AMT) / 1e8,
    explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId,
  }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 800) || String(e))); process.exit(1); });

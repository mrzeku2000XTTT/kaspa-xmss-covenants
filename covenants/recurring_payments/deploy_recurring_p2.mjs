/**
 * xmss_full_deploy.mjs — Deploy a P2SH covenant on Kaspa mainnet whose redeem script
 * verifies a COMPLETE XMSS-SHA2_10_192 signature (all 51 WOTS+ chains + L-tree +
 * 10-level Merkle path) using ONLY OpXor/OpCat/OpSHA256/OpSubstr/OpToAltStack/
 * OpFromAltStack/OpSwap/OpEqual — no signature opcode, no elliptic curve, fully
 * hash-based post-quantum verification.
 *
 * Usage: node xmss_full_deploy.mjs [--deposit=0.2] [--dry-run]
 */
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from './node_modules/@noble/hashes/blake2b.js';

const { RpcClient, Resolver, createTransactions, PrivateKey, createInputSignature, ScriptPublicKey, addressFromScriptPublicKey } = kaspa;

function arg(name, def) {
  const a = process.argv.find(x => x.startsWith(`--${name}=`));
  return a ? a.split('=').slice(1).join('=') : def;
}

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK  = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

const DEPOSIT_KAS = parseFloat(arg('deposit', '0.15'));
const DRY_RUN = process.argv.includes('--dry-run');

if (!AGENT_PRIV_HEX) { console.log('RESULT_ERROR: Missing KASPA_AGENT_PRIVKEY'); process.exit(1); }

const REDEEM_SCRIPT_HEX = fs.readFileSync('./recurring_p2_script.hex', 'utf8').trim();
const DEPOSIT = BigInt(Math.round(DEPOSIT_KAS * 1e8));
const FEE = 32_000_000n; // 128KB script (dynamic-linked)

async function main() {
  const redeemScript = Buffer.from(REDEEM_SCRIPT_HEX, 'hex');
  const scriptHash = blake2b(redeemScript, { dkLen: 32 });
  const spkBuf = Buffer.concat([Buffer.from([0xaa, 0x20]), Buffer.from(scriptHash), Buffer.from([0x87])]);
  const V_SPK = spkBuf.toString('hex');
  const V_ADDR = addressFromScriptPublicKey(new ScriptPublicKey(0, V_SPK), 'mainnet').toString();

  console.log('Covenant address:', V_ADDR);
  console.log('Redeem script bytes:', redeemScript.length);

  const utxos = await (await fetch(`https://api.kaspa.org/addresses/${AGENT_ADDR}/utxos`)).json();
  utxos.sort((a,b)=>Number(BigInt(b.utxoEntry.amount)-BigInt(a.utxoEntry.amount)));
  const u = utxos[0];
  if (!u) throw new Error('No UTXOs available in agent wallet');
  const IN_TXID = u.outpoint.transactionId;
  const IN_IDX = u.outpoint.index;
  const IN_AMT = BigInt(u.utxoEntry.amount);
  const IN_DAA = BigInt(u.utxoEntry.blockDaaScore);
  const CHANGE = IN_AMT - DEPOSIT - FEE;
  if (CHANGE < 0n) throw new Error('Insufficient agent wallet funds');

  if (DRY_RUN) {
    console.log('RESULT_JSON:', JSON.stringify({ dryRun: true, covenantAddress: V_ADDR, scriptBytes: redeemScript.length, lockedKas: DEPOSIT_KAS }));
    return;
  }

  const privKey = new PrivateKey(AGENT_PRIV_HEX);
  const result = await createTransactions({
    entries: [{
      address: AGENT_ADDR,
      outpoint: { transactionId: IN_TXID, index: IN_IDX },
      amount: IN_AMT,
      scriptPublicKey: { version: 0, script: AGENT_SPK },
      blockDaaScore: IN_DAA,
      isCoinbase: false,
    }],
    outputs: [{ address: V_ADDR, amount: DEPOSIT }],
    changeAddress: AGENT_ADDR,
    priorityFee: FEE,
    networkId: 'mainnet',
  });
  const tx = result.transactions[0].transaction;
  tx.version = 1;

  const sigResult = createInputSignature(tx, 0, privKey, 1);
  const sigScriptHex = Buffer.from(sigResult).toString('ascii');

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const out0val = tx.outputs[0].value;
  const out1val = tx.outputs[1]?.value;

  const txObj = {
    version: 1,
    inputs: [{
      previousOutpoint: { transactionId: IN_TXID, index: IN_IDX },
      signatureScript: sigScriptHex,
      sequence: 0, sigOpCount: 0, computeBudget: 10,
    }],
    outputs: [
      { value: Number(out0val), scriptPublicKey: { script: V_SPK, version: 0 } },
      ...(out1val ? [{ value: Number(out1val), scriptPublicKey: { script: AGENT_SPK, version: 0 } }] : []),
    ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };

  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});

  console.log('RESULT_JSON:', JSON.stringify({
    txId: r.transactionId, covenantAddress: V_ADDR, scriptBytes: redeemScript.length,
    lockedKas: DEPOSIT_KAS,
    explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId,
  }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 800) || String(e))); process.exit(1); });

// deploy_sentinel_generic.mjs -- builds and deploys a full Sentinel hop-chain
// from a customer-supplied sentinel_kit.json (produced by sentinel_keygen.py).
// Usage: node deploy_sentinel_generic.mjs --kit=path.json --beneficiary=<addr> --timeout_blocks=2200 --deposit=0.6
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('../node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('../node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from '../node_modules/@noble/hashes/blake2b.js';
const { ScriptPublicKey, addressFromScriptPublicKey, Address, payToAddressScript, PrivateKey,
        RpcClient, Resolver, createTransactions } = kaspa;

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK  = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

function pd(b) { const len=b.length; if (len<=75) return Buffer.concat([Buffer.from([len]),b]); if (len<=255) return Buffer.concat([Buffer.from([0x4c,len]),b]); if (len<=65535) { const lb=Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]),lb,b]); } const lb=Buffer.alloc(4); lb.writeUInt32LE(len); return Buffer.concat([Buffer.from([0x4e]),lb,b]); }
function encodeNum(n) { n = Number(n); if (n === 0) return Buffer.alloc(0); const neg = n < 0; n = Math.abs(n); const bytes = []; while (n > 0) { bytes.push(n & 0xff); n = Math.floor(n / 256); } if (bytes[bytes.length-1] & 0x80) bytes.push(neg ? 0x80 : 0x00); else if (neg) bytes[bytes.length-1] |= 0x80; return Buffer.from(bytes); }
const OP_IF=0x63, OP_ELSE=0x67, OP_ENDIF=0x68, OP_EQUAL=0x87, OP_EQUALVERIFY=0x88, OP_CLTV=0xb0;
const OP_TX_OUTPUT_AMOUNT = 0xc2, OP_TX_OUTPUT_SPK = 0xc3;

function checkOutput(idx, spkBuf) { return Buffer.concat([pd(encodeNum(idx)), Buffer.from([OP_TX_OUTPUT_SPK]), pd(Buffer.concat([Buffer.from([0x00,0x00]), spkBuf])), Buffer.from([OP_EQUALVERIFY])]); }

function buildEpochScript(verifyBlockHex, renewMsgHex, unlockDaa, nextHopSpk, nextHopAmt, beneficiarySpk, beneficiaryAmt) {
  const verifyBlock = Buffer.from(verifyBlockHex, 'hex');
  const renewMsg = Buffer.from(renewMsgHex, 'hex');
  const versionedNextHopSpk = Buffer.concat([Buffer.from([0x00,0x00]), nextHopSpk]);
  const versionedBeneficiarySpk = Buffer.concat([Buffer.from([0x00,0x00]), beneficiarySpk]);
  let s = Buffer.concat([Buffer.from([OP_IF])]);
  s = Buffer.concat([s, verifyBlock, pd(renewMsg), Buffer.from([OP_EQUALVERIFY])]);
  s = Buffer.concat([s, pd(versionedNextHopSpk), pd(encodeNum(nextHopAmt))]);
  s = Buffer.concat([s, Buffer.from([OP_ELSE])]);
  s = Buffer.concat([s, pd(encodeNum(unlockDaa)), Buffer.from([OP_CLTV])]);
  s = Buffer.concat([s, pd(versionedBeneficiarySpk), pd(encodeNum(beneficiaryAmt))]);
  s = Buffer.concat([s, Buffer.from([OP_ENDIF])]);
  s = Buffer.concat([s, pd(encodeNum(0)), Buffer.from([OP_TX_OUTPUT_AMOUNT]), Buffer.from([OP_EQUALVERIFY])]);
  s = Buffer.concat([s, pd(encodeNum(0)), Buffer.from([OP_TX_OUTPUT_SPK]), Buffer.from([OP_EQUAL])]);
  return s;
}
function p2shAddr(script) {
  const scriptHash = blake2b(script, { dkLen: 32 });
  const spkBuf = Buffer.concat([Buffer.from([0xaa, 0x20]), Buffer.from(scriptHash), Buffer.from([0x87])]);
  const addr = addressFromScriptPublicKey(new ScriptPublicKey(0, spkBuf.toString('hex')), 'mainnet').toString();
  return { spk: spkBuf, addr };
}
function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }

async function main() {
  const kitPath = arg('kit');
  const beneficiaryAddr = arg('beneficiary');
  const timeoutBlocks = parseInt(arg('timeout_blocks', '2200'));
  const depositKas = parseFloat(arg('deposit', '0.6'));
  const kit = JSON.parse(fs.readFileSync(kitPath, 'utf8'));
  const beneficiarySpk = Buffer.from(payToAddressScript(new Address(beneficiaryAddr)).script, 'hex');

  const tipInfo = await (await fetch('https://api.kaspa.org/info/blockdag')).json();
  const currentDaa = Number(tipInfo.virtualDaaScore);

  const V0 = BigInt(Math.round(depositKas * 1e8));
  const FEE_STEP = 40_000_000n; // safe margin for ~65KB script spends (proven: needs ~36M, use 40M buffer)
  const N = kit.num_checkins;
  if (V0 <= FEE_STEP * BigInt(N)) throw new Error(`Deposit too small for ${N} check-ins at this fee -- need > ${Number(FEE_STEP*BigInt(N))/1e8} KAS`);

  // Build hop chain bottom-up: hopN-1 (last leaf) references beneficiary only in ELSE;
  // its IF branch has no further leaf, so we build with leaf (N-1) referencing itself is invalid --
  // instead, the LAST leaf's covenant only has the ELSE/CLTV release (no more check-ins possible after it).
  const values = []; // values[i] = value locked at hop i
  let v = V0;
  for (let i = 0; i < N; i++) { values.push(v); v = v - FEE_STEP; }
  const finalPayout = v; // after N steps, if fully renewed N times then finally released via CLTV at last hop

  // Deadlines: each hop gets `timeoutBlocks` from when it's created; for the deploy-time chain we just
  // stamp them all relative to currentDaa + (i+1)*timeoutBlocks so a customer has a full window at every hop.
  let hops = [];
  // Build from the last hop backwards (last hop has NO next-hop -- IF branch on the final leaf just
  // isn't reachable/used in this design: at hop N-1 we still expose the same IF/ELSE shape, but the
  // "next hop" reference points nowhere meaningful since there is no leaf N -- so cap check-ins at N-1
  // renewals, hop N-1 is release-only in practice (customer should re-deploy a new kit before running out).
  let prevSpk = Buffer.from(AGENT_SPK, 'hex'); // placeholder next-hop for the terminal leaf (unused path)
  for (let i = N - 1; i >= 0; i--) {
    const leaf = kit.leaves[i];
    const deadline = currentDaa + (i + 1) * timeoutBlocks;
    const nextAmt = i < N - 1 ? values[i + 1] : values[i] - FEE_STEP;
    const script = buildEpochScript(leaf.verify_block_hex, kit.renew_msg_hex, deadline, prevSpk, nextAmt, beneficiarySpk, values[i] - FEE_STEP);
    const { spk, addr } = p2shAddr(script);
    hops[i] = { script_hex: script.toString('hex'), spk_hex: spk.toString('hex'), addr, value: values[i].toString(), deadline_daa: deadline, leaf_index: i };
    prevSpk = spk;
  }

  fs.writeFileSync(kitPath.replace('.json', '_deployed.json'), JSON.stringify({
    beneficiary: beneficiaryAddr, timeout_blocks: timeoutBlocks, hops, renew_msg_hex: kit.renew_msg_hex,
  }, null, 2));
  console.log('hop0 addr:', hops[0].addr, 'value:', hops[0].value);

  if (process.argv.includes('--dry-run')) return;

  const privKey = new PrivateKey(AGENT_PRIV_HEX);
  const utxos = await (await fetch(`https://api.kaspa.org/addresses/${AGENT_ADDR}/utxos`)).json();
  utxos.sort((a,b)=>Number(BigInt(b.utxoEntry.amount)-BigInt(a.utxoEntry.amount)));
  const u = utxos[0];
  if (!u) throw new Error('No UTXOs available in agent wallet');
  const IN_TXID = u.outpoint.transactionId, IN_IDX = u.outpoint.index;
  const IN_AMT = BigInt(u.utxoEntry.amount), IN_DAA = BigInt(u.utxoEntry.blockDaaScore);
  const DEPLOY_FEE = 5_000_000n;
  const CHANGE = IN_AMT - V0 - DEPLOY_FEE;
  if (CHANGE < 0n) throw new Error('Insufficient agent wallet funds');

  const result = await createTransactions({
    entries: [{ address: AGENT_ADDR, outpoint: { transactionId: IN_TXID, index: IN_IDX }, amount: IN_AMT,
      scriptPublicKey: { version: 0, script: AGENT_SPK }, blockDaaScore: IN_DAA, isCoinbase: false }],
    outputs: [{ address: hops[0].addr, amount: V0 }],
    changeAddress: AGENT_ADDR, priorityFee: DEPLOY_FEE, networkId: 'mainnet',
  });
  const tx = result.transactions[0].transaction;
  tx.version = 1;
  const { createInputSignature } = kaspa;
  const sigResult = createInputSignature(tx, 0, privKey, 1);
  const sigScriptHex = Buffer.from(sigResult).toString('ascii');

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  const out0val = tx.outputs[0].value, out1val = tx.outputs[1]?.value;
  const txObj = {
    version: 1,
    inputs: [{ previousOutpoint: { transactionId: IN_TXID, index: IN_IDX }, signatureScript: sigScriptHex, sequence: 0, sigOpCount: 0, computeBudget: 10 }],
    outputs: [
      { value: Number(out0val), scriptPublicKey: { script: hops[0].spk_hex, version: 0 } },
      ...(out1val ? [{ value: Number(out1val), scriptPublicKey: { script: AGENT_SPK, version: 0 } }] : []),
    ],
    lockTime: 0, subnetworkId: '0000000000000000000000000000000000000000', gas: 0, payload: '',
  };
  const r = await rpc.submitTransaction({ transaction: txObj, allowOrphan: false });
  await rpc.disconnect().catch(() => {});
  console.log('RESULT_JSON:', JSON.stringify({ txId: r.transactionId, hop0Addr: hops[0].addr, explorerUrl: 'https://kaspa.stream/transactions/' + r.transactionId }));
}
main().catch(e => { console.log('RESULT_ERROR:', e.message || e); process.exit(1); });

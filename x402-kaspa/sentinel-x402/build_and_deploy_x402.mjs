// build_and_deploy_x402.mjs — builds both epoch scripts bottom-up (2-output enforcement:
// output0 = value released this tx, output1 = value that stays in motion), computes P2SH
// addresses, deploys hop0.
//
// IF (check-in/pay):  output0 -> PROVIDER address, amount = INCREMENT
//                      output1 -> next hop covenant address, amount = remainder
// ELSE (timeout/refund): output0 -> CUSTOMER address, amount = refund part A
//                        output1 -> CUSTOMER address (same), amount = refund part B
//                        (split into 2 outputs purely so output count matches the IF branch --
//                         proven necessity from Stag Hunt: unconditional checks after ENDIF
//                         must see the same shape regardless of which branch executed)
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';
import crypto from 'crypto';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from './node_modules/@noble/hashes/blake2b.js';
const { ScriptPublicKey, addressFromScriptPublicKey, Address, payToAddressScript, PrivateKey,
        RpcClient, Resolver, createTransactions } = kaspa;

const AGENT_PRIV_HEX = process.env.KASPA_AGENT_PRIVKEY;
const AGENT_SPK  = '206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ac';
const AGENT_ADDR = 'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

function pd(b) {
  const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  if (len <= 65535) { const lb = Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]), lb, b]); }
  const lb = Buffer.alloc(4); lb.writeUInt32LE(len);
  return Buffer.concat([Buffer.from([0x4e]), lb, b]);
}
function encodeNum(n) {
  n = Number(n);
  if (n === 0) return Buffer.alloc(0);
  const neg = n < 0; n = Math.abs(n);
  const bytes = [];
  while (n > 0) { bytes.push(n & 0xff); n = Math.floor(n / 256); }
  if (bytes[bytes.length-1] & 0x80) bytes.push(neg ? 0x80 : 0x00);
  else if (neg) bytes[bytes.length-1] |= 0x80;
  return Buffer.from(bytes);
}
const OP_IF=0x63, OP_ELSE=0x67, OP_ENDIF=0x68, OP_EQUAL=0x87, OP_EQUALVERIFY=0x88, OP_CLTV=0xb0;
const OP_TX_OUTPUT_AMOUNT = 0xc2, OP_TX_OUTPUT_SPK = 0xc3;

function versioned(spk) { return Buffer.concat([Buffer.from([0x00,0x00]), spk]); }

// Push order per branch: spk0, amt0, spk1, amt1 (amt1 ends on top).
// Unconditional consumption after ENDIF must be LIFO: amt1, spk1, amt0, spk0(final).
function buildEpochScriptX402(verifyBlockHex, checkinMsgHex, unlockDaa,
                               providerSpk, incrementAmt, nextHopSpk, remainderAmt,
                               customerSpk, refundA, refundB) {
  const verifyBlock = Buffer.from(verifyBlockHex, 'hex');
  const checkinMsg = Buffer.from(checkinMsgHex, 'hex');

  let s = Buffer.concat([Buffer.from([OP_IF])]);
  s = Buffer.concat([s, verifyBlock, pd(checkinMsg), Buffer.from([OP_EQUALVERIFY])]);
  s = Buffer.concat([s, pd(versioned(providerSpk)), pd(encodeNum(incrementAmt))]);
  s = Buffer.concat([s, pd(versioned(nextHopSpk)), pd(encodeNum(remainderAmt))]);
  s = Buffer.concat([s, Buffer.from([OP_ELSE])]);
  s = Buffer.concat([s, pd(encodeNum(unlockDaa)), Buffer.from([OP_CLTV])]); // pops locktime, no DROP (Kaspa quirk)
  s = Buffer.concat([s, pd(versioned(customerSpk)), pd(encodeNum(refundA))]);
  s = Buffer.concat([s, pd(versioned(customerSpk)), pd(encodeNum(refundB))]);
  s = Buffer.concat([s, Buffer.from([OP_ENDIF])]);

  // unconditional enforcement -- identical shape regardless of branch taken
  s = Buffer.concat([s, pd(encodeNum(1)), Buffer.from([OP_TX_OUTPUT_AMOUNT]), Buffer.from([OP_EQUALVERIFY])]); // consumes amt1
  s = Buffer.concat([s, pd(encodeNum(1)), Buffer.from([OP_TX_OUTPUT_SPK]),    Buffer.from([OP_EQUALVERIFY])]); // consumes spk1
  s = Buffer.concat([s, pd(encodeNum(0)), Buffer.from([OP_TX_OUTPUT_AMOUNT]), Buffer.from([OP_EQUALVERIFY])]); // consumes amt0
  s = Buffer.concat([s, pd(encodeNum(0)), Buffer.from([OP_TX_OUTPUT_SPK]),    Buffer.from([OP_EQUAL])]);       // consumes spk0 -> final bool
  return s;
}

function p2shAddr(script) {
  const scriptHash = blake2b(script, { dkLen: 32 });
  const spkBuf = Buffer.concat([Buffer.from([0xaa, 0x20]), Buffer.from(scriptHash), Buffer.from([0x87])]);
  const addr = addressFromScriptPublicKey(new ScriptPublicKey(0, spkBuf.toString('hex')), 'mainnet').toString();
  return { spk: spkBuf, addr };
}

async function main() {
  const build = JSON.parse(fs.readFileSync('sentinel_x402_build.json', 'utf8'));

  // PROVIDER = the agent's own wallet, honestly (the agent IS the API operator in this demo).
  const providerSpk = Buffer.from(AGENT_SPK, 'hex');
  const providerAddr = AGENT_ADDR;

  // CUSTOMER = a fresh, independent throwaway keypair -- represents a real customer's own wallet.
  const custPrivHex = crypto.randomBytes(32).toString('hex');
  const custPriv = new PrivateKey(custPrivHex);
  const custAddr = custPriv.toPublicKey().toAddress('mainnet').toString();
  const custSpk = Buffer.from(payToAddressScript(new Address(custAddr)).script, 'hex');

  const tipInfo = await (await fetch('https://api.kaspa.org/info/blockdag')).json();
  const currentDaa = Number(tipInfo.virtualDaaScore);
  console.log('Current DAA:', currentDaa);

  const INCREMENT = 30_000_000n;       // 0.3 KAS paid to provider per check-in
  const FEE_CHECKIN = 40_000_000n;     // fee to move hop0 -> hop1 (65KB script margin)
  const FEE_REFUND = 40_000_000n;      // fee for the timeout/refund spend
  const V1 = 150_000_000n;             // hop1 locked value (1.5 KAS) -- must be big enough to split into 2 refund outputs later
  const V0 = INCREMENT + FEE_CHECKIN + V1; // hop0 deposit

  const REFUND_TOTAL = V1 - FEE_REFUND;
  const REFUND_A = REFUND_TOTAL / 2n;
  const REFUND_B = REFUND_TOTAL - REFUND_A;

  const DEADLINE_HOP0 = currentDaa + 2200;       // ~5 min window to check in
  const DEADLINE_HOP1 = currentDaa + 2200 + 350; // short window after check-in so we can let it lapse quickly for the refund proof

  // --- hop1 script: its IF-branch (further check-in) is a placeholder/unused in this proof;
  // its ELSE-branch (refund) is the one we'll actually trigger. ---
  const hop1Script = buildEpochScriptX402(
    build.verify_block_leaf1_hex, build.checkin_msg_hex, DEADLINE_HOP1,
    providerSpk, INCREMENT, providerSpk, 1n, // placeholder next-hop (unused -- never taken in this proof)
    custSpk, REFUND_A, REFUND_B
  );
  const { spk: hop1Spk, addr: hop1Addr } = p2shAddr(hop1Script);
  console.log('hop1 script bytes:', hop1Script.length, 'hop1 addr:', hop1Addr);

  // --- hop0 script: check-in pays provider + relocks remainder into hop1; timeout refunds customer ---
  const hop0Script = buildEpochScriptX402(
    build.verify_block_leaf0_hex, build.checkin_msg_hex, DEADLINE_HOP0,
    providerSpk, INCREMENT, hop1Spk, V1,
    custSpk, (V0 - FEE_REFUND) / 2n, (V0 - FEE_REFUND) - (V0 - FEE_REFUND) / 2n
  );
  const { spk: hop0Spk, addr: hop0Addr } = p2shAddr(hop0Script);
  console.log('hop0 script bytes:', hop0Script.length, 'hop0 addr:', hop0Addr);

  const scriptsOut = {
    provider: { addr: providerAddr },
    customer: { addr: custAddr, privHex: custPrivHex },
    hop0: { script_hex: hop0Script.toString('hex'), spk_hex: hop0Spk.toString('hex'), addr: hop0Addr, value: V0.toString(), deadline_daa: DEADLINE_HOP0 },
    hop1: { script_hex: hop1Script.toString('hex'), spk_hex: hop1Spk.toString('hex'), addr: hop1Addr, value: V1.toString(), deadline_daa: DEADLINE_HOP1 },
    increment: INCREMENT.toString(), fee_checkin: FEE_CHECKIN.toString(), fee_refund: FEE_REFUND.toString(),
    refund_a: REFUND_A.toString(), refund_b: REFUND_B.toString(),
  };
  fs.writeFileSync('scripts_out_x402.json', JSON.stringify(scriptsOut, null, 2));
  console.log('Written scripts_out_x402.json');
  console.log('V0 (hop0 deposit):', V0.toString(), 'sompi =', Number(V0)/1e8, 'KAS');

  if (process.argv.includes('--dry-run')) return;

  // --- Deploy hop0 ---
  const privKey = new PrivateKey(AGENT_PRIV_HEX);
  const utxos = await (await fetch(`https://api.kaspa.org/addresses/${AGENT_ADDR}/utxos`)).json();
  utxos.sort((a,b)=>Number(BigInt(b.utxoEntry.amount)-BigInt(a.utxoEntry.amount)));
  if (!utxos.length) throw new Error('No UTXOs available in agent wallet');
  const totalAvail = utxos.reduce((s,u)=>s+BigInt(u.utxoEntry.amount), 0n);
  const DEPLOY_FEE = 5_000_000n;
  if (totalAvail < V0 + DEPLOY_FEE) throw new Error('Insufficient agent wallet funds');

  const entries = utxos.map(u => ({
    address: AGENT_ADDR,
    outpoint: { transactionId: u.outpoint.transactionId, index: u.outpoint.index },
    amount: BigInt(u.utxoEntry.amount),
    scriptPublicKey: { version: 0, script: AGENT_SPK },
    blockDaaScore: BigInt(u.utxoEntry.blockDaaScore),
    isCoinbase: false,
  }));

  const result = await createTransactions({
    entries,
    outputs: [{ address: hop0Addr, amount: V0 }],
    changeAddress: AGENT_ADDR, priorityFee: DEPLOY_FEE, networkId: 'mainnet',
  });
  const tx = result.transactions[0].transaction;
  tx.version = 1;

  const resolver = new Resolver();
  let nodeUrl;
  try { nodeUrl = await resolver.getUrl('borsh', 'mainnet'); } catch (_) { nodeUrl = 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh'; }
  const rpc = new RpcClient({ url: nodeUrl, networkId: 'mainnet' });
  await rpc.connect();

  result.transactions[0].sign([privKey]);
  const txId = await result.transactions[0].submit(rpc);
  await rpc.disconnect().catch(() => {});

  scriptsOut.deploy_tx_id = txId;
  fs.writeFileSync('scripts_out_x402.json', JSON.stringify(scriptsOut, null, 2));
  console.log('RESULT_JSON:', JSON.stringify({ txId, hop0Addr, explorerUrl: 'https://kaspa.stream/transactions/' + txId }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0,1500) || String(e))); process.exit(1); });

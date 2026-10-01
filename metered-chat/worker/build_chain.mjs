// build_chain.mjs — Covenants++ metered AI chat: build the per-customer
// sentinel-x402 covenant chain (adapted from the mainnet-proven
// x402-kaspa/sentinel-x402/build_and_deploy_x402.mjs).
//
// KEY DIFFERENCE from the proof: NO auto-deploy. The GUEST funds hop0 via a
// Scorpion-signed PSKT (their only crypto action). This script only builds
// scripts/addresses and registers the chain with the backend.
//
// The final hop's IF branch returns the remainder to the CUSTOMER (same
// 2-output shape, no burn) — every earlier hop relocks into the next covenant.
// CLTV refund branch on every hop → guest clawback is trustless.
//
// Usage:
//   node build_chain.mjs --session=mc_xxx --customer=kaspa:qy... [--epochs=3]
//                        [--replies=10] [--increment-sompi=30000000]
//                        [--epoch-days=7] [--register]
import pkg from 'websocket';
const { w3cwebsocket } = pkg;
globalThis.WebSocket = w3cwebsocket;
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { execFileSync } from 'child_process';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const kaspa = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js');
kaspa.initSync({ module: require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js')() });
import { blake2b } from '@noble/hashes/blake2.js';
const { ScriptPublicKey, addressFromScriptPublicKey, Address, payToAddressScript } = kaspa;

// Provider/treasury = agent wallet (the platform IS the seller; the facilitator
// collapses into the worker key — 1-party sentinel shape).
const AGENT_ADDR = process.env.KASPA_AGENT_ADDRESS ||
  'kaspa:qpkn4aczvuqpmhvzv2lunjudfnda6wlk258w90yptjxv6v2q7dlkq2cm8e58e';

const WORKER_SECRET = process.env.METERED_WORKER_SECRET;
const REGISTER_URL = process.env.METERED_REGISTER_URL ||
  'https://base44.app/api/apps/6a444b036408e68ec8d6f2a6/functions/meteredChatWorker';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function pd(b) { const len = b.length;
  if (len <= 75) return Buffer.concat([Buffer.from([len]), b]);
  if (len <= 255) return Buffer.concat([Buffer.from([0x4c, len]), b]);
  if (len <= 65535) { const lb = Buffer.alloc(2); lb.writeUInt16LE(len); return Buffer.concat([Buffer.from([0x4d]), lb, b]); }
  const lb4 = Buffer.alloc(4); lb4.writeUInt32LE(len); return Buffer.concat([Buffer.from([0x4e]), lb4, b]); }
function encodeNum(n) { n = Number(n); if (n === 0) return Buffer.alloc(0);
  const bytes = []; while (n > 0) { bytes.push(n & 0xff); n = Math.floor(n / 256); }
  if (bytes[bytes.length-1] & 0x80) bytes.push(0x00); return Buffer.from(bytes); }
function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }
function versioned(spk) { return Buffer.concat([Buffer.from([0x00, 0x00]), spk]); }

const OP_IF=0x63, OP_ELSE=0x67, OP_ENDIF=0x68, OP_EQUAL=0x87, OP_EQUALVERIFY=0x88, OP_CLTV=0xb0;
const OP_TX_OUTPUT_AMOUNT = 0xc2, OP_TX_OUTPUT_SPK = 0xc3;

// Identical template to the mainnet-proven builder (2-output shape both branches).
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
  s = Buffer.concat([s, pd(encodeNum(unlockDaa)), Buffer.from([OP_CLTV])]);
  s = Buffer.concat([s, pd(versioned(customerSpk)), pd(encodeNum(refundA))]);
  s = Buffer.concat([s, pd(versioned(customerSpk)), pd(encodeNum(refundB))]);
  s = Buffer.concat([s, Buffer.from([OP_ENDIF])]);
  s = Buffer.concat([s, pd(encodeNum(1)), Buffer.from([OP_TX_OUTPUT_AMOUNT]), Buffer.from([OP_EQUALVERIFY])]);
  s = Buffer.concat([s, pd(encodeNum(1)), Buffer.from([OP_TX_OUTPUT_SPK]), Buffer.from([OP_EQUALVERIFY])]);
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

async function main() {
  const sessionId = arg('session');
  const customerAddr = arg('customer');
  if (!sessionId || !customerAddr) { console.log('RESULT_ERROR: --session and --customer are required'); process.exit(1); }
  if (!customerAddr.startsWith('kaspa:')) { console.log('RESULT_ERROR: --customer must be a kaspa: address'); process.exit(1); }

  const epochs = Math.max(1, Math.min(parseInt(arg('epochs', '3')), 8));
  const repliesPerEpoch = Math.max(1, parseInt(arg('replies', '10')));
  const INCREMENT = BigInt(arg('increment-sompi', '30000000'));   // 0.3 KAS per bundle (proven value)
  const FEE_CHECKIN = BigInt(arg('fee-checkin-sompi', '40000000')); // 0.4 KAS — 65KB script margin (proven value)
  const FEE_REFUND  = BigInt(arg('fee-refund-sompi', '40000000')); // 0.4 KAS (proven value)
  const REMAINDER_FINAL = BigInt(arg('final-remainder-sompi', '30000000')); // returned to guest on last check-in (≥0.3 KAS margin)
  const epochDays = parseFloat(arg('epoch-days', '7'));
  const DAA_PER_DAY = 86400;
  const doRegister = process.argv.includes('--register');

  // 1) Worker XMSS keys (python saves the seed to keys/ FIRST).
  const outDir = path.join(__dirname, 'out');
  fs.mkdirSync(outDir, { recursive: true });
  const buildPath = path.join(outDir, `chain_build_${sessionId}.json`);
  execFileSync('python3', [path.join(__dirname, 'build_chain.py'),
    '--session', sessionId, '--epochs', String(epochs), '--out', buildPath],
    { stdio: 'inherit' });
  const build = JSON.parse(fs.readFileSync(buildPath, 'utf8'));
  if (build.hops.length !== epochs) throw new Error('hop count mismatch');

  // 2) Chain parties.
  const providerSpk = Buffer.from(payToAddressScript(new Address(AGENT_ADDR)).script, 'hex');
  const custSpk = Buffer.from(payToAddressScript(new Address(customerAddr)).script, 'hex');

  const tipInfo = await (await fetch('https://api.kaspa.org/info/blockdag')).json();
  const currentDaa = Number(tipInfo.virtualDaaScore);
  console.log('Current DAA:', currentDaa);

  // 3) Values bottom-up. V(last) = increment + fee_checkin + remainder_final(to guest).
  //    V(i)   = increment + fee_checkin + V(i+1).
  const values = Array(epochs).fill(0n);
  values[epochs - 1] = INCREMENT + FEE_CHECKIN + REMAINDER_FINAL;
  for (let i = epochs - 2; i >= 0; i--) values[i] = INCREMENT + FEE_CHECKIN + values[i + 1];

  // 4) Refund splits per hop (both outputs ≥ 0.3 KAS margin rule).
  const refunds = values.map((v) => {
    const total = v - FEE_REFUND;
    const a = total / 2n;
    return [a, total - a];
  });

  // 5) Hop scripts bottom-up. Next-hop of the FINAL hop = the customer's own
  //    address: last check-in pays the treasury and returns the remainder to the guest.
  const hops = [];
  for (let i = epochs - 1; i >= 0; i--) {
    const deadline = currentDaa + epochDays * DAA_PER_DAY * (i + 1);
    const nextSpk = (i === epochs - 1) ? custSpk : Buffer.from(hops[0].spk_hex, 'hex');
    const remainder = (i === epochs - 1) ? REMAINDER_FINAL : values[i + 1];
    const script = buildEpochScriptX402(
      build.hops[i].verify_block_hex, build.checkin_msg_hex, Math.floor(deadline),
      providerSpk, INCREMENT, nextSpk, remainder,
      custSpk, refunds[i][0], refunds[i][1]);
    const { spk, addr } = p2shAddr(script);
    hops.unshift({ i, script_hex: script.toString('hex'), spk_hex: spk.toString('hex'),
      addr, value: values[i].toString(), remainder: remainder.toString(),
      deadline_daa: Math.floor(deadline),
      refund_a: refunds[i][0].toString(), refund_b: refunds[i][1].toString() });
  }

  const chain = {
    session: sessionId, customer: customerAddr, provider: AGENT_ADDR,
    epochs, repliesPerEpoch,
    increment: INCREMENT.toString(), fee_checkin: FEE_CHECKIN.toString(), fee_refund: FEE_REFUND.toString(),
    current_hop: 0,
    hops,
    // XMSS material lives in chain_build_<session>.json (sandbox-only).
    build_file: path.basename(buildPath),
  };
  const chainPath = path.join(outDir, `chain_out_${sessionId}.json`);
  fs.writeFileSync(chainPath, JSON.stringify(chain, null, 2));

  const depositKas = Number(values[0]) / 1e8;
  console.log('RESULT_JSON:', JSON.stringify({
    session: sessionId, hop0Address: hops[0].addr,
    depositSompi: values[0].toString(), depositKas,
    perBundleKas: Number(INCREMENT) / 1e8,
    epochs, repliesPerEpoch, chainPath,
  }));

  // 6) Register with the backend (session becomes awaiting_fund).
  if (doRegister) {
    if (!WORKER_SECRET) { console.log('RESULT_ERROR: METERED_WORKER_SECRET not set (needed for --register)'); process.exit(1); }
    const res = await fetch(REGISTER_URL, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ secret: WORKER_SECRET, action: 'register',
        sessionId, covenantAddress: hops[0].addr, amountSompi: values[0].toString(),
        epochs, repliesPerEpoch,
        // Entity fields are size-capped: store a slim public summary, not the
        // 65KB-per-hop scripts. The sandbox keeps the full chain locally
        // (out/chain_out_<session>.json) — the worker never needs it from the DB.
        chainJson: JSON.stringify({
          customer: customerAddr, provider: AGENT_ADDR, epochs,
          repliesPerEpoch, increment: INCREMENT.toString(),
          hop0Address: hops[0].addr, depositKas,
          hops: hops.map(h => ({ addr: h.addr, value: h.value, deadline_daa: h.deadline_daa })),
        }) }),
    });
    const j = await res.json().catch(() => ({}));
    console.log('REGISTER:', res.status, JSON.stringify(j));
    if (!res.ok) process.exit(1);
  } else {
    console.log('(dry-run: pass --register to register the chain with the backend)');
  }
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 2000) || String(e))); process.exit(1); });

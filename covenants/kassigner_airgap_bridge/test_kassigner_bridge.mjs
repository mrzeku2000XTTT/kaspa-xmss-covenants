// test_kassigner_bridge.mjs — proves the bridge is wire-correct WITHOUT any
// real key or mainnet spend. No private key is generated or touched except
// a throwaway local test scalar used only to prove the round trip works.
import { buildUnsignedKsptV1, parseSignedKsptV3, scanCandidatePubkeys, LIMITS } from './kassigner_bridge.mjs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { schnorr } = require('./node_modules/@noble/curves/secp256k1.js');
const { sha256 } = require('./node_modules/@noble/hashes/sha256.js');

function assert(cond, msg) { if (!cond) { console.error('FAIL:', msg); process.exit(1); } console.log('ok  -', msg); }

// ── Real redeem script from our already-deployed zkescrow_v2 covenant ──
const REDEEM_HEX = '76008763750635fd3f439f01b07520cd3205bdef99b608c3f909d3675fecc5dce069cff21b0845655d969ba90fa428ac6776518763750635fd3f439f01b075209785896f8950129ef4280fa22469aabd0fd95bfe8ccbe085987911a8b6d8db55ac6775206d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60ad00c324000020cd3205bdef99b608c3f909d3675fecc5dce069cff21b0845655d969ba90fa428ac8700c3240000209785896f8950129ef4280fa22469aabd0fd95bfe8ccbe085987911a8b6d8db55ac879b6868';
const redeemScript = Buffer.from(REDEEM_HEX, 'hex');
assert(redeemScript.length <= LIMITS.MAX_REDEEM_SIZE, `zkescrow redeem script (${redeemScript.length}B) fits device buffer (${LIMITS.MAX_REDEEM_SIZE}B)`);

// ── Step 1: does our JS opcode-aware scanner find the same 3 candidate pubkeys the real firmware would? ──
const BUYER_PK   = 'cd3205bdef99b608c3f909d3675fecc5dce069cff21b0845655d969ba90fa428';
const SELLER_PK  = '9785896f8950129ef4280fa22469aabd0fd95bfe8ccbe085987911a8b6d8db55';
const ARBITER_PK = '6d3af70267001ddd8262bfc9cb8d4cdbdd3bf6550ee2bc815c8ccd3140f37f60';
const candidates = scanCandidatePubkeys(redeemScript).map(b => b.toString('hex'));
assert(candidates.includes(BUYER_PK), 'firmware-equivalent scanner finds BUYER_PK');
assert(candidates.includes(SELLER_PK), 'firmware-equivalent scanner finds SELLER_PK');
assert(candidates.includes(ARBITER_PK), 'firmware-equivalent scanner finds ARBITER_PK');
assert(candidates.length === 3, `exactly 3 candidates found (got ${candidates.length}) — no false positives from the 0x20 salt/amount bytes elsewhere in the script`);

// ── Step 2: build an UNSIGNED KSPT v1 request (what we'd hand the device) ──
const escrowSpk = Buffer.from('aa2063d125736211b73fdf11abbdd5efc2f327833b9fa79407204b1324e84ad6709c87', 'hex');
const fakePrevTxId = Buffer.alloc(32, 0x11); // placeholder outpoint for this test
const destSpk = Buffer.concat([Buffer.from([0x20]), Buffer.from(ARBITER_PK, 'hex'), Buffer.from([0xac])]); // dummy dest

const unsignedTx = {
  version: 0,
  locktime: 0n,
  inputs: [{
    prevTxId: fakePrevTxId,
    prevIndex: 0,
    amount: 100_000_000n, // 1 KAS
    sequence: 0n,
    sigOpCount: 1,
    spkVersion: 0,
    spk: escrowSpk,
    redeemScript,
  }],
  outputs: [{ value: 99_700_000n, spkVersion: 0, spk: destSpk }],
};

const unsignedWire = buildUnsignedKsptV1(unsignedTx);
assert(unsignedWire.subarray(0, 4).toString('ascii') === 'KSPT', 'unsigned wire starts with KSPT magic');
assert(unsignedWire[4] === 0x01, 'unsigned wire version byte = 0x01');
console.log(`    unsigned KSPT wire size: ${unsignedWire.length} bytes`);

// ── Step 3: SIMULATE the device — real Schnorr signature over a throwaway test key ──
// (this key is generated ONLY for this in-memory test and is discarded — no
// funds, no mainnet address, no persistence; satisfies "never sign real
// mainnet spends without a physical device" while still proving the format)
const testKp = schnorr.keygen();
const fakeSighash = sha256(unsignedWire); // stand-in sighash for format-proof purposes only
const schnorrSig = schnorr.sign(fakeSighash, testKp.secretKey); // 64 bytes
assert(schnorrSig.length === 64, 'simulated device produced a 64-byte Schnorr signature');

// ── Step 4: build the SIGNED KSPT v3 response exactly like firmware's serialize_signed_pskt_v2 ──
function buildSignedV3(tx, pubkeyPos, sigBytes) {
  const chunks = [];
  const push = b => chunks.push(Buffer.isBuffer(b) ? b : Buffer.from(b));
  const u16 = v => { const b = Buffer.alloc(2); b.writeUInt16LE(v); return b; };
  const u32 = v => { const b = Buffer.alloc(4); b.writeUInt32LE(v); return b; };
  const u64 = v => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(v)); return b; };
  push(Buffer.from('KSPT', 'ascii')); push([0x03]); push([0x01]); // fully signed
  push(u16(tx.version)); push([1]); push([1]); // 1 input, 1 output
  push(u64(0)); push(Buffer.alloc(20)); push(u64(0)); push(u16(0)); // locktime, subnet, gas, payload_len
  const inp = tx.inputs[0];
  push(inp.prevTxId); push(u32(inp.prevIndex)); push(u64(inp.amount)); push(u64(0));
  push([1]); push(u16(0)); push([inp.spk.length]); push(inp.spk);
  push([1]); push([pubkeyPos]); push([0x01]); push(sigBytes); // sig_count=1, one sig record
  push(u16(inp.redeemScript.length)); push(inp.redeemScript); // v3 u16 redeem len
  const out = tx.outputs[0];
  push(u64(out.value)); push(u16(0)); push([out.spk.length]); push(out.spk);
  return Buffer.concat(chunks);
}
// candidate position = ARBITER_PK's slot (0=buyer,1=seller,2=arbiter per script order)
const arbiterPos = candidates.indexOf(ARBITER_PK);
const signedWire = buildSignedV3(unsignedTx, arbiterPos, schnorrSig);

// ── Step 5: parse the signed response back with OUR bridge, exactly like our own software would ──
const parsed = parseSignedKsptV3(signedWire);
assert(parsed.fullySigned === true, 'parsed signed response reports fully_signed=true');
assert(parsed.inputs[0].sigs.length === 1, 'parsed exactly 1 signature record');
assert(parsed.inputs[0].sigs[0].pubkeyPos === arbiterPos, `sig pubkey_pos matches arbiter candidate slot (${arbiterPos})`);
assert(Buffer.compare(parsed.inputs[0].sigs[0].signature, Buffer.from(schnorrSig)) === 0, 'extracted signature bytes match exactly what the (simulated) device produced');
assert(Buffer.compare(parsed.inputs[0].redeemScript, redeemScript) === 0, 'redeem script round-trips byte-for-byte through signed v3');

// ── Step 6: verify the signature actually verifies against the pubkey (proves it's a real usable Schnorr sig, not just bytes) ──
const validSig = schnorr.verify(parsed.inputs[0].sigs[0].signature, fakeSighash, testKp.publicKey);
assert(validSig, 'extracted signature cryptographically verifies against the signer pubkey + sighash');

console.log('\nAll checks passed — wire format is correct and round-trips against a real deployed covenant script.');
console.log('No real key was used for signing; no mainnet transaction was built or broadcast.');

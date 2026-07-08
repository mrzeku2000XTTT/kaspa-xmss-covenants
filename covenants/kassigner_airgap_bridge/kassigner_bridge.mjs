// kassigner_bridge.mjs — wire-compatible bridge to a real KasSigner air-gapped
// device (github.com/InKasWeRust/KasSigner). Implements the EXACT binary
// formats read by the device firmware (bootloader/src/wallet/pskt.rs):
//
//   - KSPT v1 "unsigned"  (magic "KSPT", version 0x01) — what WE build and
//     hand to the device (via QR or SD file).
//   - KSPT v3 "signed"    (magic "KSPT", version 0x03) — what the DEVICE
//     hands back after it signs (via QR or SD file).
//
// This module does NOT hold or generate any private key. The owner secret
// lives only on the physical device. We only build requests and parse
// responses.
//
// Hard device limits (from bootloader/src/wallet/transaction.rs):
//   MAX_INPUTS=8  MAX_OUTPUTS=8  MAX_SCRIPT_SIZE=512  MAX_REDEEM_SIZE=1024
//   MAX_PAYLOAD_SIZE=768  MAX_SIGS_PER_INPUT=5
//
// IMPORTANT SCOPE NOTE: this only fits covenants whose redeem script is
// <=1024 bytes (zktimelock, zkescrow, multisig, sentinel-with-sig, etc).
// Our RISC0/STARK-verifying scripts (scorpion_brain, zkgate risc0) run into
// the tens of KB and CANNOT be loaded onto this device as a single redeem
// script — that limitation is architectural (ESP32-S3 SRAM), not a bug here.

const MAX_INPUTS = 8;
const MAX_OUTPUTS = 8;
const MAX_SCRIPT_SIZE = 512;
const MAX_REDEEM_SIZE = 1024;
const MAX_PAYLOAD_SIZE = 768;

const PSKT_MAGIC = Buffer.from('KSPT', 'ascii');
const FORMAT_VERSION_V1 = 0x01;
const FORMAT_VERSION_V3 = 0x03;

// ─────────────────────────────────────────────────────────────────────────
// Little writer/reader mirroring the firmware's ByteWriter/ByteReader
// ─────────────────────────────────────────────────────────────────────────
class Writer {
  constructor() { this.chunks = []; }
  bytes(b) { this.chunks.push(Buffer.from(b)); return this; }
  u8(v) { return this.bytes([v & 0xff]); }
  u16le(v) { const b = Buffer.alloc(2); b.writeUInt16LE(v); return this.bytes(b); }
  u32le(v) { const b = Buffer.alloc(4); b.writeUInt32LE(v >>> 0); return this.bytes(b); }
  u64le(v) { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(v)); return this.bytes(b); }
  spkLen(len) {
    if (len <= 254) return this.u8(len);
    this.u8(0xff);
    return this.u16le(len);
  }
  finish() { return Buffer.concat(this.chunks); }
}

class Reader {
  constructor(buf) { this.buf = buf; this.pos = 0; }
  need(n) { if (this.buf.length - this.pos < n) throw new Error('buffer too short'); }
  bytes(n) { this.need(n); const b = this.buf.subarray(this.pos, this.pos + n); this.pos += n; return b; }
  u8() { this.need(1); return this.buf[this.pos++]; }
  u16le() { const b = this.bytes(2); return b.readUInt16LE(); }
  u32le() { const b = this.bytes(4); return b.readUInt32LE(); }
  u64le() { const b = this.bytes(8); return b.readBigUInt64LE(); }
  spkLen() { const b = this.u8(); return b === 0xff ? this.u16le() : b; }
  remaining() { return this.buf.length - this.pos; }
}

// ─────────────────────────────────────────────────────────────────────────
// Build UNSIGNED KSPT v1 — this is what goes TO the device.
//
// tx = {
//   version: number (0 or 1; must be 1 if any output has covenant binding),
//   locktime: bigint|number (0 for standard),
//   inputs: [{
//     prevTxId: Buffer(32), prevIndex: number,
//     amount: bigint|number, sequence: bigint|number (default 0),
//     sigOpCount: number (default 1),
//     spkVersion: number (default 0), spk: Buffer,
//     redeemScript: Buffer|null,   // <=1024 bytes, the covenant script
//   }],
//   outputs: [{ value: bigint|number, spkVersion: number, spk: Buffer }],
//   payload: Buffer|null,
// }
// ─────────────────────────────────────────────────────────────────────────
export function buildUnsignedKsptV1(tx) {
  const inputs = tx.inputs || [];
  const outputs = tx.outputs || [];
  if (inputs.length === 0) throw new Error('no inputs');
  if (inputs.length > MAX_INPUTS) throw new Error(`too many inputs (${inputs.length} > ${MAX_INPUTS})`);
  if (outputs.length === 0) throw new Error('no outputs');
  if (outputs.length > MAX_OUTPUTS) throw new Error(`too many outputs (${outputs.length} > ${MAX_OUTPUTS})`);

  const hasRedeem = inputs.some(i => i.redeemScript && i.redeemScript.length > 0);
  for (const i of inputs) {
    if (i.redeemScript && i.redeemScript.length > MAX_REDEEM_SIZE) {
      throw new Error(`redeem script too large for KasSigner: ${i.redeemScript.length} > ${MAX_REDEEM_SIZE} bytes (device buffer limit)`);
    }
    if (i.spk.length > MAX_SCRIPT_SIZE) throw new Error(`input spk too large: ${i.spk.length} > ${MAX_SCRIPT_SIZE}`);
  }
  for (const o of outputs) {
    if (o.spk.length > MAX_SCRIPT_SIZE) throw new Error(`output spk too large: ${o.spk.length} > ${MAX_SCRIPT_SIZE}`);
  }

  const payload = tx.payload || Buffer.alloc(0);
  if (payload.length > MAX_PAYLOAD_SIZE) throw new Error(`payload too large: ${payload.length} > ${MAX_PAYLOAD_SIZE}`);

  let flags = 0x00;
  if (hasRedeem) flags |= 0x02;
  // we never set 0x04 (covenant output binding) here — that's a KIP-20
  // output-tagging feature, separate from spending FROM a covenant input.

  const w = new Writer();
  w.bytes(PSKT_MAGIC).u8(FORMAT_VERSION_V1).u8(flags);
  w.u16le(tx.version || 0);
  w.u8(inputs.length);
  w.u8(outputs.length);
  w.u64le(tx.locktime || 0n);
  w.bytes(Buffer.alloc(20)); // subnetwork_id, all-zero standard subnetwork
  w.u64le(tx.gas || 0n);
  w.u16le(payload.length);
  if (payload.length) w.bytes(payload);

  for (const inp of inputs) {
    if (!inp.prevTxId || inp.prevTxId.length !== 32) throw new Error('prevTxId must be 32 bytes');
    w.bytes(inp.prevTxId);
    w.u32le(inp.prevIndex);
    w.u64le(inp.amount);
    w.u64le(inp.sequence || 0n);
    w.u8(inp.sigOpCount ?? 1);
    w.u16le(inp.spkVersion ?? 0);
    w.spkLen(inp.spk.length);
    w.bytes(inp.spk);
    if (hasRedeem) {
      const rs = inp.redeemScript || Buffer.alloc(0);
      w.u8(rs.length); // NOTE: unsigned v1 redeem length is a single byte (0-255)
      if (rs.length > 255) throw new Error('KSPT v1 unsigned redeem_len is 1 byte (max 255) — use SD-file v3 path for 256-1024B scripts');
      if (rs.length) w.bytes(rs);
    }
  }

  for (const out of outputs) {
    w.u64le(out.value);
    w.u16le(out.spkVersion ?? 0);
    w.spkLen(out.spk.length);
    w.bytes(out.spk);
    // has_covenant_data flag not set, so no per-output covenant bytes here
  }

  return w.finish();
}

// ─────────────────────────────────────────────────────────────────────────
// Parse SIGNED KSPT v3 — this is what comes BACK from the device.
// Mirrors firmware's serialize_signed_pskt_v2 / parse_signed_pskt_v2.
// ─────────────────────────────────────────────────────────────────────────
export function parseSignedKsptV3(buf) {
  const r = new Reader(buf);
  const magic = r.bytes(4);
  if (!magic.equals(PSKT_MAGIC)) throw new Error('bad magic — not a KSPT buffer');
  const version = r.u8();
  if (version !== FORMAT_VERSION_V3) throw new Error(`expected signed v3 (0x03), got 0x${version.toString(16)}`);
  const flags = r.u8();
  const fullySigned = (flags & 0x01) !== 0;

  const txVersion = r.u16le();
  const numInputs = r.u8();
  const numOutputs = r.u8();
  const locktime = r.u64le();
  const subnetworkId = Buffer.from(r.bytes(20));
  const gas = r.u64le();
  const payloadLen = r.u16le();
  const payload = payloadLen ? Buffer.from(r.bytes(payloadLen)) : Buffer.alloc(0);

  const inputs = [];
  for (let i = 0; i < numInputs; i++) {
    const prevTxId = Buffer.from(r.bytes(32));
    const prevIndex = r.u32le();
    const amount = r.u64le();
    const sequence = r.u64le();
    const sigOpCount = r.u8();
    const spkVersion = r.u16le();
    const spkLen = r.spkLen();
    const spk = Buffer.from(r.bytes(spkLen));

    const sigCount = r.u8();
    const sigs = [];
    for (let s = 0; s < sigCount; s++) {
      const pubkeyPos = r.u8();
      const sighashType = r.u8();
      const signature = Buffer.from(r.bytes(64));
      sigs.push({ pubkeyPos, sighashType, signature });
    }

    const redeemLen = r.u16le(); // v3: u16 LE
    const redeemScript = redeemLen ? Buffer.from(r.bytes(redeemLen)) : Buffer.alloc(0);

    inputs.push({ prevTxId, prevIndex, amount, sequence, sigOpCount, spkVersion, spk, sigs, redeemScript });
  }

  const outputs = [];
  for (let i = 0; i < numOutputs; i++) {
    const value = r.u64le();
    const spkVersion = r.u16le();
    const spkLen = r.spkLen();
    const spk = Buffer.from(r.bytes(spkLen));
    outputs.push({ value, spkVersion, spk });
  }

  return { fullySigned, txVersion, locktime, subnetworkId, gas, payload, inputs, outputs };
}

// ─────────────────────────────────────────────────────────────────────────
// Opcode-aware pubkey candidate scan — mirrors the firmware's scanner in
// sign_transaction_multisig() exactly (bootloader/src/wallet/pskt.rs
// ~line 1069-1120). Lets us PREDICT, in software, which pubkeys the real
// device will find in a given redeem script before we ever touch hardware.
// ─────────────────────────────────────────────────────────────────────────
export function scanCandidatePubkeys(redeemScript) {
  const rs = redeemScript;
  const candidates = [];
  let off = 0;
  while (off < rs.length && candidates.length < 8) {
    const op = rs[off];
    if (op === 0x20 && off + 33 <= rs.length) {
      const after = off + 33;
      const hasChecksig =
        (after < rs.length && (rs[after] === 0xac || rs[after] === 0xad)) ||
        (after + 1 < rs.length && (rs[after + 1] === 0xac || rs[after + 1] === 0xad));
      if (hasChecksig) {
        candidates.push(Buffer.from(rs.subarray(off + 1, off + 33)));
      }
      off += 33;
    } else if (op >= 0x01 && op <= 0x4b) {
      off += 1 + op;
    } else if (op === 0x4c) {
      off += (off + 1 < rs.length) ? 2 + rs[off + 1] : 1;
    } else if (op === 0x4d) {
      if (off + 2 < rs.length) {
        const n = rs[off + 1] | (rs[off + 2] << 8);
        off += 3 + n;
      } else off += 1;
    } else if (op === 0x4e) {
      if (off + 4 < rs.length) {
        const n = rs[off + 1] | (rs[off + 2] << 8) | (rs[off + 3] << 16) | (rs[off + 4] << 24);
        off += 5 + n;
      } else off += 1;
    } else {
      off += 1;
    }
  }
  return candidates;
}

export const LIMITS = { MAX_INPUTS, MAX_OUTPUTS, MAX_SCRIPT_SIZE, MAX_REDEEM_SIZE, MAX_PAYLOAD_SIZE };

const Fq = 21888242871839275222246405745257275088696311157297823662689037894645226208583n;

function bigToLE32(x) {
  const b = Buffer.alloc(32);
  let v = x;
  for (let i = 0; i < 32; i++) { b[i] = Number(v & 0xffn); v >>= 8n; }
  return b;
}

// compress a single Fq coordinate with y-sign flag baked into top bits of last byte
function compressFq(x, y) {
  const buf = bigToLE32(x);
  const flag = (y > (Fq - y)) ? 0x80 : 0x00; // bit6 = sign flag, bit7 = infinity (always 0 here)
  buf[31] |= flag;
  return buf;
}

// G1 point: (x,y) -> 32 bytes
function compressG1(pt) {
  const x = BigInt(pt[0]), y = BigInt(pt[1]);
  return compressFq(x, y);
}

// G2 point: pt = [[x_c0,x_c1],[y_c0,y_c1],[1,0]] -> 64 bytes (c0 plain LE, c1 LE with flag)
// Flag comparison for G2 uses the Fq2 element compared lexicographically: (y_c1, y_c0) vs (Fq-y_c1, Fq-y_c0)
function compressG2(pt) {
  const x0 = BigInt(pt[0][0]), x1 = BigInt(pt[0][1]);
  const y0 = BigInt(pt[1][0]), y1 = BigInt(pt[1][1]);
  const negY1 = Fq - y1, negY0 = (Fq - y0) % Fq;
  // Lexicographic compare on (c1, c0): y bigger if y1>negY1, or y1===negY1 and y0>negY0
  let larger;
  if (y1 !== negY1) larger = y1 > negY1;
  else larger = y0 > negY0;
  const c0buf = bigToLE32(x0);
  const c1buf = bigToLE32(x1);
  c1buf[31] |= larger ? 0x80 : 0x00;
  return Buffer.concat([c0buf, c1buf]);
}

function encodeProof(proof) {
  const A = compressG1(proof.pi_a);
  const B = compressG2(proof.pi_b);
  const C = compressG1(proof.pi_c);
  return Buffer.concat([A, B, C]);
}

export { compressG1, compressG2, encodeProof, Fq };

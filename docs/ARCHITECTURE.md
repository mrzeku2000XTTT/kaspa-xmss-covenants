# Architecture

## The primitive: XMSS-SHA2_10_192 in Kaspa Script

Standard XMSS ([RFC 8391](https://www.rfc-editor.org/rfc/rfc8391)) builds a
signature out of three layers:

1. **WOTS+ one-time signature** — 51 hash chains, each up to 15 steps
   (`F(x) = SHA256(0x00000000 || KEY || (x XOR BITMASK))`, where `KEY` and
   `BITMASK` are derived from a public seed + an address, not secret).
2. **L-tree** — compresses the 51 WOTS+ chain outputs into one leaf hash.
3. **Merkle tree** — walks an auth path from that leaf up to a public root.

None of this requires elliptic curve math — it's SHA256, XOR, and
concatenation the whole way down, which is exactly what `OpSHA256`, `OpXor`,
and `OpCat` give you in Kaspa script.

## Efficient conditional-skip WOTS+ verification

A naive unrolled WOTS+ chain (apply F up to 15 times, branch on every
possible digit value) costs ~8,356 bytes per chain. This repo uses a
**shared, conditional chain**: one 15-step loop where each step applies F
only if `digit <= step` (`OP_LESSTHANOREQUAL`). This is ~1,216 bytes per
chain — about 7x smaller — while still rejecting forged witnesses or wrong
digit claims, verified against all 16 possible digit values locally before
any mainnet deployment.

## On-chain message → digit pipeline

The WOTS+ checksum scheme requires computing 48 message-nibble digits plus
3 checksum digits (checksum = sum of `15 - digit` across all message
digits, split into base-16 digits via integer division/modulo). Kaspa's
post-Toccata `OpDiv`/`OpMod`/`OpAnd` opcodes make this possible entirely
on-chain — the message is revealed at spend time and the digits are
derived live in-script, not precomputed off-chain and trusted blindly.

### Two real consensus bugs found during this work (not simulated locally)

1. **CScriptNum sign encoding.** Bytes extracted via `OpSubstr` with the
   high bit set (≥ 0x80) decode as *negative* numbers under Kaspa's
   CScriptNum rules once you feed them into `OpDiv`/`OpMod` (same
   convention as Bitcoin Script: the top bit of the last byte is a sign
   flag). Every extracted byte must be padded with a trailing `0x00` before
   arithmetic to force it positive.
2. **OP_XOR requires exactly equal-length operands.** A local Python test
   harness using `zip()` will silently truncate mismatched lengths and give
   a false "PASS" — real Kaspa consensus correctly rejects this
   (`"XOR operands must be of equal length"`). This was only caught by an
   actual mainnet rejection, at the cost of one small deposit
   (permanently unspendable — the lesson paid for itself many times over
   since). `core/kvm.py` (the local interpreter used for testing) has since
   been patched to raise on length mismatch too, so this bug class can't
   silently pass in local testing again.

## Genuine hypertree linkage (2-layer XMSS^MT)

The key design goal in `covenants/hypertree_2layer/` was to avoid a common
shortcut: hardcoding an intermediate "layer0 root" as a script constant and
only checking layer1 against it. That proves the *mechanism* works but not
that the layers are actually linked.

Instead: `app_message` (revealed at spend time) feeds layer0's WOTS+ / L-tree
/ Merkle-path verification, which computes `root0` live, on-chain, from the
witness data. `root0` is then used directly as layer1's message input — no
hardcoded intermediate value anywhere in the script. A forgery test
(tampering with a single layer0 witness byte) breaks verification of the
*entire* chain, not just that one layer, which is the actual proof of
linkage.

Script size barely grows with tree height — going from height=4 (2^8
addressable) to height=16 (2^32 addressable) per layer only added about
1.4KB to the redeem script, because only the WOTS+ chains and the auth path
execute on-chain (`O(height)`); the full tree construction needed to derive
an auth path happens off-chain, in plain Python, at keygen time.

## Consensus constraints this design respects

- **244-item stack limit** — the 4-layer single-input hypertree variant was
  specifically chosen (over a 5-layer version) to fit real signature
  verification into one covenant input without hitting this cap.
- **500,000 mass cap** — full 2-layer hypertree spends land around
  270,000–280,000 mass, comfortably under the limit with room for other
  covenant types layered on top in the future.

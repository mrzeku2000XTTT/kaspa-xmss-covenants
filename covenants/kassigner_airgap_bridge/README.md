# KasSigner air-gapped signing bridge

Wire-compatible bridge to a real [KasSigner](https://github.com/InKasWeRust/KasSigner)
air-gapped hardware signer (ESP32-S3, keys live only in device RAM, never
touches a network). Implements the exact binary formats read/written by the
device firmware (`bootloader/src/wallet/pskt.rs`), reverse-engineered from
the reference open-source repo — not guessed.

## What this actually does

- `buildUnsignedKsptV1(tx)` — builds the "KSPT v1" unsigned request buffer
  (magic `KSPT` + version `0x01`) that a real device reads from a QR code or
  SD card file.
- `parseSignedKsptV3(buf)` — parses the "KSPT v3" signed response buffer
  (magic `KSPT` + version `0x03`) the device hands back after physically
  signing, extracting the raw 64-byte Schnorr signature(s) per input.
- `scanCandidatePubkeys(redeemScript)` — mirrors the firmware's own
  opcode-aware scanner byte-for-byte (walks the script honoring push-data
  lengths, looking for `OP_DATA_32 <32B> OP_CHECKSIG/CHECKSIGVERIFY`), so we
  can predict in software exactly which pubkeys a real device will find in
  any of our redeem scripts before ever touching hardware.

**No private key is generated or stored by this module.** The owner key
lives only on the physical device. This module only builds requests and
parses responses — the same "build → carry physically → sign → carry back"
flow described in KasSigner's own `cov++` design (KasSee builds/watches,
KasSigner signs).

## Verified today (13/13 checks pass, `test_kassigner_bridge.mjs`)

Round-tripped against our **real, already-deployed** `zkescrow_v2` covenant
redeem script (216 bytes, live buyer/seller/arbiter P2SH on mainnet):

1. Script fits the device's 1024-byte redeem-script buffer.
2. Our JS scanner finds the same 3 candidate pubkeys (buyer/seller/arbiter)
   the real firmware scanner would — no false positives from the `0x20`
   bytes that appear incidentally elsewhere in the script (timeout/amount
   constants).
3. Unsigned KSPT v1 request builds correctly (magic, version, all fields).
4. A simulated signature round-trips through the exact signed-v3 byte
   layout and is extracted byte-identical, with a cryptographic
   `schnorr.verify` pass proving it's a real usable signature, not just
   matching bytes.

The signing key used in the test is a disposable in-memory scalar generated
only to prove the wire format — no funds, no mainnet address, discarded
immediately, per the standing key-backup rule.

## Hard scope limit (from the device firmware itself)

```
MAX_INPUTS=8  MAX_OUTPUTS=8  MAX_SCRIPT_SIZE=512  MAX_REDEEM_SIZE=1024 bytes
```

These are firmware constants (`bootloader/src/wallet/transaction.rs`), not
something we can configure around — the ESP32-S3 SRAM budget.

**Works today for:** zktimelock, zkescrow, multisig, sentinel-with-signature,
and any covenant whose redeem script embeds a plain
`<32-byte-pubkey> OP_CHECKSIG` branch under ~1KB — all of our currently
mainnet-deployed covenant families of this shape.

**Does NOT fit today:** scorpion_brain and zkgate-risc0 — their redeem
scripts run 10-65x past the 1024-byte device buffer (RISC0/STARK verifier
opcodes are large). Air-gapping those would need either (a) much smaller
device SRAM budget than exists today, or (b) restructuring so only a small
escape-hatch authorization branch (not the whole verifier) needs a device
signature — an open design question, not solved here.

## What's still needed before real mainnet use

This is software-only proof of wire compatibility. Nothing here has touched
a physical device. Before any real fund is protected by this bridge:

1. Physical KasSigner hardware (build or buy, flash verified firmware).
2. Import the *existing* zktimelock/zkescrow owner private key into the
   device's "Raw private key" wallet slot (no need to regenerate keys —
   the device supports importing an existing 32-byte scalar via keypad).
3. Do a real end-to-end test: unsigned KSPT → physically move to device →
   sign → move back → broadcast a dust-amount transaction first.
4. Only after that succeeds, migrate real-value covenant spends to this flow.

Until step 3 passes, keep using the existing local key-backup workflow —
this bridge doesn't replace `mandatory_key_backup.md` rules, it's a future
upgrade path once hardware is in hand.

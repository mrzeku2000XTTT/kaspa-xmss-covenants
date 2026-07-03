# QR Code Transfer Spec (design doc, not yet implemented)

Goal: move data across the air gap with **zero physical connection** —
no USB drive, no cable — by displaying data as a QR code on one device and
scanning it with a camera on the other.

This is a design spec for the next implementation step, not shipped code.

## What needs to cross the gap, and in which direction

| Data | Direction | Size | Sensitive? |
|---|---|---|---|
| `public.json` (pub_seed0/1, master_root, redeem_script_hex) | offline → online | redeem_script_hex is ~130KB hex for a 2-layer tree | No — safe to expose |
| signing request (message to authorize + optional context like destination address / amount) | online → offline | small, <200 bytes | No, but should be *reviewed* on the offline screen before signing |
| `witness.json` (103 items ≈ 24 bytes each ≈ 2.5KB) | offline → online | ~2.5KB | No — a witness is a used signature, safe to expose (that's the whole point of hash-based sigs) |

Note the redeem_script_hex is too large for a single QR code (QR max
payload ~3KB at low error correction). Two options:

1. **Only QR the witness + signing request** (both small, fit in 1-3 QR
   codes easily) and move `public.json` once via USB at deploy time only
   (it's not sensitive, so this is a reasonable compromise — the repeated
   air-gap crossings that matter for security, at spend time, stay QR-only).
2. **Chunk the redeem script** across multiple sequential QR frames (like
   [BC-UR](https://github.com/BlockchainCommons/Research/blob/master/papers/bcr-2020-005-ur.md)
   / animated QR used by hardware wallets) if a fully QR-only flow end to
   end is required.

## Recommended format

Reuse [BC-UR (Uniform Resources)](https://github.com/BlockchainCommons/bc-ur)
— it's the exact standard hardware wallets like Keystone/Passport already
use for animated multi-frame QR transfer, so plenty of QR reader
implementations already exist for it (mobile + desktop). Concretely:

```
ur:xmss-witness/<cbor-encoded witness array>
```

Single-frame if the payload fits (witness data does), multi-frame
(`ur:xmss-witness/1-3/...`, `2-3/...`, `3-3/...`) if chunking the redeem
script.

## Suggested next implementation step

1. `pip install ur` (Python BC-UR implementation) on both sides.
2. `qr_encode.py --in witness.json --out qr_frames/` → generates PNGs.
3. `qr_decode.py` on the online device using any camera + a standard QR/UR
   decoder library to reassemble the JSON.

Not built yet because it needs an actual physical device/camera to test
meaningfully — flagging as the concrete next step for the hardware-signer
product idea.

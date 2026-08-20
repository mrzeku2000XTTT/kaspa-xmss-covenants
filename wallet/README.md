# KCC20 Wallet

Premium non-custodial Kaspa wallet for this repo: native KAS, KCC20 token watchlist, and covenant vaults (timelock, escrow, multisig, XMSS, Sentinel).

## Run locally

```bash
cd wallet
npx --yes serve -p 4173
```

Open http://localhost:4173

On GitHub Pages the same app is published from `docs/` (copy of these files).

## Deploy

**GitHub Pages** (this repo already uses `docs/`):

1. Repo **Settings → Pages**
2. Source: **Deploy from a branch**
3. Branch: `main`, folder: `/docs`
4. Site: `https://mrzeku2000xttt.github.io/kaspa-xmss-covenants/`

**Vercel** (static, no build):

1. [vercel.com/new](https://vercel.com/new) → import `mrzeku2000XTTT/kaspa-xmss-covenants`
2. **Root Directory:** `wallet`
3. Framework: Other / leave empty. No build command.
4. Deploy. You get a `*.vercel.app` URL.

## Features

- Video-background iOS shell (phone chrome on desktop, full-bleed on iPhone)
- Create / import Schnorr wallet (browser keygen)
- Live KAS balance, USD, UTXOs, activity from api.kaspa.org
- Send / receive (QR)
- KCC20 watchlist
- AI + manual covenant builder (existing `kccApi` backend)

Keys stay in `localStorage`. Confirmed sends still use the proven covenant builder to assemble the transaction.

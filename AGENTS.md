# Base44 Dev Environment

## What this is
A non-custodial Kaspa wallet PWA (`wallet/`) — pure static HTML/JS/CSS, no build step. The main user-facing app. Also contains `studio/` (FastAPI video/image generation backend, separate, optional).

## How it runs
- `docker-compose.base44.yml` runs `nginx:alpine` serving `wallet/` on host port 3000.
- Nginx config: `nginx.base44.conf` (MIME types for WASM, no-cache headers, API proxy rewrites matching `wallet/vercel.json`).
- No build step, no dependencies to install — just static files served by nginx.
- Edits to `wallet/` files appear immediately (nginx serves from the bind mount; call `reload_preview` to refresh the iframe).

## Verification
- `curl -s -o /dev/null -w '%{http_code}' http://localhost:3000/` → 200
- Title should be "KCC20 Wallet — Kaspa L1 covenant wallet"
- JS served as `application/javascript`, WASM as `application/wasm`.

## Secrets
- The wallet PWA needs no external credentials — keys are generated client-side and Kaspa node connections are direct WebSocket from the browser.
- `studio/` needs `OPENAI_API_KEY` but runs in mock mode without it; it is not part of the default compose setup.

## API proxies (nginx)
- `/cook-api/*` → `https://dev-api-kcc20.kaspa.com/*`
- `/vprog-tt/*` → `https://vprogs-tt.izio.fr/*`
- `/api/cook/*` → `https://dev-api-kcc20.kaspa.com/*`
- `/api/ksocial` → `https://mainnet.kaspatalk.net`

// mark_funded.mjs — record the guest's funding outpoint on the local chain file
// after the PSKT-signed funding tx confirms, so checkin.mjs knows the hop0 outpoint.
//
// Verifies on L1 that the funding tx actually paid hop0's address, then stores
// { txid, idx } as the chain's spendable outpoint.
//
// Usage: node mark_funded.mjs --session=mc_xxx --txid=<funding txid>
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
function arg(name, def) { const a = process.argv.find(x => x.startsWith(`--${name}=`)); return a ? a.split('=').slice(1).join('=') : def; }

async function main() {
  const sessionId = arg('session');
  const txid = arg('txid');
  if (!sessionId || !txid) { console.log('RESULT_ERROR: --session and --txid are required'); process.exit(1); }
  const chainPath = path.join(__dirname, 'out', `chain_out_${sessionId}.json`);
  const chain = JSON.parse(fs.readFileSync(chainPath, 'utf8'));
  const hop0 = chain.hops[0];

  const r = await fetch(`https://api.kaspa.org/transactions/${txid}`);
  if (!r.ok) { console.log(`RESULT_ERROR: tx ${txid} not found on L1 (status ${r.status})`); process.exit(1); }
  const tx = await r.json();
  const outputs = tx?.transaction?.outputs || [];

  let foundIdx = -1;
  for (let i = 0; i < outputs.length; i++) {
    const o = outputs[i];
    // amount must equal the hop0 deposit
    if (String(o.amount || '') !== String(hop0.value)) continue;
    // spk must be the hop0 covenant spk
    if (String(o.scriptPublicKey?.script || '').toLowerCase() === hop0.spk_hex.toLowerCase() ||
        o.address === hop0.addr) { foundIdx = i; break; }
  }
  if (foundIdx < 0) {
    console.log('RESULT_ERROR: funding tx does not pay hop0 covenant. Expected value '
      + hop0.value + ' to ' + hop0.addr);
    process.exit(1);
  }

  chain.hop_utxo = { txid, idx: foundIdx };
  chain.funding_tx = txid;
  fs.writeFileSync(chainPath, JSON.stringify(chain, null, 2));
  console.log('RESULT_JSON:', JSON.stringify({ ok: true, session: sessionId, txid, idx: foundIdx, hop0: hop0.addr }));
}

main().catch(e => { console.log('RESULT_ERROR: ' + (e.message?.slice(0, 2000) || String(e))); process.exit(1); });

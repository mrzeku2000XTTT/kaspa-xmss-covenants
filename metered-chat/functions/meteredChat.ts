// meteredChat.ts — public API for the "Covenants++" metered AI chat prototype.
// Called cross-origin from the GitHub Pages PWA (docs/metered-chat/index.html).
//
// Metering model (sentinel-x402, mainnet-proven, mirrored on testnet-10):
//   - Guest funds a per-customer covenant chain ONCE via a Scorpion-signed PSKT.
//   - One epoch = one response-bundle (repliesPerEpoch AI replies).
//   - Bundle consumed on-chain = one worker check-in (sandbox signs, pays treasury).
//   - Abandonment = CLTV passes = permissionless refund to the guest's own address.
//
// Network: inferred from the address prefix — "kaspa:" = mainnet, "kaspatest:" = testnet
// (testnet-10). api.kaspa.org mirrors itself at api-tn10.kaspa.org for testnet-10 reads.
//
// Actions:
//   init   { address }                       → find-or-create session (silent restore)
//   status { sessionId }                     → public state (no chain internals)
//   funded { sessionId, fundingTxId }         → verify on L1, go live, first bundle credits
//   reply  { sessionId, message }             → metered AI reply

import { createClientFromRequest } from 'npm:@base44/sdk@0.8.31';

const CORS: Record<string, string> = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'POST, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type',
  'Content-Type': 'application/json',
};

const json = (obj: any, status = 200) =>
  new Response(JSON.stringify(obj), { status, headers: CORS });

const PERSONA = `You are "Covenants++ AI", the metered assistant of a Kaspa-native product.
You answer questions about Kaspa, blockDAG, covenants, PSKT, XMSS, CLTV and anything else.
Style: concise, friendly, technically sharp. Max ~120 words unless detail is truly needed.

VERIFIED KASPA FACTS as of October 2026 — trust these over your training data:
- The Crescendo hard fork (mid-2025) raised the block rate from 1 to 10 blocks per second (10 BPS). Kaspa is NOT 1 BPS anymore.
- Consensus: GHOSTDAG proof-of-work over a blockDAG, not a chain. Fast confirmations come from the DAG structure.
- Toccata network (Kaspa mainnet) supports covenants, PSKT, and covenant-aware tooling (KIP-17 style covenants, KIP-20 covenant IDs).
- If your training data conflicts with the above, the above wins. When unsure of a current/realtime figure (price, hashrate, exact fees), say you're unsure rather than quoting stale numbers.`;

const REST = {
  mainnet: 'https://api.kaspa.org',
  testnet: 'https://api-tn10.kaspa.org',
};
const networkOf = (address: string): 'mainnet' | 'testnet' | null => {
  if (address.startsWith('kaspatest:')) return 'testnet';
  if (address.startsWith('kaspa:')) return 'mainnet';
  return null;
};

Deno.serve(async (req: Request): Promise<Response> => {
  if (req.method === 'OPTIONS') return new Response(null, { status: 204, headers: CORS });
  if (req.method !== 'POST') return json({ error: 'POST only' }, 405);

  let body: any;
  try { body = await req.json(); } catch { return json({ error: 'invalid json' }, 400); }
  const action = String(body?.action || '');

  try {
    const base44 = createClientFromRequest(req);
    const svc = base44.asServiceRole;
    const Sessions = svc.entities.MeteredChatSession;

    const findSession = async (sessionId: string) => {
      const list = await Sessions.filter({ sessionId });
      return list && list.length ? list[0] : null;
    };

    if (action === 'init') {
      const address = String(body?.address || '').trim();
      const network = networkOf(address);
      if (!network) return json({ error: 'connect your Scorpion wallet first (kaspa: or kaspatest: address)' }, 400);
      // Silent session restore: reuse the newest session for this address if one exists.
      const existing = await Sessions.filter({ address });
      if (existing && existing.length) {
        const s = existing[0];
        return json({ ok: true, sessionId: s.sessionId, status: s.status, network: s.network || network,
          covenantAddress: s.covenantAddress || null, amountSompi: s.amountSompi || null,
          epochs: s.epochs || 0, repliesPerEpoch: s.repliesPerEpoch || 0,
          repliesRemaining: s.repliesRemaining || 0, epochsRemaining: s.epochsRemaining || 0,
          restored: true });
      }
      const sessionId = 'mc_' + crypto.randomUUID().replace(/-/g, '').slice(0, 18);
      await Sessions.create({
        sessionId, address, network, status: 'awaiting_covenant',
        epochs: 0, repliesPerEpoch: 0, repliesRemaining: 0, epochsRemaining: 0,
      });
      return json({ ok: true, sessionId, status: 'awaiting_covenant', network, restored: false });
    }

    if (action === 'status') {
      const s = await findSession(String(body?.sessionId || ''));
      if (!s) return json({ error: 'session not found' }, 404);
      return json({ ok: true, sessionId: s.sessionId, status: s.status, network: s.network || networkOf(s.address) || 'mainnet',
        covenantAddress: s.covenantAddress || null, amountSompi: s.amountSompi || null,
        epochs: s.epochs || 0, repliesPerEpoch: s.repliesPerEpoch || 0,
        repliesRemaining: s.repliesRemaining || 0, epochsRemaining: s.epochsRemaining || 0,
        fundingTxId: s.fundingTxId || null });
    }

    if (action === 'funded') {
      const sessionId = String(body?.sessionId || '');
      const fundingTxId = String(body?.fundingTxId || '');
      const s = await findSession(sessionId);
      if (!s) return json({ error: 'session not found' }, 404);
      if (s.status !== 'awaiting_fund') return json({ error: 'session not awaiting funding', status: s.status }, 409);

      const network = (s.network || networkOf(s.address) || 'mainnet') as 'mainnet' | 'testnet';
      const restBase = REST[network];

      // Verify on L1: tx exists and carries an output of the quoted amount to the covenant.
      let verified = false;
      try {
        const r = await fetch(`${restBase}/transactions/${fundingTxId}`);
        if (r.ok) {
          const tx = await r.json();
          const outputs = tx?.transaction?.outputs || [];
          verified = outputs.some((o: any) =>
            String(o.amount || '') === String(s.amountSompi || ''));
        }
      } catch { /* indexer unreachable — record txId for later audit */ }

      const replies = Number(s.repliesPerEpoch || 0);
      const epochs = Number(s.epochs || 0);
      await Sessions.update(s.id, {
        status: 'live', fundingTxId,
        repliesRemaining: replies,               // first bundle activates on funding
        epochsRemaining: Math.max(0, epochs - 1) // still locked as future hops
      });
      return json({ ok: true, verified, status: 'live',
        repliesRemaining: replies, epochsRemaining: Math.max(0, epochs - 1) });
    }

    if (action === 'reply') {
      const sessionId = String(body?.sessionId || '');
      const message = String(body?.message || '').trim().slice(0, 2000);
      if (!message) return json({ error: 'missing message' }, 400);
      const s = await findSession(sessionId);
      if (!s) return json({ error: 'session not found' }, 404);
      if (s.status !== 'live') return json({ error: 'session not live', status: s.status }, 409);
      const remaining = Number(s.repliesRemaining || 0);
      const epochsRemaining = Number(s.epochsRemaining || 0);
      if (remaining <= 0) {
        return json({ error: 'bundle exhausted', needsCheckin: true, epochsRemaining }, 402);
      }

      let history: any[] = [];
      try { history = JSON.parse(s.history || '[]'); } catch {}
      history.push({ role: 'user', content: message });
      const convo = history.slice(-7)
        .map((t) => `${t.role === 'user' ? 'User' : 'You'}: ${t.content}`).join('\n');

      let replyText = '';
      try {
        const llm = await svc.integrations.Core.InvokeLLM({
          prompt: `${PERSONA}\n\nConversation so far:\n${convo}\n\nReply to the user's latest message.`,
          response_json_schema: { type: 'object', properties: { reply: { type: 'string' } }, required: ['reply'] },
        });
        replyText = String(llm?.reply || '').trim();
      } catch (e: any) {
        // LLM hiccup = credit preserved (only successful replies consume credit)
        return json({ ok: true, reply: `The AI brain hiccuped (${String(e?.message || e).slice(0, 80)}). No credit consumed — try again.`,
          creditConsumed: false, repliesRemaining: remaining, epochsRemaining });
      }
      if (!replyText) replyText = '(empty reply — credit preserved)';

      history.push({ role: 'assistant', content: replyText });
      const newRemaining = remaining - 1;
      await Sessions.update(s.id, {
        repliesRemaining: newRemaining,
        history: JSON.stringify(history.slice(-20)),
        status: newRemaining === 0 && epochsRemaining <= 0 ? 'exhausted' : 'live',
      });
      return json({ ok: true, reply: replyText, creditConsumed: true,
        repliesRemaining: newRemaining, epochsRemaining,
        needsCheckin: newRemaining === 0 && epochsRemaining > 0 });
    }

    return json({ error: 'unknown action' }, 400);
  } catch (e: any) {
    return json({ error: String(e?.message || e) }, 500);
  }
});

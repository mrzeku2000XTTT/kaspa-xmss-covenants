import { parseIntent, askFor, describeIntent } from '../js/intent.js';

const a = parseIntent('lock 200 kas pqs');
if (a.type !== 'xmss') throw new Error('expected xmss, got ' + a.type);
if (a.complete) throw new Error('should wait for kit');
if (!a.missing.some(m => /kit|xmss_keygen/i.test(m))) throw new Error('missing kit: ' + a.missing);
console.log('ask', askFor(a.missing));
console.log('desc', describeIntent(a));

const kit = JSON.stringify({ redeem_script_hex: 'aa'.repeat(120), height: 10 });
const b = parseIntent(kit, a);
if (b.type !== 'xmss') throw new Error('kit merge type ' + b.type);
if (!b.params.kit) throw new Error('kit not attached');
if (b.missing?.length) throw new Error('still missing ' + b.missing);
if (!b.complete) throw new Error('should be complete');
console.log('complete', describeIntent(b));

const priv = parseIntent(JSON.stringify({ sec_seed_hex: 'ab', redeem_script_hex: 'aa'.repeat(40) }), a);
if (!priv.missing.some(m => /PUBLIC XMSS|private key/i.test(m))) throw new Error('private kit not rejected: ' + priv.missing);
console.log('ok');

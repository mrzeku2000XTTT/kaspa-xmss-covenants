import * as kaspa from 'https://esm.sh/@onekeyfe/kaspa-wasm@1.0.2';
import { blake2b } from 'https://esm.sh/@noble/hashes@1.3.3/blake2b';

let _initialized = false;
let _initPromise = null;

async function initSDK() {
  if (_initialized) return;
  if (_initPromise) return _initPromise;
  _initPromise = (async () => {
    const res = await fetch('https://unpkg.com/@onekeyfe/kaspa-wasm@1.0.2/kaspa_bg.wasm.bin');
    const wasmBytes = await res.arrayBuffer();
    const wasmModule = await WebAssembly.compile(wasmBytes);
    kaspa.initSync({ module: wasmModule });
    _initialized = true;
  })();
  return _initPromise;
}

function randomHexKey() {
  const arr = new Uint8Array(32);
  crypto.getRandomValues(arr);
  return Array.from(arr).map(b => b.toString(16).padStart(2,'0')).join('');
}

function pkFromHex(hex) { return new kaspa.PrivateKey(hex); }
function hexToBytes(hex) {
  const arr = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) arr[i/2] = parseInt(hex.slice(i, i+2), 16);
  return arr;
}
function bytesToHex(bytes) { return Array.from(bytes).map(b => b.toString(16).padStart(2,'0')).join(''); }
function concatBytes(arrays) {
  const total = arrays.reduce((n, a) => n + a.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const a of arrays) { out.set(a, off); off += a.length; }
  return out;
}

const OP_DUP=0x76, OP_DROP=0x75, OP_EQUAL=0x87, OP_IF=0x63, OP_ELSE=0x67, OP_ENDIF=0x68;
const OP_0=0x00, OP_1=0x51, OP_CHECKSIG=0xac, OP_CLTV=0xb0;
const OP_2=0x52, OP_CHECKMULTISIG=0xae;
const DAA_PER_DAY = 864000;

function scriptNum(n) {
  if (n === 0) return new Uint8Array(0);
  let abs = Math.abs(n), bytes = [];
  while (abs > 0) { bytes.push(abs & 0xff); abs = Math.floor(abs / 256); }
  if (bytes[bytes.length-1] & 0x80) bytes.push(0x00);
  return new Uint8Array(bytes);
}

function pushData(buf) {
  if (buf.length === 0) return new Uint8Array([0x00]);
  if (buf.length <= 75) return new Uint8Array([buf.length, ...buf]);
  throw new Error('push too large');
}

async function getRpcUrl() {
  try {
    const resolver = new kaspa.Resolver();
    return await resolver.getUrl('borsh', 'mainnet');
  } catch {
    return 'wss://ivy.kaspa.green/kaspa/mainnet/wrpc/borsh';
  }
}

async function broadcastTx(tx) {
  const url = await getRpcUrl();
  const rpc = new kaspa.RpcClient({ url, networkId: 'mainnet' });
  await rpc.connect();
  let txId;
  try {
    const r = await rpc.submitTransaction({ transaction: tx, allowOrphan: false });
    txId = r.transactionId;
  } finally {
    await rpc.disconnect().catch(()=>{});
  }
  return txId;
}

self.onmessage = async (e) => {
  const { id, action, data } = e.data;
  try {
    await initSDK();

    switch (action) {
      case 'generateWallet': {
        const hex = randomHexKey();
        const pk = pkFromHex(hex);
        const addr = kaspa.createAddress(pk, 'mainnet');
        self.postMessage({ id, ok: true, result: { privKey: hex, address: addr.toString() } });
        break;
      }

      case 'importWallet': {
        const pk = pkFromHex(data.privKey);
        const addr = kaspa.createAddress(pk, 'mainnet');
        self.postMessage({ id, ok: true, result: { privKey: data.privKey, address: addr.toString() } });
        break;
      }

      case 'sendKas': {
        const { dest, amountKas, utxos, changeAddress, privKeyHex } = data;
        const pk = pkFromHex(privKeyHex);
        const entries = utxos.map(u => ({
          address: changeAddress,
          outpoint: { transactionId: u.outpoint.transactionId, index: u.outpoint.index },
          amount: BigInt(u.utxoEntry.amount),
          scriptPublicKey: { version: 0, script: u.utxoEntry.scriptPublicKey.scriptPublicKey },
          blockDaaScore: BigInt(u.utxoEntry.blockDaaScore || 0),
          isCoinbase: false
        }));
        const amountSompi = BigInt(Math.round(amountKas * 100000000));
        const result = await kaspa.createTransactions({
          entries, outputs: [{ address: dest, amount: amountSompi }],
          changeAddress, priorityFee: BigInt(5000), networkId: 'mainnet'
        });
        const tx = result.transactions[0].transaction;
        for (let i = 0; i < tx.inputs.length; i++) {
          const sig = kaspa.createInputSignature(tx, i, pk, 1);
          tx.inputs[i].signatureScript = sig;
        }
        const txId = await broadcastTx(tx);
        self.postMessage({ id, ok: true, result: { txId } });
        break;
      }

      case 'computeCovenant': {
        const { type, params, pubKeyHex } = data;
        const myPkX = hexToBytes(pubKeyHex);
        const daaRes = await fetch('https://api.kaspa.org/info/blockdag');
        const daaData = await daaRes.json();
        const currentDaa = Number(daaData.virtualDaaScore);
        let redeemScript;

        if (type === 'timelock') {
          const lockDays = params.lockDays || 1;
          const unlockDaa = currentDaa + Math.max(1, Math.round(lockDays * DAA_PER_DAY));
          redeemScript = concatBytes([
            pushData(scriptNum(unlockDaa)), new Uint8Array([OP_CLTV]),
            pushData(myPkX), new Uint8Array([OP_CHECKSIG])
          ]);
        } else if (type === 'escrow') {
          const buyerPkX = hexToBytes(data.buyerPubKeyHex);
          const arbiterPkX = data.arbiterPubKeyHex ? hexToBytes(data.arbiterPubKeyHex) : myPkX;
          const timeoutDays = params.timeoutDays || 30;
          const timeoutDaa = currentDaa + Math.round(timeoutDays * DAA_PER_DAY);
          redeemScript = concatBytes([
            new Uint8Array([OP_DUP, OP_0, OP_EQUAL, OP_IF]),
              new Uint8Array([OP_DROP]), pushData(arbiterPkX), new Uint8Array([OP_CHECKSIG]),
            new Uint8Array([OP_ELSE, OP_DUP, OP_1, OP_EQUAL, OP_IF]),
              new Uint8Array([OP_DROP]), pushData(scriptNum(timeoutDaa)), new Uint8Array([OP_CLTV]),
              pushData(myPkX), new Uint8Array([OP_CHECKSIG]),
            new Uint8Array([OP_ELSE]),
              new Uint8Array([OP_DROP]), pushData(scriptNum(timeoutDaa)), new Uint8Array([OP_CLTV]),
              pushData(buyerPkX), new Uint8Array([OP_CHECKSIG]),
            new Uint8Array([OP_ENDIF, OP_ENDIF])
          ]);
        } else if (type === 'multisig') {
          const cpPkX = hexToBytes(data.counterpartyPubKeyHex);
          redeemScript = concatBytes([
            new Uint8Array([OP_2]), pushData(myPkX), pushData(cpPkX),
            new Uint8Array([OP_2, OP_CHECKMULTISIG])
          ]);
        } else {
          throw new Error('Unknown covenant type: ' + type);
        }

        const scriptHash = blake2b(redeemScript, { dkLen: 32 });
        const spkBytes = new Uint8Array([0xaa, 0x20, ...scriptHash, 0x87]);
        const spk = new kaspa.ScriptPublicKey(0, spkBytes);
        const p2shAddr = kaspa.addressFromScriptPublicKey(spk, 'mainnet');

        let description = '', extraData = {};
        if (type === 'timelock') {
          const lockDays = params.lockDays || 1;
          const unlockDaa = currentDaa + Math.max(1, Math.round(lockDays * DAA_PER_DAY));
          description = 'Locked until DAA ' + unlockDaa + '. Only your key can release it after that block.';
          extraData = { unlockDaa };
        } else if (type === 'escrow') {
          const timeoutDays = params.timeoutDays || 30;
          const timeoutDaa = currentDaa + Math.round(timeoutDays * DAA_PER_DAY);
          description = 'Escrow: arbiter can release anytime; after DAA ' + timeoutDaa + ', seller or buyer can reclaim.';
          extraData = { timeoutDaa };
        } else if (type === 'multisig') {
          description = '2-of-2 multisig: both you and the counterparty must sign to spend.';
        }

        self.postMessage({ id, ok: true, result: {
          covenantAddress: p2shAddr.toString(),
          redeemScriptHex: bytesToHex(redeemScript),
          scriptPubKeyHex: bytesToHex(spkBytes),
          description, ...extraData
        }});
        break;
      }

      case 'fundCovenant': {
        const { covenantAddress, amountKas, utxos, changeAddress, privKeyHex } = data;
        const pk = pkFromHex(privKeyHex);
        const entries = utxos.map(u => ({
          address: changeAddress,
          outpoint: { transactionId: u.outpoint.transactionId, index: u.outpoint.index },
          amount: BigInt(u.utxoEntry.amount),
          scriptPublicKey: { version: 0, script: u.utxoEntry.scriptPublicKey.scriptPublicKey },
          blockDaaScore: BigInt(u.utxoEntry.blockDaaScore || 0),
          isCoinbase: false
        }));
        const amountSompi = BigInt(Math.round(amountKas * 100000000));
        const result = await kaspa.createTransactions({
          entries, outputs: [{ address: covenantAddress, amount: amountSompi }],
          changeAddress, priorityFee: BigInt(5000), networkId: 'mainnet'
        });
        const tx = result.transactions[0].transaction;
        for (let i = 0; i < tx.inputs.length; i++) {
          const sig = kaspa.createInputSignature(tx, i, pk, 1);
          tx.inputs[i].signatureScript = sig;
        }
        const txId = await broadcastTx(tx);
        self.postMessage({ id, ok: true, result: { txId } });
        break;
      }

      default:
        throw new Error('Unknown action: ' + action);
    }
  } catch (err) {
    self.postMessage({ id, ok: false, error: err.message, stack: err.stack });
  }
};

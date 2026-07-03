const path = require('path');
const { WASI } = require('wasi');

// Patch: create a CJS wrapper for the ESM kaspa.js
// The kaspa_bg.wasm.js provides the binary inline
const loadWasm = require('./node_modules/@onekeyfe/kaspa-wasm/kaspa_bg.wasm.js');
const wasmBinary = loadWasm();
console.log('WASM binary loaded, size:', wasmBinary.length, 'bytes');

// Now patch kaspa.js to use CJS style
const kaspaJs = require('fs').readFileSync('./node_modules/@onekeyfe/kaspa-wasm/kaspa.js', 'utf8');

// Check the exports pattern
const exportMatch = kaspaJs.match(/export\s+\{[^}]+\}/g);
if (exportMatch) {
  console.log('ESM exports found:', exportMatch[0].slice(0, 200));
}
console.log('Is ESM:', kaspaJs.includes('export default'));

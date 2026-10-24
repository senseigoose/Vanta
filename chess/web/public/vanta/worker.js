// Web Worker that runs Vanta (WASM). Loaded as a module worker by lib/engine.ts.
// Needs vanta_wasm.js / vanta_wasm_bg.wasm next to it (built by chess/scripts/build-wasm.sh)
// and the net at /nets/<id>.nnue.
import init, { Vanta } from './vanta_wasm.js';

let engine = null;

async function start(netId) {
  await init();
  const res = await fetch('/nets/' + netId + '.nnue');
  if (!res.ok) throw new Error('Net not found: ' + netId);
  engine = new Vanta(new Uint8Array(await res.arrayBuffer()), 16);
}

self.onmessage = async (e) => {
  const m = e.data;
  try {
    if (m.type === 'init') {
      await start(m.net);
      self.postMessage({ type: 'ready' });
    } else if (m.type === 'newgame') {
      engine.new_game();
    } else if (m.type === 'go') {
      engine.set_position(m.fen, m.moves || '');
      const out = engine.go(m.depth || 0, m.nodes || 0, m.movetime || 0, (line) => {
        self.postMessage({ type: 'info', id: m.id, info: JSON.parse(line) });
      });
      self.postMessage({ type: 'done', id: m.id, result: JSON.parse(out) });
    }
  } catch (err) {
    self.postMessage({ type: 'error', id: m.id, message: String((err && err.message) || err) });
  }
};

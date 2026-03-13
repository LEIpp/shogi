// Web Worker for parallel MCTS search
// Each worker loads its own WASM instance and runs independent MCTS simulations.
// Results (per-root-child visit stats) are sent back for aggregation.

import init, { wasm_mcts_search_stats } from './shogi-engine/pkg/shogi_engine.js';

let wasmReady = false;
let initPromise = null;

async function ensureInit() {
  if (wasmReady) return;
  if (!initPromise) {
    initPromise = init().then(() => { wasmReady = true; });
  }
  await initPromise;
}

self.onmessage = async function(e) {
  await ensureInit();

  const { board, sHand, gHand, simulations, maximizing, timeLimit, variantCode, requestId } = e.data;

  const stats = wasm_mcts_search_stats(
    new Int8Array(board),
    new Uint8Array(sHand),
    new Uint8Array(gHand),
    simulations,
    maximizing,
    timeLimit,
    variantCode
  );

  // Convert to plain array for structured cloning
  const result = [];
  for (let i = 0; i < stats.length; i++) {
    const s = stats[i];
    result.push({
      type: s.type, fr: s.fr, fc: s.fc, tr: s.tr, tc: s.tc,
      promote: s.promote, piece: s.piece,
      visits: s.visits, value: s.value
    });
  }

  self.postMessage({ requestId, stats: result });
};

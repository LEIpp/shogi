// Web Worker for parallel MCTS and Lazy SMP minimax search
// Each worker loads its own WASM instance with independent TT.

import init, { wasm_init, wasm_search, wasm_mcts_search_stats } from './shogi-engine/pkg/shogi_engine.js';

let wasmReady = false;
let initPromise = null;

async function ensureInit() {
  if (wasmReady) return;
  if (!initPromise) {
    initPromise = init().then(() => {
      wasm_init();  // Initialize independent TT for this worker
      wasmReady = true;
    });
  }
  await initPromise;
}

self.onmessage = async function(e) {
  await ensureInit();
  const d = e.data;

  if (d.type === 'minimax') {
    const result = wasm_search(
      new Int8Array(d.board),
      new Uint8Array(d.sHand),
      new Uint8Array(d.gHand),
      d.maxDepth,
      d.maximizing,
      d.timeLimit,
      d.variant
    );
    // Convert JsValue to plain object for structured cloning
    self.postMessage({
      type: 'minimax',
      workerId: d.workerId,
      result: {
        found: result.found,
        type: result.type,
        fr: result.fr, fc: result.fc,
        tr: result.tr, tc: result.tc,
        promote: result.promote,
        piece: result.piece,
        score: result.score,
        completedDepth: result.completedDepth
      }
    });
  } else {
    // MCTS search (default / type === 'mcts')
    const { board, sHand, gHand, simulations, maximizing, timeLimit, variantCode, requestId } = d;

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
    const statResult = [];
    for (let i = 0; i < stats.length; i++) {
      const s = stats[i];
      statResult.push({
        type: s.type, fr: s.fr, fc: s.fc, tr: s.tr, tc: s.tc,
        promote: s.promote, piece: s.piece,
        visits: s.visits, value: s.value
      });
    }

    self.postMessage({ requestId, stats: statResult });
  }
};

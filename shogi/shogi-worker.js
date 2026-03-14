// Web Worker for parallel MCTS and Lazy SMP minimax search
// Each worker loads its own WASM instance with independent TT and MCTS tree.

import init, {
  wasm_init, wasm_search, wasm_mcts_search_stats,
  wasm_mcts_init_tree, wasm_mcts_run_batch, wasm_mcts_get_stats,
  wasm_mcts_apply_move, wasm_mcts_clear_tree
} from './shogi-engine/pkg/shogi_engine.js';

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

function convertStats(stats) {
  const result = [];
  for (let i = 0; i < stats.length; i++) {
    const s = stats[i];
    result.push({
      type: s.type, fr: s.fr, fc: s.fc, tr: s.tr, tc: s.tc,
      promote: s.promote, piece: s.piece,
      visits: s.visits, value: s.value
    });
  }
  return result;
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
  } else if (d.type === 'mcts_init') {
    wasm_mcts_init_tree(
      new Int8Array(d.board),
      new Uint8Array(d.sHand),
      new Uint8Array(d.gHand),
      d.maximizing, d.variantCode
    );
    self.postMessage({ type: 'mcts_init_done', requestId: d.requestId });
  } else if (d.type === 'mcts_batch') {
    const stats = wasm_mcts_run_batch(d.count);
    self.postMessage({
      type: 'mcts_batch_done',
      requestId: d.requestId,
      stats: convertStats(stats)
    });
  } else if (d.type === 'mcts_apply_move') {
    const found = wasm_mcts_apply_move(
      d.moveType, d.fr, d.fc, d.tr, d.tc, d.promote, d.piece
    );
    self.postMessage({ type: 'mcts_apply_move_done', requestId: d.requestId, found });
  } else if (d.type === 'mcts_clear') {
    wasm_mcts_clear_tree();
    self.postMessage({ type: 'mcts_clear_done', requestId: d.requestId });
  } else {
    // Legacy: one-shot MCTS search (type === 'mcts' or untyped)
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
    self.postMessage({ requestId, stats: convertStats(stats) });
  }
};

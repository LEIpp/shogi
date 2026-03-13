use crate::types::*;
use crate::movegen::*;
use crate::eval::*;
use crate::apply::*;
use crate::tt::*;
use crate::check::*;

const MAX_DEPTH: u8 = 6;

pub struct SearchState {
    pub tt: TranspositionTable,
    pub nodes: u64,
    pub time_limit: f64,  // milliseconds, 0 = no limit
    pub start_time: f64,  // js Date.now()
    pub aborted: bool,
}

impl SearchState {
    pub fn new(tt: TranspositionTable) -> Self {
        SearchState {
            tt,
            nodes: 0,
            time_limit: 0.0,
            start_time: 0.0,
            aborted: false,
        }
    }
}

fn score_move_for_ordering(m: &ShogiMove, board: &Board, tt_best_idx: Option<usize>, move_idx: usize) -> i32 {
    // TT best move gets highest priority
    if let Some(best) = tt_best_idx {
        if move_idx == best { return 100_000; }
    }

    let mut s: i32 = 0;

    // Captures: MVV-LVA
    if m.move_type == MOVE_TYPE_MOVE {
        let target = board_get(board, m.tr as usize, m.tc as usize);
        if target != EMPTY {
            let victim = piece_value(abs_piece(target));
            let attacker = if m.fr != 255 {
                piece_value(abs_piece(board_get(board, m.fr as usize, m.fc as usize)))
            } else { 100 };
            s += 10_000 + victim * 10 - attacker;
        }
    }

    if m.promote { s += 5_000; }
    if m.move_type == MOVE_TYPE_MOVE { s += 100; }
    if m.move_type == MOVE_TYPE_PROMOTE_NIN { s += 500; }

    s
}

pub fn minimax(
    state: &mut SearchState,
    board: &Board,
    s_hand: &Hand,
    g_hand: &Hand,
    depth: u8,
    mut alpha: i32,
    mut beta: i32,
    maximizing: bool,
    eval_owner_depth: u8,
    variant: GameVariant,
) -> (i32, Option<ShogiMove>) {
    state.nodes += 1;

    // Time limit check every 1024 nodes
    if state.time_limit > 0.0 && (state.nodes & 1023) == 0 {
        let now = js_sys::Date::now();
        if now - state.start_time >= state.time_limit {
            state.aborted = true;
            return (evaluate(board, s_hand, g_hand, variant), None);
        }
    }
    if state.aborted {
        return (evaluate(board, s_hand, g_hand, variant), None);
    }

    let owner = if maximizing { SENTE } else { GOTE };

    if depth == 0 {
        return (evaluate(board, s_hand, g_hand, variant), None);
    }

    let hash = hash_board(board, s_hand, g_hand, maximizing);

    // TT probe
    let tt_hit = if depth < eval_owner_depth {
        state.tt.probe(hash, depth, alpha, beta)
    } else { None };

    if let Some((score, _)) = tt_hit {
        return (score, None);
    }

    let tt_best_move = tt_hit.and_then(|(_, m)| m)
        .or_else(|| state.tt.get_best_move(hash));

    let mut moves = get_all_legal_moves(board, owner, s_hand, g_hand, variant);

    if moves.is_empty() {
        if is_in_check(board, owner) {
            let mate_score = if maximizing {
                -99999 + (MAX_DEPTH as i32 - depth as i32) * 100
            } else {
                99999 - (MAX_DEPTH as i32 - depth as i32) * 100
            };
            return (mate_score, None);
        }
        return (0, None); // stalemate
    }

    // Move ordering
    let mut scored: Vec<(i32, usize)> = moves.iter().enumerate()
        .map(|(i, m)| (score_move_for_ordering(m, board, tt_best_move, i), i))
        .collect();
    scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));

    // Late move pruning: limit drops at shallow depths
    if depth <= 2 {
        let mut board_count = 0usize;
        let mut drop_count = 0usize;
        let mut pruned_indices = Vec::with_capacity(scored.len());
        for &(_, idx) in &scored {
            if moves[idx].move_type == MOVE_TYPE_DROP {
                if drop_count < 8 {
                    pruned_indices.push(idx);
                    drop_count += 1;
                }
            } else {
                pruned_indices.push(idx);
                board_count += 1;
            }
        }
        let _ = board_count;
        // Replace scored with pruned
        scored = pruned_indices.iter().map(|&i| (0, i)).collect();
    }

    let mut best_move_idx = scored[0].1;
    let mut best_score;
    let mut tt_flag;

    if maximizing {
        best_score = i32::MIN;
        tt_flag = TranspositionTable::FLAG_ALPHA;
        for &(_, idx) in &scored {
            let m = &moves[idx];
            let (nb, nsh, ngh) = apply_move(board, m, s_hand, g_hand, owner, variant);
            let (score, _) = minimax(state, &nb, &nsh, &ngh, depth - 1, alpha, beta, false, eval_owner_depth, variant);
            if state.aborted { break; }
            if score > best_score { best_score = score; best_move_idx = idx; }
            if score > alpha { alpha = score; tt_flag = TranspositionTable::FLAG_EXACT; }
            if beta <= alpha { tt_flag = TranspositionTable::FLAG_BETA; break; }
        }
    } else {
        best_score = i32::MAX;
        tt_flag = TranspositionTable::FLAG_BETA;
        for &(_, idx) in &scored {
            let m = &moves[idx];
            let (nb, nsh, ngh) = apply_move(board, m, s_hand, g_hand, owner, variant);
            let (score, _) = minimax(state, &nb, &nsh, &ngh, depth - 1, alpha, beta, true, eval_owner_depth, variant);
            if state.aborted { break; }
            if score < best_score { best_score = score; best_move_idx = idx; }
            if score < beta { beta = score; tt_flag = TranspositionTable::FLAG_EXACT; }
            if beta <= alpha { tt_flag = TranspositionTable::FLAG_ALPHA; break; }
        }
    }

    if !state.aborted {
        state.tt.store(hash, depth, best_score, tt_flag, Some(best_move_idx));
    }

    (best_score, Some(moves[best_move_idx]))
}

pub fn iterative_deepening(
    state: &mut SearchState,
    board: &Board,
    s_hand: &Hand,
    g_hand: &Hand,
    max_depth: u8,
    maximizing: bool,
    time_limit_ms: u32,
    variant: GameVariant,
) -> (i32, Option<ShogiMove>) {
    state.tt.generation = state.tt.generation.wrapping_add(1);
    state.nodes = 0;
    state.time_limit = time_limit_ms as f64;
    state.start_time = js_sys::Date::now();
    state.aborted = false;

    let mut best_result: (i32, Option<ShogiMove>) = (0, None);

    for d in 1..=max_depth {
        let result = minimax(state, board, s_hand, g_hand, d, i32::MIN + 1, i32::MAX - 1, maximizing, d, variant);
        if state.aborted { break; }
        if result.1.is_some() { best_result = result; }
        // Early exit on forced mate
        if result.0.unsigned_abs() > 90000 { break; }
        // Time check before next depth
        if state.time_limit > 0.0 && (js_sys::Date::now() - state.start_time) >= state.time_limit { break; }
    }

    best_result
}

mod types;
mod movegen;
mod check;
mod apply;
mod eval;
mod tt;
mod search;
mod mcts;

use wasm_bindgen::prelude::*;
use types::*;
use tt::TranspositionTable;
use search::{SearchState, iterative_deepening};
use mcts::RootChildStat;

static mut SEARCH_STATE: Option<SearchState> = None;

fn get_state() -> &'static mut SearchState {
    unsafe {
        SEARCH_STATE.as_mut().expect("Call wasm_init() first")
    }
}

#[wasm_bindgen]
pub fn wasm_init() {
    unsafe {
        let tt = TranspositionTable::new(20); // 2^20 = ~1M entries
        SEARCH_STATE = Some(SearchState::new(tt));
    }
}

#[wasm_bindgen]
pub fn wasm_tt_clear() {
    let state = get_state();
    state.tt.clear();
}

fn parse_board(board_flat: &[i8]) -> Board {
    let mut board: Board = [0i8; BOARD_SIZE];
    for i in 0..BOARD_SIZE.min(board_flat.len()) { board[i] = board_flat[i]; }
    board
}

fn parse_hands(sente_hand: &[u8], gote_hand: &[u8]) -> (Hand, Hand) {
    let mut s_hand: Hand = [0u8; 8];
    let mut g_hand: Hand = [0u8; 8];
    for i in 0..8.min(sente_hand.len()) { s_hand[i] = sente_hand[i]; }
    for i in 0..8.min(gote_hand.len()) { g_hand[i] = gote_hand[i]; }
    (s_hand, g_hand)
}

fn stats_to_js(stats: &[RootChildStat]) -> JsValue {
    let arr = js_sys::Array::new();
    for s in stats {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"type".into(), &(s.m.move_type as i32).into()).unwrap();
        js_sys::Reflect::set(&obj, &"fr".into(), &(s.m.fr as i32).into()).unwrap();
        js_sys::Reflect::set(&obj, &"fc".into(), &(s.m.fc as i32).into()).unwrap();
        js_sys::Reflect::set(&obj, &"tr".into(), &(s.m.tr as i32).into()).unwrap();
        js_sys::Reflect::set(&obj, &"tc".into(), &(s.m.tc as i32).into()).unwrap();
        js_sys::Reflect::set(&obj, &"promote".into(), &s.m.promote.into()).unwrap();
        js_sys::Reflect::set(&obj, &"piece".into(), &(s.m.piece as i32).into()).unwrap();
        js_sys::Reflect::set(&obj, &"visits".into(), &s.visits.into()).unwrap();
        js_sys::Reflect::set(&obj, &"value".into(), &s.total_value.into()).unwrap();
        arr.push(&obj);
    }
    arr.into()
}

#[wasm_bindgen]
pub fn wasm_search(
    board_flat: &[i8],
    sente_hand: &[u8],
    gote_hand: &[u8],
    max_depth: u8,
    maximizing: bool,
    time_limit_ms: u32,
    game_variant: u8,
) -> JsValue {
    let state = get_state();
    let variant = GameVariant::from_u8(game_variant);
    let board = parse_board(board_flat);
    let (s_hand, g_hand) = parse_hands(sente_hand, gote_hand);

    let (score, best_move, completed_depth) = iterative_deepening(
        state, &board, &s_hand, &g_hand,
        max_depth, maximizing, time_limit_ms, variant,
    );

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"completedDepth".into(), &(completed_depth as i32).into()).unwrap();
    match best_move {
        Some(m) => {
            js_sys::Reflect::set(&obj, &"found".into(), &true.into()).unwrap();
            js_sys::Reflect::set(&obj, &"type".into(), &(m.move_type as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"fr".into(), &(m.fr as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"fc".into(), &(m.fc as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"tr".into(), &(m.tr as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"tc".into(), &(m.tc as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"promote".into(), &m.promote.into()).unwrap();
            js_sys::Reflect::set(&obj, &"piece".into(), &(m.piece as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"score".into(), &score.into()).unwrap();
        }
        None => {
            js_sys::Reflect::set(&obj, &"found".into(), &false.into()).unwrap();
            js_sys::Reflect::set(&obj, &"score".into(), &score.into()).unwrap();
        }
    }

    obj.into()
}

#[wasm_bindgen]
pub fn wasm_mcts_search_stats(
    board_flat: &[i8],
    sente_hand: &[u8],
    gote_hand: &[u8],
    simulations: u32,
    maximizing: bool,
    time_limit_ms: u32,
    game_variant: u8,
) -> JsValue {
    let variant = GameVariant::from_u8(game_variant);
    let board = parse_board(board_flat);
    let (s_hand, g_hand) = parse_hands(sente_hand, gote_hand);

    let stats = mcts::mcts_search_root_stats(
        &board, &s_hand, &g_hand,
        simulations, maximizing, time_limit_ms, variant,
    );

    stats_to_js(&stats)
}

#[wasm_bindgen]
pub fn wasm_mcts_search(
    board_flat: &[i8],
    sente_hand: &[u8],
    gote_hand: &[u8],
    simulations: u32,
    maximizing: bool,
    time_limit_ms: u32,
    game_variant: u8,
) -> JsValue {
    let variant = GameVariant::from_u8(game_variant);
    let board = parse_board(board_flat);
    let (s_hand, g_hand) = parse_hands(sente_hand, gote_hand);

    let (score, best_move) = mcts::mcts_search(
        &board, &s_hand, &g_hand,
        simulations, maximizing, time_limit_ms, variant,
    );

    let obj = js_sys::Object::new();
    match best_move {
        Some(m) => {
            js_sys::Reflect::set(&obj, &"found".into(), &true.into()).unwrap();
            js_sys::Reflect::set(&obj, &"type".into(), &(m.move_type as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"fr".into(), &(m.fr as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"fc".into(), &(m.fc as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"tr".into(), &(m.tr as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"tc".into(), &(m.tc as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"promote".into(), &m.promote.into()).unwrap();
            js_sys::Reflect::set(&obj, &"piece".into(), &(m.piece as i32).into()).unwrap();
            js_sys::Reflect::set(&obj, &"score".into(), &score.into()).unwrap();
        }
        None => {
            js_sys::Reflect::set(&obj, &"found".into(), &false.into()).unwrap();
            js_sys::Reflect::set(&obj, &"score".into(), &score.into()).unwrap();
        }
    }

    obj.into()
}

// ============================================================
// Persistent MCTS tree WASM bindings (pondering + incremental)
// ============================================================

#[wasm_bindgen]
pub fn wasm_mcts_init_tree(
    board_flat: &[i8], sente_hand: &[u8], gote_hand: &[u8],
    maximizing: bool, game_variant: u8,
) {
    let variant = GameVariant::from_u8(game_variant);
    let board = parse_board(board_flat);
    let (s_hand, g_hand) = parse_hands(sente_hand, gote_hand);
    mcts::mcts_init_tree(&board, &s_hand, &g_hand, maximizing, variant);
}

#[wasm_bindgen]
pub fn wasm_mcts_run_batch(count: u32) -> JsValue {
    let stats = mcts::mcts_run_batch(count);
    stats_to_js(&stats)
}

#[wasm_bindgen]
pub fn wasm_mcts_get_stats() -> JsValue {
    let stats = mcts::mcts_get_stats();
    stats_to_js(&stats)
}

#[wasm_bindgen]
pub fn wasm_mcts_apply_move(
    move_type: u8, fr: u8, fc: u8, tr: u8, tc: u8, promote: bool, piece: i8,
) -> bool {
    mcts::mcts_apply_move(move_type, fr, fc, tr, tc, promote, piece)
}

#[wasm_bindgen]
pub fn wasm_mcts_clear_tree() {
    mcts::mcts_clear_tree();
}

#[wasm_bindgen]
pub fn wasm_mcts_has_tree() -> bool {
    mcts::mcts_has_tree()
}

#[wasm_bindgen]
pub fn wasm_mcts_root_visits() -> u32 {
    mcts::mcts_root_visits()
}

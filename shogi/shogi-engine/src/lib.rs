mod types;
mod movegen;
mod check;
mod apply;
mod eval;
mod tt;
mod search;

use wasm_bindgen::prelude::*;
use types::*;
use tt::TranspositionTable;
use search::{SearchState, iterative_deepening};

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

    // Convert inputs
    let mut board: Board = [0i8; BOARD_SIZE];
    for i in 0..BOARD_SIZE.min(board_flat.len()) {
        board[i] = board_flat[i];
    }

    let mut s_hand: Hand = [0u8; 8];
    let mut g_hand: Hand = [0u8; 8];
    for i in 0..8.min(sente_hand.len()) {
        s_hand[i] = sente_hand[i];
    }
    for i in 0..8.min(gote_hand.len()) {
        g_hand[i] = gote_hand[i];
    }

    let (score, best_move) = iterative_deepening(
        state, &board, &s_hand, &g_hand,
        max_depth, maximizing, time_limit_ms, variant,
    );

    // Build result JS object
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

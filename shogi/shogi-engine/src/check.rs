use crate::types::*;
use crate::movegen::{get_piece_moves, get_drop_squares};

pub fn find_king(board: &Board, owner: i8) -> Option<(usize, usize)> {
    for r in 0..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            let p = board_get(board, r, c);
            if piece_owner(p) == owner && abs_piece(p) == OU {
                return Some((r, c));
            }
        }
    }
    None
}

pub fn is_in_check(board: &Board, owner: i8) -> bool {
    let king = match find_king(board, owner) {
        Some(k) => k,
        None => return true,
    };
    let opp = -owner;
    for r in 0..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            let p = board_get(board, r, c);
            if piece_owner(p) != opp { continue; }
            let moves = get_piece_moves(p, r, c, board);
            for (mr, mc) in moves {
                if mr == king.0 && mc == king.1 { return true; }
            }
        }
    }
    false
}

pub fn is_legal_move(board: &Board, fr: usize, fc: usize, tr: usize, tc: usize, owner: i8) -> bool {
    let mut nb = *board;
    board_set(&mut nb, tr, tc, board_get(board, fr, fc));
    board_set(&mut nb, fr, fc, EMPTY);
    !is_in_check(&nb, owner)
}

pub fn is_checkmate_position(board: &Board, owner: i8, s_hand: &Hand, g_hand: &Hand, variant: GameVariant) -> bool {
    // Check if owner has any legal moves (board moves)
    for r in 0..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            let p = board_get(board, r, c);
            if piece_owner(p) != owner { continue; }
            let moves = get_piece_moves(p, r, c, board);
            for (mr, mc) in moves {
                if is_legal_move(board, r, c, mr, mc, owner) { return false; }
            }
        }
    }
    // Check drops
    let hand = if owner == SENTE { s_hand } else { g_hand };
    for (i, &pt) in HAND_PIECE_TYPES.iter().enumerate() {
        if hand[i] == 0 { continue; }
        let drops = get_drop_squares(board, pt, owner, s_hand, g_hand, variant);
        if !drops.is_empty() { return false; }
    }
    true
}

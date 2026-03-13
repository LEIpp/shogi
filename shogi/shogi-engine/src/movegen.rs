use crate::types::*;
use crate::check::*;

// Get pseudo-legal moves for a piece (does NOT check if king is left in check)
pub fn get_piece_moves(piece: i8, r: usize, c: usize, board: &Board) -> Vec<(usize, usize)> {
    let mut moves = Vec::new();
    let a = abs_piece(piece);
    let owner = piece_owner(piece);
    let dir: i32 = if owner == SENTE { -1 } else { 1 };

    let add_move = |moves: &mut Vec<(usize, usize)>, nr: i32, nc: i32| {
        if nr < 0 || nr >= BOARD_ROWS as i32 || nc < 0 || nc >= BOARD_COLS as i32 { return; }
        let target = board_get(board, nr as usize, nc as usize);
        if piece_owner(target) == owner { return; }
        moves.push((nr as usize, nc as usize));
    };

    let add_line = |moves: &mut Vec<(usize, usize)>, dr: i32, dc: i32| {
        let mut nr = r as i32 + dr;
        let mut nc = c as i32 + dc;
        while nr >= 0 && nr < BOARD_ROWS as i32 && nc >= 0 && nc < BOARD_COLS as i32 {
            let target = board_get(board, nr as usize, nc as usize);
            if piece_owner(target) == owner { break; }
            moves.push((nr as usize, nc as usize));
            if target != EMPTY { break; }
            nr += dr; nc += dc;
        }
    };

    match a {
        FU => { add_move(&mut moves, r as i32 + dir, c as i32); }
        KY => { add_line(&mut moves, dir, 0); }
        KE => {
            add_move(&mut moves, r as i32 + 2*dir, c as i32 - 1);
            add_move(&mut moves, r as i32 + 2*dir, c as i32 + 1);
        }
        GI => {
            add_move(&mut moves, r as i32 + dir, c as i32 - 1);
            add_move(&mut moves, r as i32 + dir, c as i32);
            add_move(&mut moves, r as i32 + dir, c as i32 + 1);
            add_move(&mut moves, r as i32 - dir, c as i32 - 1);
            add_move(&mut moves, r as i32 - dir, c as i32 + 1);
        }
        KI | TO | NY | NK | NG => {
            add_move(&mut moves, r as i32 + dir, c as i32 - 1);
            add_move(&mut moves, r as i32 + dir, c as i32);
            add_move(&mut moves, r as i32 + dir, c as i32 + 1);
            add_move(&mut moves, r as i32, c as i32 - 1);
            add_move(&mut moves, r as i32, c as i32 + 1);
            add_move(&mut moves, r as i32 - dir, c as i32);
        }
        KA => {
            add_line(&mut moves, -1, -1); add_line(&mut moves, -1, 1);
            add_line(&mut moves, 1, -1);  add_line(&mut moves, 1, 1);
        }
        HI => {
            add_line(&mut moves, -1, 0); add_line(&mut moves, 1, 0);
            add_line(&mut moves, 0, -1); add_line(&mut moves, 0, 1);
        }
        UM => {
            add_line(&mut moves, -1, -1); add_line(&mut moves, -1, 1);
            add_line(&mut moves, 1, -1);  add_line(&mut moves, 1, 1);
            add_move(&mut moves, r as i32 - 1, c as i32);
            add_move(&mut moves, r as i32 + 1, c as i32);
            add_move(&mut moves, r as i32, c as i32 - 1);
            add_move(&mut moves, r as i32, c as i32 + 1);
        }
        RY => {
            add_line(&mut moves, -1, 0); add_line(&mut moves, 1, 0);
            add_line(&mut moves, 0, -1); add_line(&mut moves, 0, 1);
            add_move(&mut moves, r as i32 - 1, c as i32 - 1);
            add_move(&mut moves, r as i32 - 1, c as i32 + 1);
            add_move(&mut moves, r as i32 + 1, c as i32 - 1);
            add_move(&mut moves, r as i32 + 1, c as i32 + 1);
        }
        KO => {
            // 子: 8方向に最大3マス（飛び越え不可）
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    if dr == 0 && dc == 0 { continue; }
                    for step in 1..=3 {
                        let nr = r as i32 + dr * step;
                        let nc = c as i32 + dc * step;
                        if nr < 0 || nr >= BOARD_ROWS as i32 || nc < 0 || nc >= BOARD_COLS as i32 { break; }
                        let target = board_get(board, nr as usize, nc as usize);
                        if piece_owner(target) == owner { break; }
                        moves.push((nr as usize, nc as usize));
                        if target != EMPTY { break; }
                    }
                }
            }
        }
        WK => {
            // 若（成子）: 角+飛車（クイーン）
            add_line(&mut moves, -1, -1); add_line(&mut moves, -1, 1);
            add_line(&mut moves, 1, -1);  add_line(&mut moves, 1, 1);
            add_line(&mut moves, -1, 0);  add_line(&mut moves, 1, 0);
            add_line(&mut moves, 0, -1);  add_line(&mut moves, 0, 1);
        }
        NIN => {
            // 妊: 動けない
        }
        OU | HM => {
            // 王・妃: 8方向1マス
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    if dr == 0 && dc == 0 { continue; }
                    add_move(&mut moves, r as i32 + dr, c as i32 + dc);
                }
            }
        }
        _ => {}
    }
    moves
}

pub fn must_promote(piece: i8, tr: usize) -> bool {
    let a = abs_piece(piece);
    let owner = piece_owner(piece);
    if owner == SENTE {
        if a == FU || a == KY { return tr == 0; }
        if a == KE { return tr <= 1; }
    } else {
        if a == FU || a == KY { return tr == 8; }
        if a == KE { return tr >= 7; }
    }
    false
}

pub fn should_ask_promote(piece: i8, fr: usize, tr: usize) -> bool {
    if promote_piece(abs_piece(piece)).is_none() { return false; }
    let owner = piece_owner(piece);
    if owner == SENTE { return fr <= 2 || tr <= 2; }
    fr >= 6 || tr >= 6
}

pub fn has_nifu(board: &Board, col: usize, owner: i8) -> bool {
    for r in 0..BOARD_ROWS {
        if board_get(board, r, col) == owner * FU { return true; }
    }
    false
}

pub fn get_drop_squares(board: &Board, piece_type: i8, owner: i8, s_hand: &Hand, g_hand: &Hand, variant: GameVariant) -> Vec<(usize, usize)> {
    let mut squares = Vec::new();
    for r in 0..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            if board_get(board, r, c) != EMPTY { continue; }
            if owner == SENTE {
                if (piece_type == FU || piece_type == KY) && r == 0 { continue; }
                if piece_type == KE && r <= 1 { continue; }
            } else {
                if (piece_type == FU || piece_type == KY) && r == 8 { continue; }
                if piece_type == KE && r >= 7 { continue; }
            }
            if piece_type == FU && has_nifu(board, c, owner) { continue; }
            // Pawn drop mate check
            if piece_type == FU {
                let mut nb = *board;
                board_set(&mut nb, r, c, owner * FU);
                if is_in_check(&nb, -owner) {
                    if is_checkmate_position(&nb, -owner, s_hand, g_hand, variant) { continue; }
                }
            }
            // Check that drop doesn't leave own king in check
            let mut nb = *board;
            board_set(&mut nb, r, c, owner * piece_type);
            if is_in_check(&nb, owner) { continue; }
            squares.push((r, c));
        }
    }
    squares
}

// Ouke: can promote KI to NIN if adjacent to OU
pub fn can_promote_to_nin(board: &Board, r: usize, c: usize, owner: i8) -> bool {
    let piece = board_get(board, r, c);
    if abs_piece(piece) != KI || piece_owner(piece) != owner { return false; }
    let king_pos = find_king(board, owner);
    match king_pos {
        Some((kr, kc)) => {
            let dr = (r as i32 - kr as i32).unsigned_abs();
            let dc = (c as i32 - kc as i32).unsigned_abs();
            dr <= 1 && dc <= 1
        }
        None => false,
    }
}

pub fn get_all_legal_moves(board: &Board, owner: i8, s_hand: &Hand, g_hand: &Hand, variant: GameVariant) -> Vec<ShogiMove> {
    let mut moves = Vec::with_capacity(128);
    let hand = if owner == SENTE { s_hand } else { g_hand };

    // Board moves
    for r in 0..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            let piece = board_get(board, r, c);
            if piece_owner(piece) != owner { continue; }
            // Ouke: NIN is immobile
            if variant == GameVariant::Ouke && abs_piece(piece) == NIN { continue; }

            let targets = get_piece_moves(piece, r, c, board);
            let can_prom = promote_piece(abs_piece(piece)).is_some();
            for (tr, tc) in targets {
                if !is_legal_move(board, r, c, tr, tc, owner) { continue; }
                let promote = should_ask_promote(piece, r, tr) && can_prom;
                let must = must_promote(piece, tr);
                if promote && !must {
                    moves.push(ShogiMove::new_move(r as u8, c as u8, tr as u8, tc as u8, true));
                    moves.push(ShogiMove::new_move(r as u8, c as u8, tr as u8, tc as u8, false));
                } else if must {
                    moves.push(ShogiMove::new_move(r as u8, c as u8, tr as u8, tc as u8, true));
                } else {
                    moves.push(ShogiMove::new_move(r as u8, c as u8, tr as u8, tc as u8, false));
                }
            }
        }
    }

    // Drop moves
    for (i, &pt) in HAND_PIECE_TYPES.iter().enumerate() {
        if hand[i] == 0 { continue; }
        let squares = get_drop_squares(board, pt, owner, s_hand, g_hand, variant);
        for (r, c) in squares {
            moves.push(ShogiMove::new_drop(pt, r as u8, c as u8));
        }
    }

    // Ouke special moves (only when not in check)
    if variant == GameVariant::Ouke && !is_in_check(board, owner) {
        // KI -> NIN promotion
        for r in 0..BOARD_ROWS {
            for c in 0..BOARD_COLS {
                let piece = board_get(board, r, c);
                if piece_owner(piece) == owner && abs_piece(piece) == KI {
                    if can_promote_to_nin(board, r, c, owner) {
                        moves.push(ShogiMove::new_promote_nin(r as u8, c as u8));
                    }
                }
            }
        }
        // NIN -> KI abort
        for r in 0..BOARD_ROWS {
            for c in 0..BOARD_COLS {
                let piece = board_get(board, r, c);
                if piece_owner(piece) == owner && abs_piece(piece) == NIN {
                    moves.push(ShogiMove::new_abort_nin(r as u8, c as u8));
                }
            }
        }
    }

    moves
}

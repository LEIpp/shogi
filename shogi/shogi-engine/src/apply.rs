use crate::types::*;

pub fn add_capture_to_hand(captured: i8, hand: &mut Hand, variant: GameVariant) {
    if captured == EMPTY { return; }
    let a = abs_piece(captured);
    if variant == GameVariant::Ouke && (a == NIN || a == HM) {
        // 妊/妃 → 金として手駒
        if let Some(idx) = hand_index(KI) {
            hand[idx] += 1;
        }
    } else if variant == GameVariant::Ouke && (a == KO || a == WK) {
        // 子・若は手駒にできない
    } else {
        let bt = base_type(captured);
        if let Some(idx) = hand_index(bt) {
            hand[idx] += 1;
        }
    }
}

pub fn apply_move(board: &Board, m: &ShogiMove, s_hand: &Hand, g_hand: &Hand, owner: i8, variant: GameVariant) -> (Board, Hand, Hand) {
    let mut nb = *board;
    let mut nsh = *s_hand;
    let mut ngh = *g_hand;
    let my_hand = if owner == SENTE { &mut nsh } else { &mut ngh };

    match m.move_type {
        MOVE_TYPE_MOVE => {
            let fr = m.fr as usize;
            let fc = m.fc as usize;
            let tr = m.tr as usize;
            let tc = m.tc as usize;
            let captured = board_get(&nb, tr, tc);
            let mut piece = board_get(&nb, fr, fc);
            board_set(&mut nb, fr, fc, EMPTY);
            if m.promote {
                let a = abs_piece(piece);
                if let Some(promoted) = promote_piece(a) {
                    piece = owner * promoted;
                }
            }
            board_set(&mut nb, tr, tc, piece);
            add_capture_to_hand(captured, my_hand, variant);
        }
        MOVE_TYPE_DROP => {
            let tr = m.tr as usize;
            let tc = m.tc as usize;
            board_set(&mut nb, tr, tc, owner * m.piece);
            if let Some(idx) = hand_index(m.piece) {
                if my_hand[idx] > 0 { my_hand[idx] -= 1; }
            }
        }
        MOVE_TYPE_PROMOTE_NIN => {
            // 金 → 妊
            let tr = m.tr as usize;
            let tc = m.tc as usize;
            board_set(&mut nb, tr, tc, owner * NIN);
        }
        MOVE_TYPE_ABORT_NIN => {
            // 妊 → 金
            let tr = m.tr as usize;
            let tc = m.tc as usize;
            board_set(&mut nb, tr, tc, owner * KI);
        }
        _ => {}
    }

    (nb, nsh, ngh)
}

use crate::types::*;

pub fn evaluate(board: &Board, s_hand: &Hand, g_hand: &Hand, variant: GameVariant) -> i32 {
    let mut score: i32 = 0;

    for r in 0..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            let p = board_get(board, r, c);
            if p == EMPTY { continue; }
            let owner = piece_owner(p);
            let a = abs_piece(p);
            let mut val = piece_value(a);

            // FU positional bonus
            if a == FU {
                if owner == SENTE {
                    val += ((6 - r as i32) * 2) as i32;
                } else {
                    val += ((r as i32 - 2) * 2) as i32;
                }
            }

            // KO positional bonus (closer to enemy = better)
            if a == KO {
                if owner == SENTE {
                    val += (8 - r as i32) * 15;
                } else {
                    val += r as i32 * 15;
                }
            }

            // NIN penalty if front is blocked (ouke only)
            if a == NIN && variant == GameVariant::Ouke {
                let dir: i32 = if owner == SENTE { -1 } else { 1 };
                let front_r = r as i32 + dir;
                if front_r < 0 || front_r >= BOARD_ROWS as i32 {
                    val -= 200; // edge = guaranteed birth failure
                } else if piece_owner(board_get(board, front_r as usize, c)) == owner {
                    val -= 150; // blocked by ally
                }
            }

            // OU center distance penalty
            if a == OU {
                let center_dist = (c as i32 - (BOARD_COLS as i32 / 2)).unsigned_abs() as i32;
                val += center_dist * 5;
            }

            score += owner as i32 * val;
        }
    }

    // Hand piece values (85% of face value)
    for (i, &pt) in HAND_PIECE_TYPES.iter().enumerate() {
        let pv = piece_value(pt);
        score += (s_hand[i] as i32) * (pv * 85 / 100);
        score -= (g_hand[i] as i32) * (pv * 85 / 100);
    }

    score
}

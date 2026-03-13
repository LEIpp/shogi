// Piece type constants (matches JS)
pub const EMPTY: i8 = 0;
pub const FU: i8 = 1;
pub const KY: i8 = 2;
pub const KE: i8 = 3;
pub const GI: i8 = 4;
pub const KI: i8 = 5;
pub const KA: i8 = 6;
pub const HI: i8 = 7;
pub const OU: i8 = 8;
pub const TO: i8 = 9;
pub const NY: i8 = 10;
pub const NK: i8 = 11;
pub const NG: i8 = 12;
pub const UM: i8 = 13;
pub const RY: i8 = 14;
pub const HM: i8 = 15;
pub const KO: i8 = 16;
pub const WK: i8 = 17;
pub const NIN: i8 = 18;

pub const SENTE: i8 = 1;
pub const GOTE: i8 = -1;

pub const BOARD_ROWS: usize = 9;
pub const BOARD_COLS: usize = 9;
pub const BOARD_SIZE: usize = BOARD_ROWS * BOARD_COLS; // 81

// Board: flat array of signed i8. board[r*9+c] = owner * piece_type
pub type Board = [i8; BOARD_SIZE];

// Hand pieces ordered: [HI, KA, KI, GI, KE, KY, FU, HM]
pub const HAND_PIECE_TYPES: [i8; 8] = [HI, KA, KI, GI, KE, KY, FU, HM];
pub type Hand = [u8; 8];

// Hand index lookup
pub fn hand_index(piece_type: i8) -> Option<usize> {
    match piece_type {
        HI => Some(0),
        KA => Some(1),
        KI => Some(2),
        GI => Some(3),
        KE => Some(4),
        KY => Some(5),
        FU => Some(6),
        HM => Some(7),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameVariant {
    Normal,
    Ouke,
}

impl GameVariant {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => GameVariant::Ouke,
            _ => GameVariant::Normal,
        }
    }
}

// Move types
pub const MOVE_TYPE_MOVE: u8 = 0;
pub const MOVE_TYPE_DROP: u8 = 1;
pub const MOVE_TYPE_PROMOTE_NIN: u8 = 2;
pub const MOVE_TYPE_ABORT_NIN: u8 = 3;

#[derive(Clone, Copy, Debug)]
pub struct ShogiMove {
    pub move_type: u8,
    pub fr: u8,
    pub fc: u8,
    pub tr: u8,
    pub tc: u8,
    pub promote: bool,
    pub piece: i8, // for drops
}

impl ShogiMove {
    pub fn new_move(fr: u8, fc: u8, tr: u8, tc: u8, promote: bool) -> Self {
        ShogiMove { move_type: MOVE_TYPE_MOVE, fr, fc, tr, tc, promote, piece: 0 }
    }
    pub fn new_drop(piece: i8, tr: u8, tc: u8) -> Self {
        ShogiMove { move_type: MOVE_TYPE_DROP, fr: 255, fc: 255, tr, tc, promote: false, piece }
    }
    pub fn new_promote_nin(tr: u8, tc: u8) -> Self {
        ShogiMove { move_type: MOVE_TYPE_PROMOTE_NIN, fr: 255, fc: 255, tr, tc, promote: false, piece: 0 }
    }
    pub fn new_abort_nin(tr: u8, tc: u8) -> Self {
        ShogiMove { move_type: MOVE_TYPE_ABORT_NIN, fr: 255, fc: 255, tr, tc, promote: false, piece: 0 }
    }
}

// Promotion map
pub fn promote_piece(p: i8) -> Option<i8> {
    match p {
        FU => Some(TO),
        KY => Some(NY),
        KE => Some(NK),
        GI => Some(NG),
        KA => Some(UM),
        HI => Some(RY),
        KO => Some(WK),
        _ => None,
    }
}

// Unpromote map
pub fn unpromote_piece(p: i8) -> Option<i8> {
    match p {
        TO => Some(FU),
        NY => Some(KY),
        NK => Some(KE),
        NG => Some(GI),
        UM => Some(KA),
        RY => Some(HI),
        WK => Some(KO),
        _ => None,
    }
}

// Base type (unpromoted form, or self if not promoted)
pub fn base_type(p: i8) -> i8 {
    let a = p.unsigned_abs() as i8;
    unpromote_piece(a).unwrap_or(a)
}

// Piece values for evaluation
pub fn piece_value(p: i8) -> i32 {
    match p {
        FU => 100, KY => 300, KE => 350, GI => 450, KI => 500, KA => 650, HI => 700, OU => 10000,
        TO => 420, NY => 400, NK => 410, NG => 460, UM => 850, RY => 950,
        HM => 600, KO => 400, WK => 1200, NIN => 650,
        _ => 0,
    }
}

// Board helpers
#[inline(always)]
pub fn board_get(board: &Board, r: usize, c: usize) -> i8 {
    board[r * BOARD_COLS + c]
}

#[inline(always)]
pub fn board_set(board: &mut Board, r: usize, c: usize, val: i8) {
    board[r * BOARD_COLS + c] = val;
}

#[inline(always)]
pub fn piece_owner(val: i8) -> i8 {
    if val > 0 { SENTE } else if val < 0 { GOTE } else { 0 }
}

#[inline(always)]
pub fn abs_piece(val: i8) -> i8 {
    val.unsigned_abs() as i8
}

pub fn is_promoted(p: i8) -> bool {
    let a = abs_piece(p);
    unpromote_piece(a).is_some()
}

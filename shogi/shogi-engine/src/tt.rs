use crate::types::*;

const TT_EXACT: u8 = 0;
const TT_ALPHA: u8 = 1;
const TT_BETA: u8 = 2;

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub hash: u32,
    pub depth: u8,
    pub score: i32,
    pub flag: u8,
    pub best_move: Option<usize>, // index into moves array (not stored across calls)
    pub gen: u16,
}

pub struct TranspositionTable {
    table: Vec<Option<TTEntry>>,
    mask: usize,
    pub generation: u16,
}

impl TranspositionTable {
    pub fn new(size_power: u8) -> Self {
        let size = 1usize << size_power;
        TranspositionTable {
            table: vec![None; size],
            mask: size - 1,
            generation: 0,
        }
    }

    pub fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        // Don't reallocate, just bump generation
    }

    pub fn probe(&self, hash: u32, depth: u8, alpha: i32, beta: i32) -> Option<(i32, Option<usize>)> {
        let idx = (hash as usize) & self.mask;
        let entry = self.table[idx].as_ref()?;
        if entry.hash != hash || entry.gen != self.generation { return None; }
        if entry.depth < depth { return None; }

        match entry.flag {
            TT_EXACT => Some((entry.score, entry.best_move)),
            TT_ALPHA if entry.score <= alpha => Some((alpha, entry.best_move)),
            TT_BETA if entry.score >= beta => Some((beta, entry.best_move)),
            _ => None,
        }
    }

    // Get best move from TT even if depth/bounds don't match
    pub fn get_best_move(&self, hash: u32) -> Option<usize> {
        let idx = (hash as usize) & self.mask;
        let entry = self.table[idx].as_ref()?;
        if entry.hash == hash { entry.best_move } else { None }
    }

    pub fn store(&mut self, hash: u32, depth: u8, score: i32, flag: u8, best_move: Option<usize>) {
        let idx = (hash as usize) & self.mask;
        let should_replace = match &self.table[idx] {
            None => true,
            Some(e) => e.gen != self.generation || e.depth <= depth,
        };
        if should_replace {
            self.table[idx] = Some(TTEntry {
                hash, depth, score, flag,
                best_move,
                gen: self.generation,
            });
        }
    }

    pub const FLAG_EXACT: u8 = TT_EXACT;
    pub const FLAG_ALPHA: u8 = TT_ALPHA;
    pub const FLAG_BETA: u8 = TT_BETA;
}

pub fn hash_board(board: &Board, s_hand: &Hand, g_hand: &Hand, maximizing: bool) -> u32 {
    let mut h: i32 = if maximizing { 1 } else { 0 };
    for r in 0..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            h = h.wrapping_mul(31).wrapping_add(board_get(board, r, c) as i32 + 50);
        }
    }
    for (i, _) in HAND_PIECE_TYPES.iter().enumerate() {
        h = h.wrapping_mul(31).wrapping_add(s_hand[i] as i32);
        h = h.wrapping_mul(31).wrapping_add(g_hand[i] as i32);
    }
    h as u32
}

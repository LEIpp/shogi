use crate::types::*;
use crate::movegen::*;
use crate::eval::*;
use crate::apply::*;
use crate::check::*;

const EXPLORATION_C: f64 = 1.414; // sqrt(2) for UCB1

struct MctsNode {
    board: Board,
    s_hand: Hand,
    g_hand: Hand,
    maximizing: bool,
    visit_count: u32,
    total_value: f64,
    children: Vec<MctsNode>,
    unexpanded_moves: Vec<ShogiMove>,
    move_from_parent: Option<ShogiMove>,
}

impl MctsNode {
    fn new(
        board: Board,
        s_hand: Hand,
        g_hand: Hand,
        maximizing: bool,
        move_from_parent: Option<ShogiMove>,
        variant: GameVariant,
    ) -> Self {
        let owner = if maximizing { SENTE } else { GOTE };
        let moves = get_all_legal_moves(&board, owner, &s_hand, &g_hand, variant);
        MctsNode {
            board,
            s_hand,
            g_hand,
            maximizing,
            visit_count: 0,
            total_value: 0.0,
            children: Vec::new(),
            unexpanded_moves: moves,
            move_from_parent,
        }
    }

    fn is_terminal(&self) -> bool {
        self.unexpanded_moves.is_empty() && self.children.is_empty()
    }

    fn is_fully_expanded(&self) -> bool {
        self.unexpanded_moves.is_empty()
    }
}

/// Normalize evaluation score to [0, 1] using sigmoid
fn normalize_score(score: i32) -> f64 {
    1.0 / (1.0 + (-score as f64 / 1000.0).exp())
}

/// UCB1 value for child selection
fn ucb1(child: &MctsNode, parent_visits: u32, parent_maximizing: bool) -> f64 {
    if child.visit_count == 0 {
        return f64::MAX;
    }

    let exploitation = child.total_value / child.visit_count as f64;
    let adjusted_exploitation = if parent_maximizing {
        exploitation
    } else {
        1.0 - exploitation
    };

    let exploration = EXPLORATION_C
        * ((parent_visits as f64).ln() / child.visit_count as f64).sqrt();

    adjusted_exploitation + exploration
}

/// Select path through tree using UCB1
fn select_path(root: &MctsNode) -> Vec<usize> {
    let mut path = Vec::new();
    let mut node = root;

    while node.is_fully_expanded() && !node.children.is_empty() {
        let mut best_idx = 0;
        let mut best_ucb = f64::NEG_INFINITY;

        for (i, child) in node.children.iter().enumerate() {
            let u = ucb1(child, node.visit_count, node.maximizing);
            if u > best_ucb {
                best_ucb = u;
                best_idx = i;
            }
        }

        path.push(best_idx);
        node = &node.children[best_idx];
    }

    path
}

/// Navigate to a mutable node using index path
fn get_node_mut<'a>(root: &'a mut MctsNode, path: &[usize]) -> &'a mut MctsNode {
    let mut node = root;
    for &idx in path {
        node = &mut node.children[idx];
    }
    node
}

/// Expand one unexpanded move from a node, returns child index
fn expand_node(node: &mut MctsNode, variant: GameVariant) -> usize {
    let m = node.unexpanded_moves.pop().unwrap();
    let owner = if node.maximizing { SENTE } else { GOTE };
    let (nb, nsh, ngh) = apply_move(&node.board, &m, &node.s_hand, &node.g_hand, owner, variant);
    let child = MctsNode::new(nb, nsh, ngh, !node.maximizing, Some(m), variant);
    node.children.push(child);
    node.children.len() - 1
}

/// Backpropagate value up the tree along a path
fn backpropagate(root: &mut MctsNode, path: &[usize], value: f64) {
    root.visit_count += 1;
    root.total_value += value;

    let mut node = root;
    for &idx in path {
        node = &mut node.children[idx];
        node.visit_count += 1;
        node.total_value += value;
    }
}

/// Maximum number of moves to evaluate at each simulation step
const SIM_MOVE_LIMIT: usize = 12;
/// Number of plies for guided simulation playout
const SIM_DEPTH: u8 = 4;

/// Quick score for prioritizing which moves to evaluate during simulation
/// (captures and promotions first)
fn sim_move_priority(m: &ShogiMove, board: &Board) -> i32 {
    let mut s: i32 = 0;
    if m.move_type == MOVE_TYPE_MOVE {
        let target = board_get(board, m.tr as usize, m.tc as usize);
        if target != EMPTY {
            s += 1000 + piece_value(abs_piece(target));
        }
    }
    if m.promote { s += 500; }
    if m.move_type == MOVE_TYPE_PROMOTE_NIN { s += 300; }
    s
}

/// Guided simulation: play SIM_DEPTH plies greedily using evaluation function,
/// then return sigmoid-normalized score of the final position.
fn simulate_value(node: &MctsNode, variant: GameVariant) -> f64 {
    if node.is_terminal() {
        let owner = if node.maximizing { SENTE } else { GOTE };
        if is_in_check(&node.board, owner) {
            if node.maximizing { 0.0 } else { 1.0 }
        } else {
            0.5
        }
    } else {
        // Guided playout: greedily pick best-evaluated moves for a few plies
        let mut cur_board = node.board;
        let mut cur_sh = node.s_hand;
        let mut cur_gh = node.g_hand;
        let mut cur_maximizing = node.maximizing;

        for _ in 0..SIM_DEPTH {
            let owner = if cur_maximizing { SENTE } else { GOTE };
            let moves = get_all_legal_moves(&cur_board, owner, &cur_sh, &cur_gh, variant);
            if moves.is_empty() {
                // Terminal: checkmate or stalemate
                if is_in_check(&cur_board, owner) {
                    return if cur_maximizing { 0.0 } else { 1.0 };
                }
                return 0.5;
            }

            // Select top-K moves by priority, then pick the one with best eval
            let best_move = if moves.len() <= SIM_MOVE_LIMIT {
                // Few enough moves: evaluate all
                pick_best_move(&moves, &cur_board, &cur_sh, &cur_gh, owner, cur_maximizing, variant)
            } else {
                // Too many moves: pre-filter to top SIM_MOVE_LIMIT by capture/promotion priority
                let mut scored: Vec<(i32, usize)> = moves.iter().enumerate()
                    .map(|(i, m)| (sim_move_priority(m, &cur_board), i))
                    .collect();
                scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
                scored.truncate(SIM_MOVE_LIMIT);
                let subset: Vec<&ShogiMove> = scored.iter().map(|&(_, i)| &moves[i]).collect();
                pick_best_move_subset(&subset, &cur_board, &cur_sh, &cur_gh, owner, cur_maximizing, variant)
            };

            let (nb, nsh, ngh) = apply_move(&cur_board, &best_move, &cur_sh, &cur_gh, owner, variant);
            cur_board = nb;
            cur_sh = nsh;
            cur_gh = ngh;
            cur_maximizing = !cur_maximizing;
        }

        let score = evaluate(&cur_board, &cur_sh, &cur_gh, variant);
        normalize_score(score)
    }
}

/// Pick the best move from all moves by evaluation
fn pick_best_move(
    moves: &[ShogiMove], board: &Board, s_hand: &Hand, g_hand: &Hand,
    owner: i8, maximizing: bool, variant: GameVariant,
) -> ShogiMove {
    let mut best_score = if maximizing { i32::MIN } else { i32::MAX };
    let mut best_idx = 0;
    for (i, m) in moves.iter().enumerate() {
        let (nb, nsh, ngh) = apply_move(board, m, s_hand, g_hand, owner, variant);
        let score = evaluate(&nb, &nsh, &ngh, variant);
        if (maximizing && score > best_score) || (!maximizing && score < best_score) {
            best_score = score;
            best_idx = i;
        }
    }
    moves[best_idx]
}

/// Pick the best move from a subset of move references by evaluation
fn pick_best_move_subset(
    moves: &[&ShogiMove], board: &Board, s_hand: &Hand, g_hand: &Hand,
    owner: i8, maximizing: bool, variant: GameVariant,
) -> ShogiMove {
    let mut best_score = if maximizing { i32::MIN } else { i32::MAX };
    let mut best_idx = 0;
    for (i, m) in moves.iter().enumerate() {
        let (nb, nsh, ngh) = apply_move(board, m, s_hand, g_hand, owner, variant);
        let score = evaluate(&nb, &nsh, &ngh, variant);
        if (maximizing && score > best_score) || (!maximizing && score < best_score) {
            best_score = score;
            best_idx = i;
        }
    }
    *moves[best_idx]
}

/// Core MCTS loop — runs simulations on root node, returns the root
fn run_mcts_core(
    board: &Board,
    s_hand: &Hand,
    g_hand: &Hand,
    simulations: u32,
    maximizing: bool,
    time_limit_ms: u32,
    variant: GameVariant,
) -> MctsNode {
    let mut root = MctsNode::new(*board, *s_hand, *g_hand, maximizing, None, variant);

    if root.unexpanded_moves.is_empty() {
        return root;
    }

    let start_time = js_sys::Date::now();
    let time_limit = time_limit_ms as f64;

    for iter in 0..simulations {
        // Time check every 64 iterations
        if time_limit > 0.0 && (iter & 63) == 0 && iter > 0 {
            if js_sys::Date::now() - start_time >= time_limit {
                break;
            }
        }

        // SELECT
        let mut path = select_path(&root);
        let leaf = get_node_mut(&mut root, &path);

        // EXPAND + SIMULATE
        let value = if !leaf.is_terminal() {
            if !leaf.is_fully_expanded() {
                let child_idx = expand_node(leaf, variant);
                path.push(child_idx);
                simulate_value(&leaf.children[child_idx], variant)
            } else {
                simulate_value(leaf, variant)
            }
        } else {
            simulate_value(leaf, variant)
        };

        // BACKPROPAGATE
        backpropagate(&mut root, &path, value);
    }

    root
}

/// Root child statistics for parallel aggregation
pub struct RootChildStat {
    pub m: ShogiMove,
    pub visits: u32,
    pub total_value: f64,
}

/// Main MCTS search — returns (score, best_move)
pub fn mcts_search(
    board: &Board,
    s_hand: &Hand,
    g_hand: &Hand,
    simulations: u32,
    maximizing: bool,
    time_limit_ms: u32,
    variant: GameVariant,
) -> (i32, Option<ShogiMove>) {
    let root = run_mcts_core(board, s_hand, g_hand, simulations, maximizing, time_limit_ms, variant);

    if root.children.is_empty() {
        let score = evaluate(board, s_hand, g_hand, variant);
        return (score, None);
    }

    let mut best_idx = 0;
    let mut best_visits = 0;
    for (i, child) in root.children.iter().enumerate() {
        if child.visit_count > best_visits {
            best_visits = child.visit_count;
            best_idx = i;
        }
    }

    let best_child = &root.children[best_idx];
    let score = evaluate(&best_child.board, &best_child.s_hand, &best_child.g_hand, variant);
    (score, best_child.move_from_parent)
}

/// MCTS search returning per-root-child statistics (for parallel aggregation)
pub fn mcts_search_root_stats(
    board: &Board,
    s_hand: &Hand,
    g_hand: &Hand,
    simulations: u32,
    maximizing: bool,
    time_limit_ms: u32,
    variant: GameVariant,
) -> Vec<RootChildStat> {
    let root = run_mcts_core(board, s_hand, g_hand, simulations, maximizing, time_limit_ms, variant);

    root.children.iter().filter_map(|c| {
        c.move_from_parent.map(|m| RootChildStat {
            m,
            visits: c.visit_count,
            total_value: c.total_value,
        })
    }).collect()
}

// ============================================================
// Persistent MCTS tree for pondering and incremental search
// ============================================================

static mut MCTS_TREE: Option<MctsNode> = None;
static mut MCTS_VARIANT: GameVariant = GameVariant::Normal;

fn moves_match(a: &ShogiMove, mt: u8, fr: u8, fc: u8, tr: u8, tc: u8, promote: bool, piece: i8) -> bool {
    a.move_type == mt && a.fr == fr && a.fc == fc && a.tr == tr && a.tc == tc
        && a.promote == promote && a.piece == piece
}

fn get_stats_inner(root: &MctsNode) -> Vec<RootChildStat> {
    root.children.iter().filter_map(|c| {
        c.move_from_parent.map(|m| RootChildStat {
            m,
            visits: c.visit_count,
            total_value: c.total_value,
        })
    }).collect()
}

/// Initialize a new persistent MCTS tree for the given position
pub fn mcts_init_tree(
    board: &Board, s_hand: &Hand, g_hand: &Hand,
    maximizing: bool, variant: GameVariant,
) {
    let root = MctsNode::new(*board, *s_hand, *g_hand, maximizing, None, variant);
    unsafe {
        MCTS_TREE = Some(root);
        MCTS_VARIANT = variant;
    }
}

/// Run a batch of simulations on the persistent tree, returns root child stats
pub fn mcts_run_batch(count: u32) -> Vec<RootChildStat> {
    unsafe {
        let variant = MCTS_VARIANT;
        if let Some(ref mut root) = MCTS_TREE {
            if root.is_terminal() {
                return Vec::new();
            }
            for _ in 0..count {
                let mut path = select_path(root);
                let leaf = get_node_mut(root, &path);
                let value = if !leaf.is_terminal() {
                    if !leaf.is_fully_expanded() {
                        let child_idx = expand_node(leaf, variant);
                        path.push(child_idx);
                        simulate_value(&leaf.children[child_idx], variant)
                    } else {
                        simulate_value(leaf, variant)
                    }
                } else {
                    simulate_value(leaf, variant)
                };
                backpropagate(root, &path, value);
            }
            get_stats_inner(root)
        } else {
            Vec::new()
        }
    }
}

/// Get current root child statistics without running simulations
pub fn mcts_get_stats() -> Vec<RootChildStat> {
    unsafe {
        if let Some(ref root) = MCTS_TREE {
            get_stats_inner(root)
        } else {
            Vec::new()
        }
    }
}

/// Descend the tree by applying a move (for pondering tree reuse).
/// Returns true if the child was found (expanded or unexpanded).
pub fn mcts_apply_move(
    move_type: u8, fr: u8, fc: u8, tr: u8, tc: u8, promote: bool, piece: i8,
) -> bool {
    unsafe {
        let variant = MCTS_VARIANT;
        if let Some(mut root) = MCTS_TREE.take() {
            // Search in expanded children
            let found_idx = root.children.iter().position(|c| {
                if let Some(ref m) = c.move_from_parent {
                    moves_match(m, move_type, fr, fc, tr, tc, promote, piece)
                } else {
                    false
                }
            });

            if let Some(idx) = found_idx {
                let child = root.children.swap_remove(idx);
                MCTS_TREE = Some(child);
                return true;
            }

            // Search in unexpanded moves
            let unexp_idx = root.unexpanded_moves.iter().position(|m| {
                moves_match(m, move_type, fr, fc, tr, tc, promote, piece)
            });

            if let Some(idx) = unexp_idx {
                let m = root.unexpanded_moves[idx];
                let owner = if root.maximizing { SENTE } else { GOTE };
                let (nb, nsh, ngh) = apply_move(&root.board, &m, &root.s_hand, &root.g_hand, owner, variant);
                let child = MctsNode::new(nb, nsh, ngh, !root.maximizing, Some(m), variant);
                MCTS_TREE = Some(child);
                return true;
            }

            // Move not found — clear tree
            MCTS_TREE = None;
            false
        } else {
            false
        }
    }
}

/// Clear the persistent tree
pub fn mcts_clear_tree() {
    unsafe { MCTS_TREE = None; }
}

/// Check if a persistent tree exists
pub fn mcts_has_tree() -> bool {
    unsafe { MCTS_TREE.is_some() }
}

/// Get the total visit count of the root node
pub fn mcts_root_visits() -> u32 {
    unsafe {
        if let Some(ref root) = MCTS_TREE {
            root.visit_count
        } else {
            0
        }
    }
}

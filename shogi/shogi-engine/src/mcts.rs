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
/// Positive scores (SENTE advantage) → closer to 1.0
fn normalize_score(score: i32) -> f64 {
    1.0 / (1.0 + (-score as f64 / 1000.0).exp())
}

/// UCB1 value for child selection
fn ucb1(child: &MctsNode, parent_visits: u32, parent_maximizing: bool) -> f64 {
    if child.visit_count == 0 {
        return f64::MAX; // always explore unvisited nodes
    }

    let exploitation = child.total_value / child.visit_count as f64;
    // If parent is maximizing, we want high-value children
    // If parent is minimizing, we want low-value children (invert)
    let adjusted_exploitation = if parent_maximizing {
        exploitation
    } else {
        1.0 - exploitation
    };

    let exploration = EXPLORATION_C
        * ((parent_visits as f64).ln() / child.visit_count as f64).sqrt();

    adjusted_exploitation + exploration
}

/// Select path through tree using UCB1, returns indices
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

/// Main MCTS search function
pub fn mcts_search(
    board: &Board,
    s_hand: &Hand,
    g_hand: &Hand,
    simulations: u32,
    maximizing: bool,
    time_limit_ms: u32,
    variant: GameVariant,
) -> (i32, Option<ShogiMove>) {
    let mut root = MctsNode::new(*board, *s_hand, *g_hand, maximizing, None, variant);

    // No legal moves
    if root.unexpanded_moves.is_empty() {
        let score = evaluate(board, s_hand, g_hand, variant);
        return (score, None);
    }

    let start_time = js_sys::Date::now();
    let time_limit = time_limit_ms as f64;

    for iter in 0..simulations {
        // Time check every 64 iterations
        if time_limit > 0.0 && (iter & 63) == 0 && iter > 0 {
            let now = js_sys::Date::now();
            if now - start_time >= time_limit {
                break;
            }
        }

        // SELECT
        let mut path = select_path(&root);

        // Get the selected node
        let leaf = get_node_mut(&mut root, &path);

        // EXPAND (if not terminal)
        let value = if !leaf.is_terminal() {
            if !leaf.is_fully_expanded() {
                let child_idx = expand_node(leaf, variant);
                path.push(child_idx);
                let child = &leaf.children[child_idx];

                // SIMULATE: use evaluate() with sigmoid normalization
                if child.is_terminal() {
                    // Terminal position
                    let owner = if child.maximizing { SENTE } else { GOTE };
                    if is_in_check(&child.board, owner) {
                        // Checkmate: previous player won
                        if child.maximizing {
                            0.0 // GOTE won (bad for SENTE/maximizing)
                        } else {
                            1.0 // SENTE won (good for maximizing)
                        }
                    } else {
                        0.5 // Stalemate
                    }
                } else {
                    let score = evaluate(&child.board, &child.s_hand, &child.g_hand, variant);
                    normalize_score(score)
                }
            } else {
                // Fully expanded but somehow reached here (shouldn't normally happen)
                let score = evaluate(&leaf.board, &leaf.s_hand, &leaf.g_hand, variant);
                normalize_score(score)
            }
        } else {
            // Terminal node evaluation
            let owner = if leaf.maximizing { SENTE } else { GOTE };
            if is_in_check(&leaf.board, owner) {
                if leaf.maximizing { 0.0 } else { 1.0 }
            } else {
                0.5
            }
        };

        // BACKPROPAGATE
        backpropagate(&mut root, &path, value);
    }

    // Select best move: child with most visits
    if root.children.is_empty() {
        // Only unexpanded moves remain (shouldn't happen after at least 1 iteration)
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

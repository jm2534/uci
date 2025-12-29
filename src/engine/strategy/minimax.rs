use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::Strategy;
use crate::game::board::Board;
use crate::game::moves::Move;

#[derive(Eq, PartialEq)]
struct ScoredMove {
    score: isize,
    attempt: Option<Move>,
}

impl Ord for ScoredMove {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for ScoredMove {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct Minimax {
    start: Instant,
    limit: Duration,
}

impl Default for Minimax {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            limit: Duration::from_secs(5),
        }
    }
}

impl Minimax {
    const MAX_DEPTH: usize = 20;

    fn should_stop(&self) -> bool {
        self.start.elapsed() >= self.limit
    }

    /// Returns the maximum utility value and associated move for `self.player`
    /// given `board` and the search parameters `alpha` and `beta`.
    fn maxvalue(
        &self,
        board: Board,
        depth: usize,
        mut alpha: isize,
        beta: isize,
    ) -> (ScoredMove, usize) {
        // stop condition check, checking time only ever other depth to reduce cost
        if board.winner().is_some() || depth == 0 || self.should_stop() {
            return (
                ScoredMove {
                    score: self.evaluate(&board),
                    attempt: None,
                },
                1,
            );
        }

        let mut best_score = isize::MIN;
        let mut best_move = None;
        let mut nodes = 0;
        for action in self.order(&board) {
            let mut child_board = board.clone();
            child_board.try_move(action).unwrap();

            // Get the score from opponent's perspective
            let (child_result, nodes_explored) = self.minvalue(child_board, depth - 1, alpha, beta);
            nodes += nodes_explored;

            if child_result.score > best_score {
                best_score = child_result.score;
                best_move = Some(action);
            }

            if best_score >= beta {
                return (
                    ScoredMove {
                        score: best_score,
                        attempt: best_move,
                    },
                    nodes,
                );
            }
            alpha = std::cmp::max(best_score, alpha);
        }

        (
            ScoredMove {
                score: best_score,
                attempt: best_move,
            },
            nodes,
        )
    }

    /// Returns the minimum utility value and associated move for `self.player`
    /// given `board` and the search parameters `alpha` and `beta`.
    fn minvalue(
        &self,
        board: Board,
        depth: usize,
        alpha: isize,
        mut beta: isize,
    ) -> (ScoredMove, usize) {
        // stop condition check, checking time only ever other depth to reduce cost
        if board.winner().is_some() || depth == 0 || self.should_stop() {
            return (
                ScoredMove {
                    score: self.evaluate(&board),
                    attempt: None,
                },
                1,
            );
        }

        let mut best_score = isize::MAX;
        let mut best_move = None;
        let mut nodes: usize = 0;
        for action in self.order(&board) {
            let mut child_board = board.clone();
            child_board.try_move(action).unwrap();

            let (child_result, nodes_explored) = self.maxvalue(child_board, depth - 1, alpha, beta);
            nodes += nodes_explored;

            if child_result.score < best_score {
                best_score = child_result.score;
                best_move = Some(action); // <-- Opponent's best move at this level
            }

            if best_score <= alpha {
                return (
                    ScoredMove {
                        score: best_score,
                        attempt: best_move,
                    },
                    nodes,
                );
            }
            beta = std::cmp::min(best_score, beta);
        }

        (
            ScoredMove {
                score: best_score,
                attempt: best_move,
            },
            nodes,
        )
    }
}

impl Strategy for Minimax {
    fn step(&mut self, board: &mut Board) -> Move {
        // iterative deepening
        self.start = Instant::now();
        let mut best_move = None;
        for depth in 1..=Minimax::MAX_DEPTH {
            if self.should_stop() {
                break; // time's up
            }

            let (result, nodes) = self.maxvalue(board.to_owned(), depth, isize::MIN, isize::MAX);
            if let Some(mv) = result.attempt {
                best_move = Some(mv);
                println!(
                    "info depth {} score {} move {}, nodes {}",
                    depth, result.score, mv, nodes
                );
            }
        }

        best_move.expect("No valid move found")
    }
}

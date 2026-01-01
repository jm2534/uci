use std::time::{Duration, Instant};

use super::Strategy;
use crate::game::Color;
use crate::game::board::Board;
use crate::game::moves::PseudoLegalMove;

#[derive(Eq, PartialEq)]
pub struct ScoredMove {
    pub score: isize,
    pub attempt: Option<PseudoLegalMove>,
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
    pub limit: Duration,
    pub max_depth: usize,
    // move_buffers: Vec<Vec<PseudoLegalMove>>,
}

impl Default for Minimax {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            limit: Duration::from_secs(5),
            max_depth: Self::DEFAULT_MAX_DEPTH,
            // move_buffers: vec![Vec::new(); Self::DEFAULT_MAX_DEPTH],
        }
    }
}

impl Minimax {
    pub const DEFAULT_MAX_DEPTH: usize = if cfg!(debug_assertions) { 5 } else { 20 };

    fn should_stop(&self) -> bool {
        // only in release builds
        if cfg!(debug_assertions) {
            false
        } else {
            self.start.elapsed() >= self.limit
        }
    }

    /// By default, lists the  moves available to the current player on the current `board`
    /// ordered by the results of `Strategy::evaluate` called on each position.
    pub fn order(&self, board: &mut Board, moves: &mut [PseudoLegalMove]) {
        moves.sort_by_key(|m| {
            // score captures higher than quiet moves
            match board.occupants()[m.stop().as_index()] {
                Some(piece) => -(self.value(piece.kind)), // negative for descending order
                None => 0,
            }
        });
    }

    /// Returns the maximum utility value and associated move for `self.player`
    /// given `board` and the search parameters `alpha` and `beta`.
    pub fn maxvalue(
        &self,
        board: &mut Board,
        depth: usize,
        mut alpha: isize,
        beta: isize,
        move_buffers: &mut [Vec<PseudoLegalMove>],
    ) -> (ScoredMove, usize) {
        // stop condition check, checking time only ever other depth to reduce cost
        if board.winner().is_some() || depth == 0 || self.should_stop() {
            return (
                ScoredMove {
                    score: self.evaluate(board),
                    attempt: None,
                },
                1,
            );
        }

        let (moves, child_buffers) = move_buffers.split_first_mut().unwrap();
        board.populate_legal_moves(Color::White, moves);
        self.order(board, moves);

        let mut best_score = isize::MIN;
        let mut best_move = None;
        let mut nodes: usize = 0;

        for action in moves.drain(..) {
            board
                .make_validated_move(action)
                .expect("Search make illegal move");

            let (child_result, nodes_explored) =
                self.minvalue(board, depth - 1, alpha, beta, child_buffers);
            board.unmake_move();

            // Get the score from opponent's perspective
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
    pub fn minvalue(
        &self,
        board: &mut Board,
        depth: usize,
        alpha: isize,
        mut beta: isize,
        move_buffers: &mut [Vec<PseudoLegalMove>],
    ) -> (ScoredMove, usize) {
        // stop condition check, checking time only ever other depth to reduce cost
        if board.winner().is_some() || depth == 0 || self.should_stop() {
            return (
                ScoredMove {
                    score: self.evaluate(board),
                    attempt: None,
                },
                1,
            );
        }

        let (moves, child_buffers) = move_buffers.split_first_mut().unwrap();
        board.populate_legal_moves(Color::Black, moves);
        self.order(board, moves);

        let mut best_score = isize::MAX;
        let mut best_move = None;
        let mut nodes: usize = 0;

        // note we reverse the order of moves to prioritize low-value moves for max
        for action in moves.drain(..).rev() {
            board
                .make_validated_move(action)
                .expect("Search make illegal move");

            let (child_result, nodes_explored) =
                self.maxvalue(board, depth - 1, alpha, beta, child_buffers);
            board.unmake_move();

            nodes += nodes_explored;

            if child_result.score < best_score {
                best_score = child_result.score;
                best_move = Some(action);
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
    fn step(&mut self, board: &mut Board) -> PseudoLegalMove {
        // iterative deepening
        self.start = Instant::now();
        let mut best_move = None;
        let mut buffers = vec![Vec::with_capacity(256); self.max_depth];
        for depth in 1..=self.max_depth {
            if self.should_stop() {
                break; // time's up
            }

            let (buffers, _) = buffers.split_at_mut(depth);
            let alpha = isize::MIN;
            let beta = isize::MAX;
            let (result, nodes) = match board.to_move() {
                Color::Black => self.minvalue(board, depth, alpha, beta, buffers),
                Color::White => self.maxvalue(board, depth, alpha, beta, buffers),
            };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimax() {
        let mut board =
            Board::try_from("r1b1kQnr/p1p2p2/4p3/8/4P3/5N2/1PPP1PPP/qNB1K2R b Kkq - 0 1").unwrap();
        Board::initialize();

        let mut strategy = Minimax::default();
        strategy.step(&mut board);
    }
}

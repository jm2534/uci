use std::time::{Duration, Instant};

use super::Strategy;
use crate::game::Color;
use crate::game::board::Board;
use crate::game::moves::Move;

#[derive(Eq, PartialEq)]
pub struct ScoredMove {
    pub score: isize,
    pub attempt: Option<Move>,
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

    /// By default, lists the  moves available to the current player on the current `board`
    /// ordered by the results of `Strategy::evaluate` called on each position.
    pub fn order(&self, board: &mut Board, moves: &mut [Move]) {
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
        mut board: Board,
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

        let mut moves = Vec::new();
        self.order(&mut board, &mut moves);

        let mut best_score = isize::MIN;
        let mut best_move = None;
        let mut nodes: usize = 0;

        for action in moves {
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
    pub fn minvalue(
        &self,
        mut board: Board,
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

        let mut moves = Vec::new();
        self.order(&mut board, &mut moves);

        let mut best_score = isize::MAX;
        let mut best_move = None;
        let mut nodes: usize = 0;

        for action in moves {
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

            let alpha = isize::MIN;
            let beta = isize::MAX;
            let (result, nodes) = match board.to_move() {
                Color::Black => self.minvalue(board.to_owned(), depth, alpha, beta),
                Color::White => self.maxvalue(board.to_owned(), depth, alpha, beta),
            };

            if let Some(mv) = result.attempt {
                best_move = Some(mv);
                // println!(
                //     "info depth {} score {} move {}, nodes {}",
                //     depth, result.score, mv, nodes
                // );
            }
        }

        best_move.expect("No valid move found")
    }
}

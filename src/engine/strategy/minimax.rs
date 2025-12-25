use super::Strategy;
use crate::game::board::Board;
use crate::game::moves::Move;

#[derive(Default)]
pub struct Minimax;

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

impl std::cmp::PartialOrd for ScoredMove {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl Minimax {
    /// Returns the maximum utility value and associated move for `self.player`
    /// given `board` and the search parameters `alpha` and `beta`.
    fn maxvalue(&self, board: Board, mut alpha: isize, beta: isize) -> ScoredMove {
        let mut result = ScoredMove {
            score: isize::MIN,
            attempt: None,
        };
        for action in self.order(&board) {
            let mut board = board.clone();
            board.try_move(action).unwrap();
            result = std::cmp::max(result, self.minvalue(board, alpha, beta));
            if result.score >= beta {
                return result;
            }
            alpha = std::cmp::max(result.score, alpha);
        }
        result
    }

    /// Returns the minimum utility value and associated move for `self.player`
    /// given `board` and the search parameters `alpha` and `beta`.
    fn minvalue(&self, board: Board, alpha: isize, mut beta: isize) -> ScoredMove {
        let mut result = ScoredMove {
            score: isize::MAX,
            attempt: None,
        };
        for action in self.order(&board) {
            let mut board = board.clone();
            board.try_move(action).unwrap();
            result = std::cmp::min(result, self.maxvalue(board, alpha, beta));
            if result.score <= alpha {
                return result;
            }
            beta = std::cmp::min(result.score, beta);
        }
        result
    }
}

impl Strategy for Minimax {
    fn step(&mut self, board: &mut Board) -> Move {
        let movement = self.maxvalue(board.to_owned(), isize::MIN, isize::MAX);
        movement.attempt.expect("Game is already over")
    }
}

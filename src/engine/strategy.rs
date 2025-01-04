use crate::game::{
    board::Board,
    color::Color,
    moves::Move,
    piece::{Piece, PieceKind},
};

pub mod minimax;

pub trait Strategy {
    /// By default, lists the  moves available to `color` on the current `board`
    /// ordered by the results of `Strategy::evaluate` called on each position.
    fn order(&self, board: Board, color: Color) -> Vec<Move> {
        let captures = board.possible_captures(color);
        let mut ordering: Vec<Move> = Vec::from_iter(captures.clone());

        let eval = |movement: &Move| {
            let target = board.occupant(movement.stop).unwrap();
            if target.kind == PieceKind::King {
                i32::MAX
            } else {
                self.value(&target)
            }
        };
        // Reverse sorting of moves
        ordering.sort_by(|m1, m2| eval(m2).partial_cmp(&eval(m1)).unwrap());

        // Add remaining moves that were not captures
        ordering.extend(&board.possible_moves(color) - &captures);
        ordering
    }

    /// Returns the current value of the board from the perspective of `color`.
    fn evaluate(&self, board: Board, color: Color) -> i32 {
        board
            .pieces_of(color)
            .iter()
            .fold(0, |sum, (piece, _)| sum + self.value(piece))
            - board
                .pieces_of(!color)
                .iter()
                .fold(0, |sum, (piece, _)| sum + self.value(piece))
    }

    /// Prescribes a numerical value for each piece.
    fn value(&self, piece: &Piece) -> i32 {
        match piece.kind {
            PieceKind::King => 0,
            PieceKind::Queen => 900,
            PieceKind::Knight => 300,
            PieceKind::Bishop => 300,
            PieceKind::Rook => 500,
            PieceKind::Pawn => 100,
        }
    }

    fn step(&mut self, board: Board) -> Move;
}

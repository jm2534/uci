use crate::game::{board::Board, moves::Move, piece::Piece};

pub mod minimax;

const VALUES: [i32; 6] = [
    100,      // Pawn
    300,      // Knight
    300,      // Bishop
    500,      // Rook
    900,      // Queen
    i32::MAX, // King
];

pub trait Strategy {
    /// By default, lists the  moves available to the current player on the current `board`
    /// ordered by the results of `Strategy::evaluate` called on each position.
    fn order(&self, board: &Board) -> impl IntoIterator<Item = Move> {
        let mut moves: Vec<Move> = board
            .legal_moves(board.to_move())
            .map(|(_, action)| action)
            .collect();

        moves.sort_by(|m1, m2| {
            self.value(board.occupants()[m1.stop()].unwrap())
                .cmp(&self.value(board.occupants()[m2.stop()].unwrap()))
        });
        moves.into_iter()
    }

    /// Prescribes a numerical value for each piece.
    fn value(&self, piece: Piece) -> i32 {
        VALUES[piece.kind as usize]
    }

    fn step(&mut self, board: &mut Board) -> Move;
}

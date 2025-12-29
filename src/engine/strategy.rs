use crate::game::{Color, board::Board, moves::Move, piece::PieceKind};

pub mod minimax;

const VALUES: [isize; 6] = [
    100, // Pawn
    300, // Knight
    300, // Bishop
    500, // Rook
    900, // Queen
    0,   // King
];

pub trait Strategy {
    fn evaluate(&self, board: &Board) -> isize {
        // terminal positions first
        if let Some(winner) = board.winner() {
            return if winner == board.to_move() {
                isize::MAX - 1 // We win
            } else {
                isize::MIN + 1 // We lose
            };
        }

        let mut value = 0;
        for kind in enum_iterator::all::<PieceKind>() {
            let positions = board.positions(kind);
            let n_white_pieces = (board.occupancy(Color::White) & positions).0.count_ones();
            let n_black_pieces = (board.occupancy(Color::Black) & positions).0.count_ones();

            value += n_white_pieces as isize * self.value(kind);
            value -= n_black_pieces as isize * self.value(kind);
        }

        value
    }

    /// By default, lists the  moves available to the current player on the current `board`
    /// ordered by the results of `Strategy::evaluate` called on each position.
    fn order(&self, board: &Board) -> impl IntoIterator<Item = Move> {
        let mut moves: Vec<Move> = board
            .legal_moves(board.to_move())
            .map(|(_, action)| action)
            .collect();

        moves.sort_by_key(|m| {
            // score captures higher than quiet moves
            match board.occupants()[m.stop().as_index()] {
                Some(piece) => -(self.value(piece.kind)), // negative for descending order
                None => 0,
            }
        });

        moves
    }

    /// Prescribes a numerical value for each piece.
    fn value(&self, piece: PieceKind) -> isize {
        VALUES[piece as usize]
    }

    fn step(&mut self, board: &mut Board) -> Move;
}

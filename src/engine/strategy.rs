use crate::game::{Color, board::Board, moves::Move, piece::PieceKind};

pub mod minimax;

const PIECE_VALUES: [isize; 6] = [
    100, // Pawn
    300, // Knight
    300, // Bishop
    500, // Rook
    900, // Queen
    0,   // King
];

// Pawn table - pawns are stronger in center and advanced
const PAWN_TABLE: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 50, 50, 50, 50, 50, 50, 50, 50, 10, 10, 20, 30, 30, 20, 10, 10, 5, 5,
    10, 25, 25, 10, 5, 5, 0, 0, 0, 20, 20, 0, 0, 0, 5, -5, -10, 0, 0, -10, -5, 5, 5, 10, 10, -20,
    -20, 10, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0,
];

// Knight table - knights are stronger in center
const KNIGHT_TABLE: [i32; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50, -40, -20, 0, 0, 0, 0, -20, -40, -30, 0, 10, 15, 15, 10,
    0, -30, -30, 5, 15, 20, 20, 15, 5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 10, 15, 15, 10,
    5, -30, -40, -20, 0, 5, 5, 0, -20, -40, -50, -40, -30, -30, -30, -30, -40, -50,
];

pub trait Strategy {
    fn evaluate(&self, board: &Board) -> isize {
        // terminal positions first
        if let Some(winner) = board.winner() {
            return match winner {
                Color::Black => isize::MIN,
                Color::White => isize::MAX,
            };
        }

        let mut value = 0;
        for kind in enum_iterator::all::<PieceKind>() {
            let piece_value = self.value(kind);
            let positions = board.positions(kind);
            let n_white_pieces = (board.occupancy(Color::White) & positions).0.count_ones();
            let n_black_pieces = (board.occupancy(Color::Black) & positions).0.count_ones();

            value += piece_value * (n_white_pieces as isize - n_black_pieces as isize);
        }

        value
    }

    /// Prescribes a numerical value for each piece.
    fn value(&self, piece: PieceKind) -> isize {
        PIECE_VALUES[piece as usize]
    }

    fn step(&mut self, board: &mut Board) -> Move;
}

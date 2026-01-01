use crate::game::{Color, board::Board, moves::PseudoLegalMove, piece::PieceKind};

pub mod minimax;

const PIECE_VALUES: [isize; 6] = [
    100, // Pawn
    300, // Knight
    300, // Bishop
    500, // Rook
    900, // Queen
    0,   // King
];

// Piece-Square Tables (from White's perspective)
// Values in centipawns (±50 is typical range)
// See https://www.chessprogramming.org/Simplified_Evaluation_Function

// Pawns: Reward advancement toward promotion (increasing values from rank 1 to rank 6)
const PAWN_TABLE: [isize; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, -20, -20, 10, 10, 5, 5, -5, -10, 0, 0, -10, -5, 5, 0, 0, 0,
    20, 20, 0, 0, 0, 5, 5, 10, 25, 25, 10, 5, 5, 10, 10, 20, 30, 30, 20, 10, 10, 50, 50, 50, 50,
    50, 50, 50, 50, 0, 0, 0, 0, 0, 0, 0, 0,
];

// Knights: Reward center squares, penalize edges
const KNIGHT_TABLE: [isize; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50, -40, -20, 0, 5, 5, 0, -20, -40, -30, 5, 10, 15, 15, 10,
    5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 15, 20, 20, 15, 5, -30, -30, 0, 10, 15, 15, 10,
    0, -30, -40, -20, 0, 0, 0, 0, -20, -40, -50, -40, -30, -30, -30, -30, -40, -50,
];

// Bishops: Reward long diagonals and center
const BISHOP_TABLE: [isize; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20, -10, 5, 0, 0, 0, 0, 5, -10, -10, 10, 10, 10, 10, 10,
    10, -10, -10, 0, 10, 10, 10, 10, 0, -10, -10, 5, 5, 10, 10, 5, 5, -10, -10, 0, 5, 10, 10, 5, 0,
    -10, -10, 0, 0, 0, 0, 0, 0, -10, -20, -10, -10, -10, -10, -10, -10, -20,
];

// Rooks: Reward 7th rank and center files
const ROOK_TABLE: [isize; 64] = [
    0, 0, 0, 5, 5, 0, 0, 0, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0,
    0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, 5, 10, 10, 10, 10, 10, 10, 5, 0, 0,
    0, 0, 0, 0, 0, 0,
];

// Queens: Slight center preference, avoid early development
const QUEEN_TABLE: [isize; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 0, 0, 0, 5, 0, -10, -10, 0, 5, 5, 5, 5, 5, -10,
    -5, 0, 5, 5, 5, 5, 0, 0, -5, 0, 5, 5, 5, 5, 0, -5, -10, 0, 5, 5, 5, 5, 0, -10, -10, 0, 0, 0, 0,
    0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
];

// Kings: Encourage castled position (corners on rank 1), penalize center
const KING_TABLE: [isize; 64] = [0; 64]; // TODO: midgame vs endgame positions

// 2D array indexed by PieceKind
const PIECE_SQUARE_TABLES: [[isize; 64]; 6] = [
    PAWN_TABLE,
    KNIGHT_TABLE,
    BISHOP_TABLE,
    ROOK_TABLE,
    QUEEN_TABLE,
    KING_TABLE,
];

// Helper function to flip square index vertically for Black pieces
// Black pieces should use PST values as if they were White pieces on the flipped board
#[inline]
fn pst_index(square: usize, color: Color) -> usize {
    match color {
        Color::White => square,
        Color::Black => 63 - square, // flip vertically
    }
}

pub trait Strategy {
    #[inline]
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
            let pst = PIECE_SQUARE_TABLES[kind as usize];

            // White pieces: add material + PST bonus
            for tile in (board.occupancy(Color::White) & positions).tiles() {
                let square = tile.as_index();
                value += piece_value + pst[pst_index(square, Color::White)];
            }

            // Black pieces: subtract material + PST bonus
            for tile in (board.occupancy(Color::Black) & positions).tiles() {
                let square = tile.as_index();
                value -= piece_value + pst[pst_index(square, Color::Black)];
            }
        }

        value
    }

    /// Prescribes a numerical value for each piece.
    #[inline]
    fn value(&self, piece: PieceKind) -> isize {
        PIECE_VALUES[piece as usize]
    }

    fn step(&mut self, board: &mut Board) -> PseudoLegalMove;
}

#[cfg(test)]
mod tests {
    use crate::game::{Move, board::Tile};

    use super::*;

    #[test]
    fn test_pst_evaluation_startpos_equal() {
        // Test that PST values are actually being applied
        Board::initialize();

        // Start position should have symmetric evaluation (both sides equal)
        let board = Board::new();
        let eval = minimax::Minimax::default().evaluate(&board);
        assert_eq!(
            eval, 0,
            "Starting position should evaluate to 0 (symmetric)"
        );
    }

    #[test]
    fn test_pst_pawn_evaluation() {
        let mut board = Board::new();
        Board::initialize();

        board.make_move(Move::new(Tile::E2, Tile::E4)).unwrap();
        let eval = minimax::Minimax::default().evaluate(&board);

        assert!(
            eval > 0,
            "Pawn move should increase evaluation past 0, found {}",
            eval
        );
    }

    #[test]
    fn test_pst_knight_evaluation() {
        let mut board = Board::new();
        Board::initialize();

        // Develop knight from b1 to c3
        board.make_move(Move::new(Tile::B1, Tile::C3)).unwrap();
        let eval = minimax::Minimax::default().evaluate(&board);

        assert!(
            eval > 0,
            "Knight development should increase evaluation past 0, found {}",
            eval
        );
    }
}

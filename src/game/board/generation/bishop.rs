//! Bishop move generation using bitboard techniques.

pub(crate) mod magic;

use super::super::{Bitset, Board};
use crate::game::{Color, board::tile::Tile};

impl Board {
    /// Generate all pseudo-legal moves for a bishop at the given tile.
    /// Returns a bitboard of valid destination squares (excluding squares occupied by own pieces).
    pub fn bishop_moves(&self, color: Color, tile: Tile) -> Bitset {
        let all_occupancy = self.occupancy[Color::White] | self.occupancy[Color::Black];
        let moves = magic::magic_moves(tile, all_occupancy);
        moves & !self.occupancy[color]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Color;

    #[test]
    fn test_bishop_unblocked_moves() {
        // sanity check for bishop on central square, no blockers
        magic::initialize();
        let mut board = Board::empty();
        board.to_move = Color::White;
        let moves = board.bishop_moves(board.to_move, Tile::D4);

        // Should have 13 squares on empty board from center
        assert_eq!(moves.len(), 13);

        // Check all four diagonals
        assert!(moves.contains(Tile::E5));
        assert!(moves.contains(Tile::F6));
        assert!(moves.contains(Tile::G7));
        assert!(moves.contains(Tile::H8));

        assert!(moves.contains(Tile::C5));
        assert!(moves.contains(Tile::B6));
        assert!(moves.contains(Tile::A7));

        assert!(moves.contains(Tile::E3));
        assert!(moves.contains(Tile::F2));
        assert!(moves.contains(Tile::G1));

        assert!(moves.contains(Tile::C3));
        assert!(moves.contains(Tile::B2));
        assert!(moves.contains(Tile::A1));
    }

    #[test]
    fn test_bishop_blocked_moves() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;
        let bishop_pos = Tile::D4;

        // Place blockers at f6 (northeast) and b2 (southwest)
        board.occupancy[Color::Black] = Tile::F6.as_bitset() | Tile::B2.as_bitset();

        let moves = board.bishop_moves(board.to_move, bishop_pos);

        // Can move to blocker squares (captures)
        assert!(moves.contains(Tile::F6));
        assert!(moves.contains(Tile::B2));

        // Cannot move beyond blockers
        assert!(!moves.contains(Tile::G7));
        assert!(!moves.contains(Tile::H8));
        assert!(!moves.contains(Tile::A1));

        // Can still move in unblocked directions
        assert!(moves.contains(Tile::E5));
        assert!(moves.contains(Tile::C5));
        assert!(moves.contains(Tile::E3));
        assert!(moves.contains(Tile::C3));
    }

    #[test]
    fn test_bishop_corner_position() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;

        // Test from a1
        let moves = board.bishop_moves(board.to_move, Tile::A1);
        assert_eq!(moves.len(), 7); // Only one diagonal

        // Test from h8
        let moves = board.bishop_moves(board.to_move, Tile::H8);
        assert_eq!(moves.len(), 7);
    }

    #[test]
    fn test_bishop_center_position() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;
        let bishop_pos = Tile::D4;

        let moves = board.bishop_moves(board.to_move, bishop_pos);

        // On empty board from center, should have 13 moves
        assert_eq!(moves.len(), 13);
    }

    #[test]
    fn test_bishop_captures() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;
        let bishop_pos = Tile::D4;

        // Place enemy pieces on all four diagonals
        board.occupancy[Color::Black] = Tile::F6.as_bitset()
            | Tile::B6.as_bitset()
            | Tile::F2.as_bitset()
            | Tile::B2.as_bitset();

        let moves = board.bishop_moves(board.to_move, bishop_pos);

        // Should be able to capture all enemy pieces
        assert!(moves.contains(Tile::F6));
        assert!(moves.contains(Tile::B6));
        assert!(moves.contains(Tile::F2));
        assert!(moves.contains(Tile::B2));
    }

    #[test]
    fn test_bishop_blocked_by_own_pieces() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;
        let bishop_pos = Tile::D4;

        // Place own pieces on two diagonals
        board.occupancy[Color::White] =
            Tile::D4.as_bitset() | Tile::F6.as_bitset() | Tile::C3.as_bitset();

        let moves = board.bishop_moves(board.to_move, bishop_pos);

        // Should NOT be able to move to squares occupied by own pieces
        assert!(!moves.contains(Tile::F6));
        assert!(!moves.contains(Tile::C3));

        // Should NOT be able to move beyond own pieces
        assert!(!moves.contains(Tile::G7));
        assert!(!moves.contains(Tile::H8));
        assert!(!moves.contains(Tile::B2));
        assert!(!moves.contains(Tile::A1));

        // Can still move in unblocked directions
        assert!(moves.contains(Tile::E5));
        assert!(moves.contains(Tile::C5));
        assert!(moves.contains(Tile::B6));
        assert!(moves.contains(Tile::A7));
        assert!(moves.contains(Tile::E3));
        assert!(moves.contains(Tile::F2));
        assert!(moves.contains(Tile::G1));
    }
}

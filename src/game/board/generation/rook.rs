//! Rook move generation using bitboard techniques.

pub(crate) mod magic;

use super::super::{Bitset, Board};
use crate::game::{Color, board::tile::Tile};

impl Board {
    /// Generate all pseudo-legal moves for a rook at the given tile.
    /// Returns a bitboard of valid destination squares (excluding squares occupied by own pieces).
    pub fn rook_moves(&self, color: Color, tile: Tile) -> Bitset {
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
    fn test_rook_unblocked_moves() {
        // sanity check for rook on central square, no blockers
        magic::initialize();
        let mut board = Board::empty();
        board.to_move = Color::White;
        let moves = board.rook_moves(board.to_move, Tile::D4);

        assert_eq!(
            moves & Board::RANK_MASKS[3],
            Board::RANK_MASKS[3] ^ Tile::D4
        );
        assert_eq!(
            moves & Board::FILE_MASKS[3],
            Board::FILE_MASKS[3] ^ Tile::D4
        );
    }

    #[test]
    fn test_rook_blocked_moves() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;
        let rook_pos = Tile::D4;

        // Place blockers at d6 (north) and f4 (east)
        board.occupancy[Color::Black] = Tile::D6.as_bitset() | Tile::F4.as_bitset();

        let moves = board.rook_moves(board.to_move, rook_pos);

        // Can move to blocker squares (captures)
        assert!(moves.contains(Tile::D6));
        assert!(moves.contains(Tile::F4));

        // Cannot move beyond blockers
        assert!(!moves.contains(Tile::D7));
        assert!(!moves.contains(Tile::D8));
        assert!(!moves.contains(Tile::G4));
        assert!(!moves.contains(Tile::H4));

        // Can still move in unblocked directions
        assert!(moves.contains(Tile::D1));
        assert!(moves.contains(Tile::A4));
    }

    #[test]
    fn test_rook_corner_position() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;

        // Test from a1
        let moves = board.rook_moves(board.to_move, Tile::A1);
        assert_eq!(moves.len(), 14); // 7 squares on rank + 7 on file

        // Test from h8
        let moves = board.rook_moves(board.to_move, Tile::H8);
        assert_eq!(moves.len(), 14);
    }

    #[test]
    fn test_rook_center_position() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;
        let rook_pos = Tile::D4;

        let moves = board.rook_moves(board.to_move, rook_pos);

        // On empty board from center, should have 14 moves
        assert_eq!(moves.len(), 14);
    }

    #[test]
    fn test_rook_captures() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;
        let rook_pos = Tile::D4;

        // Place enemy pieces that can be captured
        board.occupancy[Color::Black] = Tile::D7.as_bitset()
            | Tile::D2.as_bitset()
            | Tile::G4.as_bitset()
            | Tile::A4.as_bitset();

        let moves = board.rook_moves(board.to_move, rook_pos);

        // Should be able to capture all enemy pieces
        assert!(moves.contains(Tile::D7));
        assert!(moves.contains(Tile::D2));
        assert!(moves.contains(Tile::G4));
        assert!(moves.contains(Tile::A4));
    }

    #[test]
    fn test_rook_blocked_by_own_pieces() {
        magic::initialize();
        let mut board = Board::empty();

        board.to_move = Color::White;
        let rook_pos = Tile::D4;

        // Place own pieces around the rook
        board.occupancy[Color::White] =
            Tile::D4.as_bitset() | Tile::D6.as_bitset() | Tile::F4.as_bitset();

        let moves = board.rook_moves(board.to_move, rook_pos);

        // Should NOT be able to move to squares occupied by own pieces
        assert!(!moves.contains(Tile::D6));
        assert!(!moves.contains(Tile::F4));

        // Should NOT be able to move beyond own pieces
        assert!(!moves.contains(Tile::D7));
        assert!(!moves.contains(Tile::D8));
        assert!(!moves.contains(Tile::G4));
        assert!(!moves.contains(Tile::H4));

        // Can still move in unblocked directions
        assert!(moves.contains(Tile::D3));
        assert!(moves.contains(Tile::D2));
        assert!(moves.contains(Tile::D1));
        assert!(moves.contains(Tile::C4));
        assert!(moves.contains(Tile::B4));
        assert!(moves.contains(Tile::A4));
    }
}

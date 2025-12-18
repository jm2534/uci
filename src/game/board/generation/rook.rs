//! Rook move generation using bitboard techniques.

use super::super::{Bitset, Board};
use crate::game::board::tile::Tile;

impl Board {
    /// Generate all possible moves for a rook at the given tile.
    pub(super) fn rook_moves(&self, _tile: Tile) -> Bitset {
        todo!("Rook move generation")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{
        board::tile::Tile,
        color::Color,
        piece::{Piece, PieceKind},
    };

    #[test]
    fn test_rook_rank_moves() {
        // TODO: Test rook moves along ranks (horizontal)
    }

    #[test]
    fn test_rook_file_moves() {
        // TODO: Test rook moves along files (vertical)
    }

    #[test]
    fn test_rook_blocked_by_pieces() {
        // TODO: Test that rook cannot move through other pieces
    }

    #[test]
    fn test_rook_corner_position() {
        // TODO: Test rook moves from corner squares
    }

    #[test]
    fn test_rook_center_position() {
        // TODO: Test rook moves from center of board
    }

    #[test]
    fn test_rook_captures() {
        // TODO: Test rook capturing enemy pieces
    }

    #[test]
    fn test_rook_blocked_by_own_pieces() {
        // TODO: Test that rook cannot capture own pieces
    }
}

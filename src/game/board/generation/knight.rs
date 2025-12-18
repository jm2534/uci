//! Knight move generation using bitboard techniques.

use super::super::{Bitset, Board};
use crate::game::board::tile::Tile;

impl Board {
    /// Generate all possible moves for a knight at the given tile.
    pub(super) fn knight_moves(&self, _tile: Tile) -> Bitset {
        todo!("Knight move generation")
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
    fn test_knight_center_moves() {
        // TODO: Test knight moves from center of board (8 possible moves)
    }

    #[test]
    fn test_knight_corner_moves() {
        // TODO: Test knight moves from corner (2 possible moves)
    }

    #[test]
    fn test_knight_edge_moves() {
        // TODO: Test knight moves from edge squares
    }

    #[test]
    fn test_knight_blocked_by_own_pieces() {
        // TODO: Test that knight cannot move to squares occupied by own pieces
    }

    #[test]
    fn test_knight_captures() {
        // TODO: Test knight capturing enemy pieces
    }
}

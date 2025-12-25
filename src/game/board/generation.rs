//! Move generation for chess pieces using bitboard techniques.
//!
//! This module implements efficient move generation for all piece types,
//! following established bitboard patterns from chess programming literature.

mod bishop;
mod king;
mod knight;
mod pawn;
mod queen;
pub(crate) mod rook;

use super::{Bitset, Board};
use crate::game::{board::tile::Tile, piece::PieceKind};

impl Board {
    /// Returns a bitboard of all valid destination squares for the piece at the given tile.
    pub fn moves_from_tile(&self, tile: Tile, piece_kind: PieceKind) -> Bitset {
        match piece_kind {
            PieceKind::Pawn => self.pawn_moves(tile),
            PieceKind::Knight => self.knight_moves(tile),
            PieceKind::Bishop => self.bishop_moves(tile),
            PieceKind::Rook => self.rook_moves(tile),
            PieceKind::Queen => self.queen_moves(tile),
            PieceKind::King => self.king_moves(tile),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::moves::Move;

    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_initial_board_generation() {
        Board::initialize();
        let board = Board::new();
        let moves: HashSet<Move> = board.possible_moves().collect();

        let mut expected_moves = HashSet::new();
        let pawns = board.positions[PieceKind::Pawn] & board.occupancy[board.to_move];
        for start in pawns.tiles() {
            let single_push = Move::new(start, Tile::new(start.rank() + 1, start.file()));
            expected_moves.insert(single_push);

            let double_push = Move::new(start, Tile::new(start.rank() + 2, start.file()));
            expected_moves.insert(double_push);
        }

        let knights = board.positions[PieceKind::Knight] & board.occupancy[board.to_move];
        for start in knights.tiles() {
            let left = Move::new(start, Tile::new(start.rank() + 2, start.file() - 1));
            let right = Move::new(start, Tile::new(start.rank() + 2, start.file() + 1));
            expected_moves.insert(left);
            expected_moves.insert(right);
        }

        assert_eq!(moves, expected_moves);
    }
}

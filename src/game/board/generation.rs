//! Move generation for chess pieces using bitboard techniques.
//!
//! This module implements efficient move generation for all piece types,
//! following established bitboard patterns from chess programming literature.

mod bishop;
mod king;
mod knight;
mod pawn;
mod queen;
mod rook;
mod tables;

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

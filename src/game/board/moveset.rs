use crate::game::{
    piece::{Piece, PieceKind},
    tile::Tile,
};

use super::bitset::{Bitset, Offset};

pub(super) fn index_of(piece: Piece, tile: Tile) -> usize {
    match piece.kind {
        PieceKind::Pawn => todo!(),
        PieceKind::Rook => todo!(),
        PieceKind::Bishop => todo!(),
        PieceKind::Knight => todo!(),
        PieceKind::Queen => todo!(),
        PieceKind::King => todo!(),
    }
}

fn seed_pawn_movesets(store: &mut Vec<Bitset>) {}

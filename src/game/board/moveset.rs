use super::bitset::{Bitset, Offset};
use crate::game::{
    piece::{Piece, PieceKind},
    tile::Tile,
};

pub(super) fn extract_moves(moveset: Bitset, buf: &mut Vec) -> usize {
    while !moveset.is_empty() {
        let index = moveset.trailing_zeros() as u8;
        moveset = moveset.clear_bit(index);
        let m = Move {
            to: index,
            from: index as i8,
        };
        moves.push(m);
    }
}

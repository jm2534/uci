use crate::game::color::Color;
use enum_iterator::Sequence;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Sequence)]
pub enum PieceKind {
    Pawn,
    Rook,
    Bishop,
    Knight,
    Queen,
    King,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Piece {
    pub kind: PieceKind,
    pub color: Color,
}

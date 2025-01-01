use crate::game::color::Color;
use enum_iterator::Sequence;
use thiserror::Error;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Sequence)]
pub enum PieceKind {
    Pawn,
    Rook,
    Bishop,
    Knight,
    Queen,
    King,
}

#[derive(Error, Debug)]
pub enum ParsePieceError {
    #[error("`{0}` is not a recognized piece symbol`")]
    UnknownCharacter(char),
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Piece {
    pub kind: PieceKind,
    pub color: Color,
}

impl TryFrom<char> for Piece {
    type Error = ParsePieceError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let kind = match value.to_uppercase().next().unwrap() {
            'P' => PieceKind::Pawn,
            'R' => PieceKind::Rook,
            'B' => PieceKind::Bishop,
            'N' => PieceKind::Knight,
            'Q' => PieceKind::Queen,
            'K' => PieceKind::King,
            _ => return Err(ParsePieceError::UnknownCharacter(value)),
        };

        let color = Color::from(value.is_uppercase());
        Ok(Self { kind, color })
    }
}

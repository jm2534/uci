use crate::game::color::Color;
use enum_iterator::Sequence;
use thiserror::Error;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Sequence)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Error, Debug)]
pub enum ParsePieceError {
    #[error("`{0}` is not a recognized piece symbol`")]
    UnknownCharacter(char),
}

impl std::fmt::Display for PieceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PieceKind::Pawn => write!(f, "Pawn"),
            PieceKind::Knight => write!(f, "Knight"),
            PieceKind::Bishop => write!(f, "Bishop"),
            PieceKind::Rook => write!(f, "Rook"),
            PieceKind::Queen => write!(f, "Queen"),
            PieceKind::King => write!(f, "King"),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Piece {
    pub kind: PieceKind,
    pub color: Color,
}

impl Piece {
    pub fn fen_char(&self) -> char {
        match self.kind {
            PieceKind::Pawn => match self.color {
                Color::White => 'P',
                Color::Black => 'p',
            },
            PieceKind::Knight => match self.color {
                Color::White => 'N',
                Color::Black => 'n',
            },
            PieceKind::Bishop => match self.color {
                Color::White => 'B',
                Color::Black => 'b',
            },
            PieceKind::Rook => match self.color {
                Color::White => 'R',
                Color::Black => 'r',
            },
            PieceKind::Queen => match self.color {
                Color::White => 'Q',
                Color::Black => 'q',
            },
            PieceKind::King => match self.color {
                Color::White => 'K',
                Color::Black => 'k',
            },
        }
    }
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

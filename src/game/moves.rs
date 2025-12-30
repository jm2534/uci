use std::{
    fmt::{Display, Formatter},
    ops::{BitOr, BitOrAssign},
    sync::LazyLock,
};

use regex::Regex;
use thiserror::Error;

use crate::game::piece::PieceKind;

use super::{
    board::tile::{Tile, TileParseError},
    piece::Piece,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MoveKind {
    /// All moves which have no captures or promotions, or result in check
    Quiet,

    /// Initial pawn move two steps forward
    DoublePawnPush,

    /// King-side castle
    KingCastle,

    /// Queen-side castle
    QueenCastle,

    /// A move resulting in the capture of the contained piece
    Capture(PieceKind),

    /// A capture accomplished through enpassant
    EnPassantCapture(PieceKind),

    /// A promotion of a piece into the contained piece
    Promotion(PieceKind),

    /// A promotion of a piece into the first contained piece, capturing the second
    PromotionCapture(Piece, Piece),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum SpecialMove {
    Promotion = 1,
    EnPassant = 2,
    Castle = 3,
}

impl Display for SpecialMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialMove::Promotion => write!(f, "Promotion"),
            SpecialMove::EnPassant => write!(f, "EnPassant"),
            SpecialMove::Castle => write!(f, "Castle"),
        }
    }
}

/// Stockfish definition of a move:
/// A move needs 16 bits to be stored
///
/// bit  0..=5: destination square (from 0 to 63)
/// bit  6..=11: origin square (from 0 to 63)
/// bit 12..=13: promotion piece type - 2 (from KNIGHT-2 to QUEEN-2)
/// bit 14..=15: special move flag: promotion (1), en passant (2), castling (3)
/// NOTE: EN-PASSANT bit is set only when a pawn can be captured
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Move(u16);

impl Move {
    const NULL_MOVE: &str = "0000";

    pub fn new(start: Tile, stop: Tile) -> Self {
        let mut value = start.as_index() as u16;
        value |= (stop.as_index() as u16) << 6;
        // TODO: Implement promotion and special move flags
        Self(value)
    }

    pub fn start(&self) -> Tile {
        Tile::from_index((self.0 & 0x003F) as usize)
    }

    pub fn stop(&self) -> Tile {
        Tile::from_index((self.0 & 0x0FC0) as usize >> 6)
    }

    pub fn special(&self) -> Option<SpecialMove> {
        let value = (self.0 >> 14) as u8;

        match value {
            1 => Some(SpecialMove::Promotion),
            2 => Some(SpecialMove::EnPassant),
            3 => Some(SpecialMove::Castle),
            _ => None,
        }

        // Safety: Self is typed to have a u16, so there will only ever be 4 values
        // after shifting by 14, one being 0 and the other 3 being valid SpecialMoves
        // if value == 0 {
        //     None
        // } else {
        //     Some(unsafe { std::mem::transmute::<u8, SpecialMove>(value) })
        // }
    }
}

impl BitOr<SpecialMove> for Move {
    type Output = Self;

    fn bitor(self, rhs: SpecialMove) -> Self::Output {
        Self(self.0 | ((rhs as u16) << 14))
    }
}

impl BitOrAssign<SpecialMove> for Move {
    fn bitor_assign(&mut self, rhs: SpecialMove) {
        *self = *self | rhs;
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.start(), self.stop())
    }
}

#[derive(Error, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum MoveParseError {
    #[error("Null move passed")]
    NullMove,

    #[error("Bad move format: expected [a-g][1-8][a-g][1-8], found `{0}`")]
    BadFormat(String),

    #[error("Could not parse tile from move `{1}`: {0}")]
    BadTile(TileParseError, String),
}

impl TryFrom<&str> for Move {
    type Error = MoveParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        static REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?<start>[a-g][1-8])(?<stop>[a-g][1-8])(?<promo>q)?").unwrap()
        });

        match value.trim() {
            Move::NULL_MOVE => Err(MoveParseError::NullMove),
            trimmed => {
                let re: &Regex = &REGEX;
                let captures = re.captures(value);

                match captures {
                    None => Err(MoveParseError::BadFormat(trimmed.to_owned())),
                    Some(caps) => {
                        let start = caps["start"].try_into();
                        let stop = caps["stop"].try_into();
                        match (start, stop) {
                            (Ok(start), Ok(stop)) => Ok(Move::new(start, stop)),
                            (Err(e), _) | (_, Err(e)) => {
                                Err(MoveParseError::BadTile(e, trimmed.to_owned()))
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_start_stop() {
        let start = Tile::new(1, 0);
        let stop = Tile::new(2, 0);
        let m = Move::new(start, stop);

        assert_eq!(m.start(), start);
        assert_eq!(m.stop(), stop);
    }

    #[test]
    fn test_edge_file() -> Result<()> {
        let value = "a2a3";

        assert_eq!(
            Move::try_from(value)?,
            Move::new(Tile::new(1, 0), Tile::new(2, 0))
        );
        Ok(())
    }

    #[test]
    fn test_edge_rank() -> Result<()> {
        let value = "b1b2";

        assert_eq!(
            Move::try_from(value)?,
            Move::new(Tile::new(0, 1), Tile::new(1, 1))
        );
        Ok(())
    }

    #[test]
    fn test_same() -> Result<()> {
        let value = "a1a1";

        assert_eq!(
            Move::try_from(value)?,
            Move::new(Tile::new(0, 0), Tile::new(0, 0))
        );
        Ok(())
    }

    #[test]
    fn test_legal() -> Result<()> {
        let value = "e2e4";

        assert_eq!(
            Move::try_from(value)?,
            Move::new(Tile::new(1, 4), Tile::new(3, 4))
        );
        Ok(())
    }

    #[test]
    fn test_castle() -> Result<()> {
        let value = "e1g1";

        assert_eq!(
            Move::try_from(value)?,
            Move::new(Tile::new(0, 4), Tile::new(0, 6))
        );
        Ok(())
    }

    #[test]
    fn test_null() -> Result<()> {
        let value = Move::NULL_MOVE;

        assert_eq!(Move::try_from(value), Err(MoveParseError::NullMove),);
        Ok(())
    }

    #[test]
    fn test_out_of_bounds_rank() {
        let value = "a0a0";

        assert_eq!(
            Move::try_from(value),
            Err(MoveParseError::BadFormat(value.to_owned()))
        );
    }

    #[test]
    fn test_empty_special() {
        let value = Move::try_from("e2e4").unwrap();
        assert!(value.special().is_none());
    }

    #[test]
    fn test_special_promotion() {
        let mut value = Move::try_from("e7e8q").unwrap();
        value.0 |= 0b0100_0000_0000_0000;

        assert_eq!(value.special(), Some(SpecialMove::Promotion));
        assert_eq!(value.start(), Tile::E7);
        assert_eq!(value.stop(), Tile::E8);
    }

    #[test]
    fn test_special_enpassant() {
        let mut value = Move::try_from("e1g1").unwrap();
        value.0 |= 0b1000_0000_0000_0000;

        assert_eq!(value.special(), Some(SpecialMove::EnPassant));
        assert_eq!(value.start(), Tile::E1);
        assert_eq!(value.stop(), Tile::G1);
    }

    #[test]
    fn test_special_castling() {
        let mut value = Move::try_from("e1g1").unwrap();
        value.0 |= 0b1100_0000_0000_0000;

        assert_eq!(value.special(), Some(SpecialMove::Castle));
        assert_eq!(value.start(), Tile::E1);
        assert_eq!(value.stop(), Tile::G1);
    }

    #[test]
    fn test_move_or_promotion() {
        let mut value = Move::try_from("e7e8q").unwrap();
        value |= SpecialMove::Promotion;

        assert_eq!(value.0 >> 14, 1);
        assert_eq!(value.special(), Some(SpecialMove::Promotion));
        assert_eq!(value.start(), Tile::E7);
        assert_eq!(value.stop(), Tile::E8);
    }

    #[test]
    fn test_move_or_enpassant() {
        let mut value = Move::try_from("e1g1").unwrap();
        value |= SpecialMove::EnPassant;

        assert_eq!(value.0 >> 14, 2);
        assert_eq!(value.special(), Some(SpecialMove::EnPassant));
        assert_eq!(value.start(), Tile::E1);
        assert_eq!(value.stop(), Tile::G1);
    }

    #[test]
    fn test_move_or_castle() {
        let mut value = Move::try_from("e1g1").unwrap();
        value |= SpecialMove::Castle;

        assert_eq!(value.0 >> 14, 3);
        assert_eq!(value.special(), Some(SpecialMove::Castle));
        assert_eq!(value.start(), Tile::E1);
        assert_eq!(value.stop(), Tile::G1);
    }
}

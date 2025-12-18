use std::{fmt::Display, sync::LazyLock};

use regex::Regex;
use thiserror::Error;

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
    Capture(Piece),

    /// A capture accomplished through enpassant
    EnPassantCapture(Piece),

    /// A promotion of a piece into the contained piece
    Promotion(Piece),

    /// A promotion of a piece into the first contained piece, capturing the second
    PromotionCapture(Piece, Piece),
}

/// Stockfish definition of a move:
/// A move needs 16 bits to be stored
///
/// bit  0-5: destination square (from 0 to 63)
/// bit  6-11: origin square (from 0 to 63)
/// bit 12-13: promotion piece type - 2 (from KNIGHT-2 to QUEEN-2)
/// bit 14-15: special move flag: promotion (1), en passant (2), castling (3)
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
                let re: &Regex = &*REGEX;
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
}

use std::sync::LazyLock;

use regex::Regex;
use thiserror::Error;

use super::tile::{Tile, TileParseError};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Move {
    pub start: Tile,
    pub stop: Tile,
}

impl Move {
    const NULL_MOVE: &str = "0000";
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
                            (Ok(start), Ok(stop)) => Ok(Move { start, stop }),
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
    fn test_edge_file() -> Result<()> {
        let value = "a2a3";

        assert_eq!(
            Move::try_from(value)?,
            Move {
                start: Tile { file: 0, rank: 1 },
                stop: Tile { file: 0, rank: 2 }
            }
        );
        Ok(())
    }

    #[test]
    fn test_edge_rank() -> Result<()> {
        let value = "b1b2";

        assert_eq!(
            Move::try_from(value)?,
            Move {
                start: Tile { file: 1, rank: 0 },
                stop: Tile { file: 1, rank: 1 }
            }
        );
        Ok(())
    }

    #[test]
    fn test_same() -> Result<()> {
        let value = "a1a1";

        assert_eq!(
            Move::try_from(value)?,
            Move {
                start: Tile { file: 0, rank: 0 },
                stop: Tile { file: 0, rank: 0 }
            }
        );
        Ok(())
    }

    #[test]
    fn test_legal() -> Result<()> {
        let value = "e2e4";

        assert_eq!(
            Move::try_from(value)?,
            Move {
                start: Tile { file: 4, rank: 1 },
                stop: Tile { file: 4, rank: 3 }
            }
        );
        Ok(())
    }

    #[test]
    fn test_castle() -> Result<()> {
        let value = "e1g1";

        assert_eq!(
            Move::try_from(value)?,
            Move {
                start: Tile { file: 4, rank: 0 },
                stop: Tile { file: 6, rank: 0 }
            }
        );
        Ok(())
    }

    #[test]
    fn test_promotion() -> Result<()> {
        let value = "e7e8q";
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

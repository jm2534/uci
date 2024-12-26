use std::fmt::Display;
use thiserror::Error;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Tile {
    pub rank: usize,
    pub file: usize,
}

impl Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            char::from_u32((self.file as u32) + ('a' as u32)).unwrap(),
            self.rank + 1 // zero -> one indexing
        )
    }
}

impl Tile {
    fn parse_rank(ch: char) -> Option<usize> {
        match ch {
            '1'..='8' => ch.to_digit(10).and_then(|x| (x as usize).checked_sub(1)),
            _ => None,
        }
    }

    fn parse_file(ch: char) -> Option<usize> {
        match ch {
            'a'..='g' => (ch as usize).checked_sub('a' as usize),
            _ => None,
        }
    }
}

impl TryFrom<&str> for Tile {
    type Error = TileParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut chars = value.trim_start().chars();
        let file = Tile::parse_file(chars.next().unwrap());
        let rank = Tile::parse_rank(chars.next().unwrap());

        match (rank, file) {
            (Some(rank), Some(file)) => Ok(Tile { rank, file }),
            _ => Err(TileParseError::BadFormat(value.to_owned())),
        }
    }
}

#[derive(Error, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum TileParseError {
    #[error("Format must be `[a-g][1-8]`, both inclusive; found `{0}`")]
    BadFormat(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_tile_display() -> Result<()> {
        let value = "a2";

        let disp = format!("{}", Tile::try_from(value)?);
        assert_eq!(disp, value);
        Ok(())
    }
}

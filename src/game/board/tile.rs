use crate::game::board::{Board, bitset::Bitset};
use paste::paste;
use std::{
    fmt::Display,
    ops::{BitAnd, BitOr, BitXor, Index, IndexMut, Not},
};
use thiserror::Error;

// from https://stackoverflow.com/questions/71443259/possible-rust-macro-to-define-a-bunch-of-constants
macro_rules! board {
    // For each column, call column!() passing the details of that column
    // and all of the rows. (This can't be done in one macro because macro
    // repetition works like "zip", not like "cartesian product".)
    ( ($($cols:ident $colnos:literal),*), $rows:tt ) => {
        $( column!($cols, $colnos, $rows); )*
    };
}

macro_rules! column {
    ( $col:ident, $colno:literal, ($($rows:literal),*) ) => {
        $(
            paste! {
                // [< >] are special brackets that tell the `paste!` macro to
                // paste together all the pieces appearing within them into
                // a single identifier.
                #[allow(dead_code)]
                pub const [< $col $rows >]: Tile = Tile::from_index($colno + (($rows - 1) * 8));
            }
        )*
    };
}

/// Represents a tile on a chessboard. Implements common set-wise operations for
/// bitset interactions.
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Tile(Bitset);

impl Tile {
    board!((A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7), (1, 2, 3, 4, 5, 6, 7, 8));

    /// Create a new tile from (zero-indexed) rank and file.
    /// Saturates at the maximum rank and file values, meaning that if the rank
    /// or file is greater than or equal to the maximum dimension of the board,
    /// it will be set to the maximum dimension minus one.
    ///
    /// # Examples
    ///
    /// ```
    /// use uci::game::board::tile::Tile;
    ///
    /// let tile = Tile::new(0, 0);
    /// assert_eq!(tile.as_index(), 0); // a1
    ///
    /// let tile = Tile::new(255, 0);
    /// assert_eq!(tile.as_index(), 56); // a8
    ///
    /// let tile = Tile::new(0, 255);
    /// assert_eq!(tile.as_index(), 7); // h1
    ///
    /// let tile = Tile::new(255, 255);
    /// assert_eq!(tile.as_index(), 63); // h8
    /// ```
    pub fn new(rank: u8, file: u8) -> Self {
        let rank_shift = Board::MAX_DIM as u64 * rank.min(Board::MAX_DIM - 1) as u64;
        let file_shift = file.min(Board::MAX_DIM - 1) as u64;
        let value = 1 << (rank_shift + file_shift);
        Self(Bitset(value))
    }

    /// Creates a tile from an index. Panics if the index is not the range `[0, 63]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use uci::game::board::tile::Tile;
    ///
    /// let tile = Tile::from_index(0);
    /// assert_eq!(tile.as_index(), 0); // a1
    ///
    /// let tile = Tile::from_index(56);
    /// assert_eq!(tile.as_index(), 56); // a8
    ///
    /// let tile = Tile::from_index(7);
    /// assert_eq!(tile.as_index(), 7); // h1
    ///
    /// let tile = Tile::from_index(63);
    /// assert_eq!(tile.as_index(), 63); // h8
    /// ```
    pub const fn from_index(index: usize) -> Self {
        Self(Bitset(1 << index))
    }

    /// Returns the representation of the tile as an index, meaning the
    /// zero-indexed position of the tile in the flattened board (thus in
    /// the range `[0, 63]`).
    pub const fn as_index(&self) -> usize {
        self.0.0.trailing_zeros() as usize
    }

    /// Returns the representation of the tile as a bitset.
    pub const fn as_bitset(&self) -> Bitset {
        self.0
    }

    /// Returns the rank of the tile, meaning the zero-indexed row of the
    /// tile in the board (thus in the range `[0, 7]`).
    pub const fn rank(&self) -> u8 {
        (self.as_index() >> 3) as u8
    }

    /// Returns the file of the tile, meaning the zero-indexed column of the
    /// tile in the board (thus in the range `[0, 7]`).
    pub const fn file(&self) -> u8 {
        (self.as_index() & 7) as u8
    }

    fn parse_rank(ch: char) -> Option<u8> {
        match ch {
            '1'..='8' => ch.to_digit(10).map(|x| (x as u8) - 1),
            _ => None,
        }
    }

    fn parse_file(ch: char) -> Option<u8> {
        match ch {
            'a'..='h' => Some((ch as u8) - b'a'),
            _ => None,
        }
    }
}

impl Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            char::from_u32((self.file() as u32) + ('a' as u32)).unwrap(),
            self.rank() + 1 // zero -> one indexing
        )
    }
}

impl Not for Tile {
    type Output = Bitset;

    fn not(self) -> Self::Output {
        !self.0
    }
}

impl<T> Index<Tile> for Vec<T> {
    type Output = T;

    fn index(&self, index: Tile) -> &Self::Output {
        &self[index.as_index()]
    }
}

impl<T> Index<Tile> for [T] {
    type Output = T;

    fn index(&self, index: Tile) -> &Self::Output {
        &self[index.as_index()]
    }
}

impl<T> IndexMut<Tile> for Vec<T> {
    fn index_mut(&mut self, index: Tile) -> &mut Self::Output {
        &mut self[index.as_index()]
    }
}

impl<T> IndexMut<Tile> for [T] {
    fn index_mut(&mut self, index: Tile) -> &mut Self::Output {
        &mut self[index.as_index()]
    }
}

/// Tile iterator over a bitset
#[derive(Copy, Clone, Debug)]
pub struct Tiles {
    value: i64,
}

impl Tiles {
    pub fn new(value: Bitset) -> Self {
        Self {
            value: value.0 as i64,
        }
    }
}

impl Iterator for Tiles {
    type Item = Tile;

    fn next(&mut self) -> Option<Self::Item> {
        match self.value {
            0 => None,
            x => {
                // TODO: works, but need to understand why wrapping specifically
                let ls1b = x & x.wrapping_neg(); // isolate LS1B
                self.value ^= ls1b; // clear LS1B
                Some(Tile(Bitset(ls1b as u64)))
            }
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TileConversionError {
    #[error("Multiple tiles found in set {0:?}")]
    MultipleTiles(Bitset),

    #[error("Empty bitset provided")]
    Empty,
}

impl From<Tile> for u64 {
    fn from(tile: Tile) -> Self {
        tile.0.into()
    }
}

impl TryFrom<u64> for Tile {
    type Error = TileConversionError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let value = Bitset(value);
        if value.len() > 1 {
            Err(TileConversionError::MultipleTiles(value))
        } else if value.is_empty() {
            Err(TileConversionError::Empty)
        } else {
            Ok(Self(value))
        }
    }
}

impl TryFrom<Bitset> for Tile {
    type Error = TileConversionError;

    fn try_from(set: Bitset) -> Result<Self, Self::Error> {
        Self::try_from(set.0)
    }
}

#[derive(Error, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum TileParseError {
    #[error("Format must be `[a-g][1-8]`, both inclusive; found `{0}`")]
    BadFormat(String),

    #[error("Unexpected end of string")]
    UnexpectedEnd,
}

impl TryFrom<&str> for Tile {
    type Error = TileParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut chars = value.trim_start().chars();
        let file = Tile::parse_file(chars.next().ok_or(TileParseError::UnexpectedEnd)?);
        let rank = Tile::parse_rank(chars.next().ok_or(TileParseError::UnexpectedEnd)?);
        match (rank, file) {
            (Some(rank), Some(file)) => Ok(Tile::new(rank, file)),
            _ => Err(TileParseError::BadFormat(value.to_owned())),
        }
    }
}

impl BitAnd for Tile {
    type Output = Bitset;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.0 & rhs.0
    }
}

impl BitOr for Tile {
    type Output = Bitset;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.0 | rhs.0
    }
}

impl BitXor for Tile {
    type Output = Bitset;

    fn bitxor(self, rhs: Self) -> Self::Output {
        self.0 ^ rhs.0
    }
}

impl BitAnd<Bitset> for Tile {
    type Output = Bitset;

    fn bitand(self, rhs: Bitset) -> Self::Output {
        self.0 & rhs
    }
}

impl BitOr<Bitset> for Tile {
    type Output = Bitset;
    fn bitor(self, rhs: Bitset) -> Bitset {
        self.0 | rhs
    }
}

impl BitXor<Bitset> for Tile {
    type Output = Bitset;

    fn bitxor(self, rhs: Bitset) -> Self::Output {
        self.0 ^ rhs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_tile_macro() {
        assert_eq!(Tile::A1, Tile::from_index(0));
        assert_eq!(Tile::A2, Tile::from_index(8));
        assert_eq!(Tile::H8, Tile::from_index(63));
    }

    #[test]
    fn test_tile_display() -> Result<()> {
        let value = "a2";

        let disp = format!("{}", Tile::try_from(value)?);
        assert_eq!(disp, value);
        Ok(())
    }

    mod parsing {
        use super::*;
        use anyhow::Result;

        #[test]
        fn test_parse_tile() -> Result<()> {
            let value = "a2";

            let tile = Tile::try_from(value)?;
            assert_eq!(tile, Tile::A2);
            Ok(())
        }

        #[test]
        fn test_parse_rank() {
            let ranks = ['1', '2', '3', '4', '5', '6', '7', '8'];
            for (i, rank) in ranks.iter().enumerate() {
                assert_eq!(Tile::parse_rank(*rank), Some(i as u8));
            }
        }

        #[test]
        fn test_parse_file() {
            let files = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
            for (i, file) in files.iter().enumerate() {
                assert_eq!(Tile::parse_file(*file), Some(i as u8));
            }
        }
    }

    mod conversation {
        use crate::game::board::{bitset::Bitset, tile::TileConversionError};

        use super::Tile;

        #[test]
        fn test_tile_to_index() {
            let tile = Tile::new(0, 0);
            assert_eq!(tile.as_index(), 0);

            let tile = Tile::new(2, 3);
            assert_eq!(tile.as_index(), 19);

            let tile = Tile::new(7, 7);
            assert_eq!(tile.as_index(), 63);
        }

        #[test]
        fn test_tile_to_index_saturation() {
            let tile = Tile::new(255, 0);
            assert_eq!(tile.as_index(), 56); // a8

            let tile = Tile::new(0, 255);
            assert_eq!(tile.as_index(), 7); // g1

            let tile = Tile::new(255, 255);
            assert_eq!(tile.as_index(), 63);
        }

        #[test]
        fn test_tile_from_index() {
            assert_eq!(Tile::try_from(1).unwrap(), Tile::new(0, 0)); // a1
            assert_eq!(Tile::try_from(2).unwrap(), Tile::new(0, 1)); // b1
            assert_eq!(Tile::try_from(1 << 8).unwrap(), Tile::new(1, 0)); // a2
            assert_eq!(Tile::try_from(1 << 63).unwrap(), Tile::new(7, 7)); // h8
        }

        #[test]
        fn test_tile_empty_source() {
            assert_eq!(Tile::try_from(0), Err(TileConversionError::Empty));
            assert_eq!(Tile::try_from(Bitset(0)), Err(TileConversionError::Empty));
        }
    }

    #[test]
    fn test_bitset_iterator() {
        let mut iter = Bitset(0b101).tiles();
        assert_eq!(iter.next(), Some(Tile(Bitset(1))));
        assert_eq!(iter.next(), Some(Tile(Bitset(4))));
        assert_eq!(iter.next(), None);
    }
}

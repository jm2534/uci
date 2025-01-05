use enum_iterator::Sequence;

use crate::game::tile::Tile;
use std::{
    fmt::{Binary, Error, Formatter},
    ops::{
        BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref, Index, Not, Shl,
        ShlAssign, Shr, ShrAssign,
    },
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Sequence)]
pub(super) enum Offset {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
    None,
}

impl Offset {
    pub fn apply<T>(&self, bits: T) -> T
    where
        T: Shl<usize, Output = T> + Shr<usize, Output = T>,
    {
        match self {
            Offset::North => bits << 8,
            Offset::South => bits >> 8,
            Offset::East => bits << 1,
            Offset::West => bits >> 1,
            Offset::NorthEast => bits << 9,
            Offset::NorthWest => bits << 7,
            Offset::SouthEast => bits >> 7,
            Offset::SouthWest => bits >> 9,
            Offset::None => bits,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Bitset(pub u64);

pub struct BitsetTiles {
    value: i64,
}

impl Iterator for BitsetTiles {
    type Item = Tile;

    fn next(&mut self) -> Option<Self::Item> {
        match self.value {
            0 => None,
            x => {
                let ls1b = x & -x; // isolate LS1B
                self.value ^= ls1b; // clear LS1B
                Some(Tile::from_index(ls1b as usize))
            }
        }
    }
}

impl Bitset {
    /// The maximum cardinality of a Bitset
    pub const MAX_LEN: usize = 64;

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Determines the cardinality of `self`; that is, the number of set
    /// bits in `self`.
    pub fn len(&self) -> u64 {
        // Kernighan's way: count how many times we have to reset the least
        // significant 1-bit.
        let mut count = 0;
        let mut x = self.0;
        while x != 0 {
            count += 1;
            x &= x - 1; // reset LS1B
        }
        count
    }

    pub fn tiles(&self) -> BitsetTiles {
        BitsetTiles {
            value: self.0 as i64,
        }
    }

    pub fn contains(&self, tile: Tile) -> bool {
        !(*self & tile).is_empty()
    }

    pub fn insert(&mut self, tile: Tile) {
        self.0 |= u64::from(tile)
    }

    pub fn toggle(&mut self, tile: Tile) {
        self.0 ^= u64::from(tile)
    }

    pub fn offset(&self, offset: Offset) -> Self {
        Self(offset.apply(self.0))
    }

    /// Modifies `self` in-place with the set union from
    /// `self`` and `other`.
    pub fn union_with(&mut self, other: Self) {
        self.0 |= other.0
    }

    /// Modifies `self` in-place with the set intersection from
    /// `self`` and `other`.
    pub fn intersect_with(&mut self, other: Self) {
        self.0 &= other.0
    }

    /// Creates a bitset with the least significant 1 bit of `other` as the
    /// only bit set.
    pub fn ls1b(other: Bitset) -> Bitset {
        Bitset(other.0 & !other.0)
    }

    /// Creates a bitset that is the set union of an iterable of
    /// existing bitsets.
    pub fn union<'a, T>(bitsets: T) -> Bitset
    where
        T: Iterator<Item = Bitset>,
    {
        bitsets.fold(Bitset(0), |val, set| val | set)
    }

    /// Creates a bitset that is the set intersection of an iterable of
    /// existing bitsets.
    pub fn intersect<'a, T>(bitsets: T) -> Bitset
    where
        T: Iterator<Item = &'a Bitset>,
    {
        bitsets.fold(Bitset(0), |val, &set| val & set)
    }
}

impl From<Bitset> for bool {
    fn from(value: Bitset) -> Self {
        value.0 > 0
    }
}

impl From<Bitset> for u64 {
    fn from(value: Bitset) -> Self {
        value.0
    }
}

impl From<u64> for Bitset {
    fn from(value: u64) -> Self {
        Bitset(value)
    }
}

impl From<Tile> for Bitset {
    fn from(value: Tile) -> Self {
        Bitset(value.into())
    }
}

impl<T: Into<u64>> BitAnd<T> for Bitset {
    type Output = Bitset;

    fn bitand(self, rhs: T) -> Self::Output {
        Bitset(self.0 & rhs.into())
    }
}

impl<T: Into<u64>> BitOr<T> for Bitset {
    type Output = Bitset;

    fn bitor(self, rhs: T) -> Self::Output {
        Bitset(self.0 | rhs.into())
    }
}

impl<T: Into<u64>> BitXor<T> for Bitset {
    type Output = Bitset;

    fn bitxor(self, rhs: T) -> Self::Output {
        Bitset(self.0 ^ rhs.into())
    }
}

impl<T: Into<u64>> BitAndAssign<T> for Bitset {
    fn bitand_assign(&mut self, rhs: T) {
        self.0 &= rhs.into();
    }
}

impl<T: Into<u64>> BitOrAssign<T> for Bitset {
    fn bitor_assign(&mut self, rhs: T) {
        self.0 |= rhs.into();
    }
}

impl<T: Into<u64>> BitXorAssign<T> for Bitset {
    fn bitxor_assign(&mut self, rhs: T) {
        self.0 ^= rhs.into();
    }
}

impl<T: Into<u64>> Shl<T> for Bitset {
    type Output = Self;

    fn shl(self, rhs: T) -> Self::Output {
        Bitset(self.0 << rhs.into())
    }
}

impl ShlAssign<usize> for Bitset {
    fn shl_assign(&mut self, rhs: usize) {
        *self = Self(self.0 << rhs)
    }
}

impl<T: Into<u64>> Shr<T> for Bitset {
    type Output = Self;

    fn shr(self, rhs: T) -> Self::Output {
        Bitset(self.0 >> rhs.into())
    }
}

impl ShrAssign<usize> for Bitset {
    fn shr_assign(&mut self, rhs: usize) {
        *self = Self(self.0 >> rhs)
    }
}

impl Not for Bitset {
    type Output = Self;

    fn not(self) -> Self::Output {
        Bitset(!self.0)
    }
}

impl Binary for Bitset {
    // Required method
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{:b}", self.0)
    }
}

#[cfg(test)]
mod test_bitset {
    use super::*;

    #[test]
    fn test_is_empty() {
        assert!(Bitset(0).is_empty());
        assert!(!Bitset(1).is_empty());
    }

    #[test]
    fn test_is_not_empty() {
        assert!(!Bitset(1).is_empty());
    }

    #[test]
    fn test_count() {
        assert_eq!(Bitset(0).len(), 0);
        assert_eq!(Bitset(1).len(), 1);
        assert_eq!(Bitset(255).len(), 8);
    }

    #[test]
    fn test_contains() {
        assert!(!Bitset(0).contains(Tile { rank: 0, file: 0 }));
        assert!(Bitset(1).contains(Tile { rank: 0, file: 0 }));
        assert!(Bitset(8).contains(Tile { rank: 0, file: 3 }));
    }

    #[test]
    fn test_bitset_iterator() {
        let mut iter = Bitset(0b101).tiles();
        assert_eq!(iter.next(), Some(Tile::from_index(1)));
    }
}

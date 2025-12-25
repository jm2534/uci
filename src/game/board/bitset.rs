use enum_iterator::Sequence;
use std::{
    fmt::{Binary, Error, Formatter},
    ops::{
        BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
        ShrAssign,
    },
};

use crate::game::board::tile::{Tile, Tiles};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Sequence)]
pub enum Offset {
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

#[derive(Clone, Copy, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Bitset(pub u64);

impl Bitset {
    /// The maximum cardinality of a Bitset
    pub const MAX_LEN: usize = 64;

    /// Creates an empty Bitset
    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Determines the cardinality of `self`; that is, the number of set
    /// bits in `self`.
    pub fn len(&self) -> u32 {
        self.0.count_ones()
    }

    /// Returns an iterator over the set bits in `self` as `Tile`s.
    ///
    /// ### Example
    /// ```
    /// use uci::game::board::{bitset::Bitset, tile::Tile};
    ///
    /// let bitset = Tile::A1 | Tile::B2 | Tile::C3;
    /// let tiles: Vec<Tile> = bitset.tiles().collect();
    /// assert_eq!(tiles, vec![Tile::A1, Tile::B2, Tile::C3]);
    /// ```
    pub fn tiles(&self) -> Tiles {
        Tiles::new(*self)
    }

    /// Determines whether `self` contains all the bits from `other`.
    pub fn contains(&self, other: impl Into<Self>) -> bool {
        !(*self & other.into()).is_empty()
    }

    /// Inserts the bits from `other` into `self`.
    pub fn insert(&mut self, other: impl Into<Self>) {
        self.0 |= other.into().0
    }

    /// Toggles the bits from `other` on `self`.
    pub fn toggle(&mut self, other: impl Into<Self>) {
        self.0 ^= other.into().0
    }

    /// Applies the given offset to each set bit in `self`.
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

    /// Returns the position of the least-significant set bit
    pub const fn lsb1_as_index(&self) -> usize {
        self.0.trailing_zeros() as usize
    }

    /// Indexes into the bitset's occupancy patterns to return a subset of the set bits.
    /// Put another way, we can image a `Bitset` as having `2^self.len()`
    /// possible occupancy patterns; this function returns the occupancy pattern corresponding
    /// to the given index.
    ///
    /// See also PEXT, which is a hardware instruction on newer processor that can perform
    /// this operation (where the bitset itself is the value and `index` is the mask)
    /// in parallel.
    ///
    /// ### Example:
    ///
    /// Let's say we have a `Bitset(0b0010110)`, meaning bits at positions 1, 2, 4 are set.
    /// We have occupancies defined for `index` in `0..8`:
    /// ```text
    /// index = 0 (binary: 000) → result = 0b0000000  (no bits set)
    /// index = 1 (binary: 001) → result = 0b0000010  (bit 1 set)
    /// index = 2 (binary: 010) → result = 0b0000100  (bit 2 set)
    /// index = 3 (binary: 011) → result = 0b0000110  (bits 1,2 set)
    /// index = 4 (binary: 100) → result = 0b0010000  (bit 4 set)
    /// index = 5 (binary: 101) → result = 0b0010010  (bits 1,4 set)
    /// index = 6 (binary: 110) → result = 0b0010100  (bits 2,4 set)
    /// index = 7 (binary: 111) → result = 0b0010110  (bits 1,2,4 set)
    /// ```
    pub fn occupancy(self, index: u32) -> Bitset {
        let mut occupancy = Bitset::empty();
        let mut i = index;
        for tile in self.tiles() {
            if i & 1 != 0 {
                occupancy.insert(tile);
            }
            i >>= 1;
        }
        occupancy
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

impl From<Tile> for Bitset {
    fn from(tile: Tile) -> Self {
        tile.as_bitset()
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

impl BitAnd<Bitset> for u64 {
    type Output = Bitset;

    fn bitand(self, rhs: Bitset) -> Self::Output {
        Bitset(self & rhs.0)
    }
}

impl BitOr<Bitset> for u64 {
    type Output = Bitset;

    fn bitor(self, rhs: Bitset) -> Self::Output {
        Bitset(self | rhs.0)
    }
}

impl BitXor<Bitset> for u64 {
    type Output = Bitset;

    fn bitxor(self, rhs: Bitset) -> Self::Output {
        Bitset(self ^ rhs.0)
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

impl<T: Into<u64>> ShlAssign<T> for Bitset {
    fn shl_assign(&mut self, rhs: T) {
        *self = Self(self.0 << rhs.into());
    }
}

impl<T: Into<u64>> Shr<T> for Bitset {
    type Output = Self;

    fn shr(self, rhs: T) -> Self::Output {
        Bitset(self.0 >> rhs.into())
    }
}

impl<T: Into<u64>> ShrAssign<T> for Bitset {
    fn shr_assign(&mut self, rhs: T) {
        *self = Self(self.0 >> rhs.into());
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
        assert!(!Bitset(0).contains(Bitset(1)));
        assert!(Bitset(1).contains(Bitset(1)));
        assert!(Bitset(8).contains(Bitset(8)));
    }

    #[test]
    fn test_occupancy() {
        let mask = Bitset(0b0010110);

        assert_eq!(mask.occupancy(0), Bitset(0b0000000));
        assert_eq!(mask.occupancy(1), Bitset(0b0000010));
        assert_eq!(mask.occupancy(2), Bitset(0b0000100));
        assert_eq!(mask.occupancy(3), Bitset(0b0000110));
        assert_eq!(mask.occupancy(4), Bitset(0b0010000));
        assert_eq!(mask.occupancy(5), Bitset(0b0010010));
        assert_eq!(mask.occupancy(6), Bitset(0b0010100));
        assert_eq!(mask.occupancy(7), Bitset(0b0010110));
    }
}

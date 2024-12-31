use enum_iterator::Sequence;

use crate::game::tile::Tile;
use std::ops::{BitAnd, BitOr, BitOrAssign, Shl, ShlAssign, Shr, ShrAssign};

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
pub(super) struct Bitset(pub u64);

impl Bitset {
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Determines the cardinality of `self`; that is, the number of set
    /// bits in `self`.
    pub fn count(&self) -> u64 {
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

    pub fn contains(&self, tile: Tile) -> bool {
        !(*self & tile).is_empty()
    }

    pub fn insert(&mut self, tile: Tile) {
        self.0 |= u64::from(tile)
    }

    pub fn toggle(&mut self, tile: Tile) {
        self.0 ^= u64::from(tile)
    }

    pub fn offset(&mut self, offset: Offset) {
        self.0 = offset.apply(self.0)
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
        T: Iterator<Item = &'a Bitset>,
    {
        bitsets.fold(Bitset(0), |val, &set| val | set)
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

// impl<T> BitOr<&T> for Bitset
// where
//     Bitset: BitOr<T>,
//     T: Copy,
//     // <Bitset as BitOr<T>>::Output: Into<Bitset>,
// {
//     type Output = Bitset;

//     fn bitor(self, rhs: &T) -> Self::Output {
//         // (self | *rhs).into()
//         self
//     }
// }

impl ShlAssign<usize> for Bitset {
    fn shl_assign(&mut self, rhs: usize) {
        *self = Self(self.0 << rhs)
    }
}

impl ShrAssign<usize> for Bitset {
    fn shr_assign(&mut self, rhs: usize) {
        *self = Self(self.0 >> rhs)
    }
}

// impl BitOrAssign<&Bitset> for Bitset {
//     fn bitor_assign(&mut self, rhs: &Bitset) {
//         *self = Self(self.0 | rhs.0)
//     }
// }

impl BitOrAssign<Bitset> for Bitset {
    fn bitor_assign(&mut self, rhs: Bitset) {
        *self = Self(self.0 | rhs.0)
    }
}

// impl BitOr<Tile> for Bitset {
//     type Output = Bitset;

//     fn bitor(self, rhs: Tile) -> Self::Output {
//         Bitset(self.0 | u64::from(rhs))
//     }
// }

// impl BitAnd<&Bitset> for Bitset {
//     type Output = Bitset;

//     fn bitand(self, rhs: &Bitset) -> Self::Output {
//         Bitset(self.0 & rhs.0)
//     }
// }

// impl BitAnd<&Tile> for Bitset {
//     type Output = Bitset;

//     fn bitand(self, rhs: &Tile) -> Self::Output {
//         Bitset(self.0 & u64::from(rhs))
//     }
// }

// impl BitAnd<&Tile> for &Bitset {
//     type Output = Bitset;

//     fn bitand(self, rhs: &Tile) -> Self::Output {
//         Bitset(self.0 & u64::from(rhs))
//     }
// }

// impl BitAnd<Tile> for Bitset {
//     type Output = Bitset;

//     fn bitand(self, rhs: Tile) -> Self::Output {
//         Bitset(self.0 & u64::from(rhs))
//     }
// }

// impl<'a, T> From<T> for Bitset
// where
//     T: Iterator<Item = Tile>,
// {
//     fn from(tiles: T) -> Self {
//         let and = |bits: u64, tile: Tile| bits | u64::from(tile);
//         Bitset(tiles.fold(0, and))
//     }
// }

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
        assert_eq!(Bitset(0).count(), 0);
        assert_eq!(Bitset(1).count(), 1);
        assert_eq!(Bitset(255).count(), 8);
    }

    #[test]
    fn test_contains() {
        assert!(!Bitset(0).contains(Tile { rank: 0, file: 0 }));
        assert!(Bitset(1).contains(Tile { rank: 0, file: 0 }));
        assert!(Bitset(8).contains(Tile { rank: 0, file: 3 }));
    }
}

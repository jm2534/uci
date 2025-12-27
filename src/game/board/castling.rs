use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use crate::game::Color;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Right {
    BlackQueenSide = 0b0001,
    BlackKingSide = 0b0010,
    WhiteQueenSide = 0b0100,
    WhiteKingSide = 0b1000,
}

impl From<Right> for char {
    fn from(right: Right) -> Self {
        match right {
            Right::BlackQueenSide => 'q',
            Right::BlackKingSide => 'k',
            Right::WhiteQueenSide => 'Q',
            Right::WhiteKingSide => 'K',
        }
    }
}

impl BitAnd for Right {
    type Output = CastlingRights;

    fn bitand(self, other: Self) -> Self::Output {
        CastlingRights(self as u8 & other as u8)
    }
}

impl BitOr for Right {
    type Output = CastlingRights;

    fn bitor(self, other: Self) -> Self::Output {
        CastlingRights(self as u8 | other as u8)
    }
}

/// Caslting rights,
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct CastlingRights(
    // MSB -> LSB: white king, white queen, black king, black queen
    u8,
);

impl CastlingRights {
    pub const NONE: CastlingRights = CastlingRights(0);
    pub const ALL: CastlingRights = CastlingRights(0b1111);
    pub const WHITE: CastlingRights = CastlingRights(0b1100);
    pub const BLACK: CastlingRights = CastlingRights(0b0011);

    /// Whether no castling rights are present.
    pub fn none(&self) -> bool {
        !self.any()
    }

    /// Whether any castling rights are present.
    pub fn any(&self) -> bool {
        self.0 != 0
    }

    /// Whether all castling rights are present.
    pub fn all(&self) -> bool {
        self == &CastlingRights::ALL
    }

    /// Whether the indicated castling right is present.
    pub fn has(&self, right: Right) -> bool {
        self.0 & (right as u8) != 0
    }

    pub fn unset_color(&mut self, color: Color) {
        match color {
            Color::White => *self &= !Self::WHITE,
            Color::Black => *self &= !Self::BLACK,
        }
    }

    /// Removes the indicated castling right.
    pub fn unset(&mut self, right: Right) {
        self.0 &= !(right as u8);
    }

    /// Adds the indicated castling right.
    pub fn set(&mut self, right: Right) {
        self.0 |= right as u8;
    }

    /// Toggles the indicated castling right.
    pub fn toggle(&mut self, right: Right) {
        self.0 ^= right as u8;
    }
}

impl From<Right> for CastlingRights {
    fn from(castling_rights: Right) -> Self {
        CastlingRights(castling_rights as u8)
    }
}

impl Not for CastlingRights {
    type Output = Self;

    fn not(self) -> Self::Output {
        CastlingRights(!self.0)
    }
}

impl BitAnd<Color> for CastlingRights {
    type Output = CastlingRights;

    fn bitand(self, color: Color) -> Self::Output {
        CastlingRights(self.0 & (0b0011 << (color as u8)))
    }
}

impl BitAnd for CastlingRights {
    type Output = CastlingRights;

    fn bitand(self, other: Self) -> Self::Output {
        CastlingRights(self.0 & other.0)
    }
}

impl BitAnd<Right> for CastlingRights {
    type Output = CastlingRights;

    fn bitand(self, right: Right) -> Self::Output {
        CastlingRights(self.0 & (right as u8))
    }
}

impl BitOr for CastlingRights {
    type Output = CastlingRights;

    fn bitor(self, other: Self) -> Self::Output {
        CastlingRights(self.0 | other.0)
    }
}

impl BitOr<Right> for CastlingRights {
    type Output = CastlingRights;

    fn bitor(self, right: Right) -> Self::Output {
        CastlingRights(self.0 | (right as u8))
    }
}

impl BitXor for CastlingRights {
    type Output = CastlingRights;

    fn bitxor(self, other: Self) -> Self::Output {
        CastlingRights(self.0 ^ other.0)
    }
}

impl BitXor<Right> for CastlingRights {
    type Output = CastlingRights;

    fn bitxor(self, right: Right) -> Self::Output {
        CastlingRights(self.0 ^ (right as u8))
    }
}

impl BitAndAssign for CastlingRights {
    fn bitand_assign(&mut self, other: Self) {
        self.0 &= other.0;
    }
}

impl BitOrAssign for CastlingRights {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl BitXorAssign for CastlingRights {
    fn bitxor_assign(&mut self, other: Self) {
        self.0 ^= other.0;
    }
}

impl BitAndAssign<Right> for CastlingRights {
    fn bitand_assign(&mut self, other: Right) {
        self.0 &= other as u8;
    }
}

impl Default for CastlingRights {
    fn default() -> Self {
        Self::ALL
    }
}

impl IntoIterator for CastlingRights {
    type Item = Right;
    type IntoIter = RightIter;

    fn into_iter(self) -> Self::IntoIter {
        RightIter::new(self)
    }
}

/// An iterator over castling rights.
///
/// Order follows the conventional pattern (white king side, white queen side,
/// black king side, black queen side), yielding each right if present.
pub struct RightIter {
    value: u8,
    shift: usize,
}

impl RightIter {
    pub fn new(value: CastlingRights) -> Self {
        Self {
            value: value.0,
            shift: 0,
        }
    }
}

impl Iterator for RightIter {
    type Item = Right;

    fn next(&mut self) -> Option<Self::Item> {
        const LOOKUP: [Right; 4] = [
            Right::BlackQueenSide,
            Right::BlackKingSide,
            Right::WhiteQueenSide,
            Right::WhiteKingSide,
        ];

        // isolate msb, updating shift and value along the way
        let mut msb = 0;
        while msb == 0 && self.value > 0 {
            msb = self.value & 0b00001000;
            self.value = (self.value & 0b1111) << 1;
            self.shift += 1;
        }

        if msb == 0 {
            None
        } else {
            Some(LOOKUP[LOOKUP.len() - self.shift])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_order() {
        let rights = CastlingRights::ALL;
        let mut iter = rights.into_iter();
        assert_eq!(iter.next(), Some(Right::WhiteKingSide));
        assert_eq!(iter.next(), Some(Right::WhiteQueenSide));
        assert_eq!(iter.next(), Some(Right::BlackKingSide));
        assert_eq!(iter.next(), Some(Right::BlackQueenSide));
        assert_eq!(iter.next(), None);
    }
}

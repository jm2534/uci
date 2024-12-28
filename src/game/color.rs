use std::{ops::Not, str::FromStr};

use enum_iterator::Sequence;

#[derive(Debug)]
pub struct ColorParseError;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Sequence)]
pub enum Color {
    White,
    Black,
}

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "white" | "w" => Ok(Self::White),
            "black" | "b" => Ok(Self::Black),
            _ => Err(ColorParseError),
        }
    }
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Self::Output {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}

use std::{
    ops::{Index, IndexMut, Not},
    str::FromStr,
};

use enum_iterator::Sequence;

#[derive(Debug)]
pub struct ColorParseError;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Sequence)]
pub enum Color {
    Black,
    White,
}

impl From<bool> for Color {
    fn from(value: bool) -> Self {
        match value {
            true => Color::White,
            false => Color::Black,
        }
    }
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

impl<T> Index<Color> for [T] {
    type Output = T;

    fn index(&self, index: Color) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Color> for [T] {
    fn index_mut(&mut self, index: Color) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

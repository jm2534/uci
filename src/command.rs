use std::{collections::HashSet, fmt::Display};

use crate::game::{
    board::MoveError,
    moves::{Move, MoveParseError},
};
use thiserror::Error;

#[derive(Debug)]
pub enum Setting {}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Unrecognized command `{0}")]
    UnrecognizedCommand(String),

    #[error("Unrecognized command argument `{0}`")]
    UnrecognizedArgument(String),
}

enum PositionMode {
    Start,
    Fen(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Initialize a new UCI-based connection
    Uci,

    UciNewGame,

    Debug(bool),

    /// Series of moves to apply to a board in a starting configuration
    Position(Vec<Move>),

    IsReady,

    Register,

    Go,

    /// Stop calculating moves as soon as possible
    Stop,

    /// Quit the program as soon as possible
    Quit,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Command::Uci => "uci",
            Command::UciNewGame => "ucinewgame",
            Command::Debug(true) => "debug on",
            Command::Debug(false) => "debug off",
            Command::Position(vec) => "position",
            Command::IsReady => "isready",
            Command::Register => "register",
            Command::Go => "go",
            Command::Stop => "stop",
            Command::Quit => "quit",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<&str> for Command {
    type Error = CommandError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut tokens = value.split_whitespace().map(str::trim);

        let unrecognized = |value: &str| {
            Err::<Command, CommandError>(CommandError::UnrecognizedCommand(value.to_owned()))
        };

        while let Some(token) = tokens.next() {
            let result = match token {
                "uci" => Ok(Command::Uci),
                "ucinewgame" => Ok(Command::UciNewGame),
                "quit" => Ok(Command::Quit),
                "stop" => Ok(Command::Stop),
                "debug" => match tokens.next() {
                    Some("on") => Ok(Self::Debug(true)),
                    Some("off") => Ok(Self::Debug(false)),
                    Some(_) | None => unrecognized(value),
                },
                "isready" => Ok(Command::IsReady),
                "position" => {
                    // get starting position that follow-up moves reference
                    // let fenstring = match tokens.next() {
                    // Some("startpos") => None,
                    // Some("fenstring") => tokens.next().unwrap()),
                    // Some(_) | None => return unrecognized(value),
                    // };

                    if let Some("moves") = tokens.next() {
                        tokens
                            .map(Move::try_from)
                            .collect::<Result<Vec<Move>, MoveParseError>>()
                            .map(Command::Position)
                            .map_err(|_| CommandError::UnrecognizedArgument(value.to_owned()))
                    } else {
                        unrecognized(value)
                    }
                }
                "go" => match tokens.next() {
                    Some("ponder") => unimplemented!(),
                    Some(_) | None => unrecognized(value),
                },
                _ => continue,
            };
            return result;
        }

        unrecognized(value)
    }
}

/// Provides common methods for scanning iterators
// struct Scanner<T, S>
// where
//     S: Iterator<Item = T>,
// {
//     stream: S,
// }

// impl<T, S> Scanner<T, S>
// where
//     S: Iterator<Item = T>,
//     T: std::cmp::Eq,
// {
//     fn multifind_vectored(&mut self, elements: Vec<T>) -> Option<T> {
//         self.stream.find(|e| elements.contains(e))
//     }
// }

// impl<T, S> Scanner<T, S>
// where
//     S: Iterator<Item = T>,
//     T: std::cmp::Eq + std::hash::Hash,
// {
//     fn multifind(&mut self, elements: HashSet<T>) -> Option<T> {
//         self.stream.find(|e| elements.contains(e))
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::tile::Tile;

    #[test]
    fn test_debug_on() {
        let result = Command::try_from("abc  debug   on  ");
        assert_eq!(result.unwrap(), Command::Debug(true));
    }

    #[test]
    fn test_debug_off() {
        let result = Command::try_from("abc debug   off  ");
        assert_eq!(result.unwrap(), Command::Debug(false));
    }

    #[test]
    fn test_single_position() {
        let result = Command::try_from("abc position e2e4");
        assert_eq!(
            result.unwrap(),
            Command::Position(vec![Move {
                start: Tile { file: 4, rank: 1 },
                stop: Tile { file: 4, rank: 3 }
            }])
        )
    }

    #[test]
    fn test_multi_position() {
        let result = Command::try_from("  abc  position  def startpos  ghi  moves e2e4 b3b7 g1a4");
        assert_eq!(
            result.unwrap(),
            Command::Position(vec![
                Move {
                    start: Tile { file: 4, rank: 1 },
                    stop: Tile { file: 4, rank: 3 }
                },
                Move {
                    start: Tile { file: 1, rank: 2 },
                    stop: Tile { file: 1, rank: 6 }
                },
                Move {
                    start: Tile { file: 6, rank: 0 },
                    stop: Tile { file: 0, rank: 3 }
                }
            ])
        )
    }
}

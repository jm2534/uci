use std::fmt::Display;

use crate::game::{
    board::Board,
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

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Initialize a new UCI-based connection
    Uci,

    UciNewGame,

    Debug(bool),

    /// Series of moves to apply to a board in a starting configuration
    MovePosition(Vec<Move>),

    /// Starting configuration without speficiation of moves taken
    FenPosition(Board),

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
            Command::MovePosition(_) => "position startpos",
            Command::FenPosition(_) => "position fen",
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
                "go" => Ok(Command::Go),
                "debug" => match tokens.next() {
                    Some("on") => Ok(Self::Debug(true)),
                    Some("off") => Ok(Self::Debug(false)),
                    Some(_) | None => unrecognized(value),
                },
                "isready" => Ok(Command::IsReady),
                "position" => {
                    // get starting position that follow-up moves reference
                    match tokens.next() {
                        Some("fen") => {
                            let fenstring = tokens
                                .next()
                                .ok_or(CommandError::UnrecognizedCommand(value.to_owned()))?;

                            let board = Board::try_from(fenstring).map_err(|_| {
                                CommandError::UnrecognizedArgument(fenstring.to_owned())
                            })?;

                            Ok(Command::FenPosition(board))
                        }
                        Some("startpos") => {
                            if let Some("moves") = tokens.next() {
                                tokens
                                    .map(Move::try_from)
                                    .collect::<Result<Vec<Move>, MoveParseError>>()
                                    .map(Command::MovePosition)
                                    .map_err(|_| {
                                        CommandError::UnrecognizedArgument(value.to_owned())
                                    })
                            } else {
                                unrecognized(value)
                            }
                        }
                        Some(_) | None => return unrecognized(value),
                    }
                }
                _ => continue,
            };
            return result;
        }

        unrecognized(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tile::Tile;

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
        let result = Command::try_from("abc position startpos moves e2e4");
        assert_eq!(
            result.unwrap(),
            Command::MovePosition(vec![Move::new(Tile::new(1, 4), Tile::new(3, 4))])
        )
    }

    #[test]
    fn test_multi_position() {
        let result = Command::try_from("  abc  position startpos moves e2e4 e7e5");
        assert_eq!(
            result.unwrap(),
            Command::MovePosition(vec![
                Move::new(Tile::new(1, 4), Tile::new(3, 4)),
                Move::new(Tile::new(6, 4), Tile::new(4, 4)),
            ])
        )
    }
}

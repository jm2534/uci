mod strategy;
use std::env;
use strategy::{Strategy, minimax::Minimax};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

use crate::{
    command::Command,
    game::{
        board::{Board, MoveError},
        color::Color,
    },
};

pub struct Engine<S: Strategy> {
    /// The last received command
    state: Command,

    /// Debug mode; whether the engine should respond with supplemental `info` strings
    debug: bool,

    /// Board instance, representing the currently configured game state
    pub board: Board,

    /// The strategy external to the engine used for move searching
    strategy: S,
}

impl Default for Engine<Minimax> {
    fn default() -> Self {
        Engine {
            state: Command::Uci,
            board: Board::new(),
            debug: false,
            strategy: Minimax::new(Color::White),
        }
    }
}

impl<S: Strategy> Engine<S> {
    pub fn finished(&self) -> bool {
        self.board.winner().is_some()
    }

    pub fn handle(&mut self, command: Command) -> Result<Option<String>, MoveError> {
        let response = match command {
            Command::Uci => Ok(Some(format!(
                "id name {NAME}-{VERSION}\nid author {AUTHOR}\nuciok"
            ))),
            Command::UciNewGame => Ok(None),
            Command::Debug(debug) => {
                self.debug = debug;
                Ok(None)
            }
            Command::Position(ref moves) => {
                for attempt in moves {
                    if let Err(e) = self.board.try_move(*attempt) {
                        return Err(e);
                    }
                }
                Ok(None)
            }
            Command::IsReady => Ok(Some("readyok".to_string())),
            Command::Register => todo!(),
            Command::Go => {
                let best_move = self.strategy.step(&mut self.board);
                Ok(Some(best_move.to_string()))
            }
            Command::Stop => todo!(),
            Command::Quit => Ok(None),
        };

        if let Ok(_) = response {
            self.state = command;
        }
        response
    }
}

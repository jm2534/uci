pub mod strategy;
use std::env;
use strategy::{Strategy, minimax::Minimax};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

use crate::{
    command::Command,
    game::board::{Board, IllegalMove},
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
            strategy: Minimax,
        }
    }
}

impl<S: Strategy> Engine<S> {
    pub fn finished(&self) -> bool {
        self.board.winner().is_some()
    }

    pub fn handle(&mut self, command: Command) -> Result<Option<String>, IllegalMove> {
        let response = match command {
            Command::Uci => Ok(Some(format!(
                "id name {NAME}-{VERSION}\nid author {AUTHOR}\nuciok"
            ))),
            Command::UciNewGame => Ok(None),
            Command::Debug(debug) => {
                self.debug = debug;
                Ok(None)
            }
            Command::FenPosition(ref board) => {
                self.board = board.to_owned();
                Ok(None)
            }
            Command::MovePosition(ref moves) => {
                for attempt in moves {
                    self.board.try_move(*attempt)?;
                }
                Ok(None)
            }
            Command::IsReady => {
                Board::initialize();
                Ok(Some("readyok".to_string()))
            }
            Command::Register => todo!(),
            Command::Go => {
                let best_move = self.strategy.step(&mut self.board);
                Ok(Some(best_move.to_string()))
            }
            Command::Stop => todo!(),
            Command::Quit => Ok(None),
        };

        if response.is_ok() {
            self.state = command;
        }
        response
    }
}

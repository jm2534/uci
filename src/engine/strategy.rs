use crate::game::{board::Board, moves::Move};

pub mod minimax;

pub trait Strategy {
    fn step(&mut self, board: Board) -> Move;
}

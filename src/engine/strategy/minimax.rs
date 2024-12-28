use super::Strategy;
use crate::game::board::Board;
use crate::game::moves::Move;

pub struct Minimax {
    alpha: f32,
    beta: f32,
}

impl Minimax {
    pub fn new() -> Self {
        Self {
            alpha: 0.0,
            beta: 0.0,
        }
    }
}

impl Strategy for Minimax {
    fn step(&mut self, board: Board) -> Move {
        todo!()
    }
}

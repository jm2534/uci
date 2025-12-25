//! Move generation for chess pieces using bitboard techniques.
//!
//! This module implements efficient move generation for all piece types,
//! following established bitboard patterns from chess programming literature.

pub(crate) mod bishop;
mod king;
mod knight;
mod pawn;
mod queen;
pub(crate) mod rook;

#[cfg(test)]
mod tests {
    use crate::game::{
        board::{Board, tile::Tile},
        moves::Move,
        piece::PieceKind,
    };
    use std::collections::HashSet;

    #[test]
    fn test_initial_board_generation() {
        Board::initialize();
        let board = Board::new();
        let moves: HashSet<Move> = board
            .possible_moves(board.to_move())
            .map(|(_, action)| action)
            .collect();

        let mut expected_moves = HashSet::new();
        let pawns = board.positions[PieceKind::Pawn] & board.occupancy[board.to_move];
        for start in pawns.tiles() {
            let single_push = Move::new(start, Tile::new(start.rank() + 1, start.file()));
            expected_moves.insert(single_push);

            let double_push = Move::new(start, Tile::new(start.rank() + 2, start.file()));
            expected_moves.insert(double_push);
        }

        let knights = board.positions[PieceKind::Knight] & board.occupancy[board.to_move];
        for start in knights.tiles() {
            let left = Move::new(start, Tile::new(start.rank() + 2, start.file() - 1));
            let right = Move::new(start, Tile::new(start.rank() + 2, start.file() + 1));
            expected_moves.insert(left);
            expected_moves.insert(right);
        }

        assert_eq!(moves, expected_moves);
    }
}

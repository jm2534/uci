mod bitset;

use crate::game::{
    color::Color,
    piece::{Piece, PieceKind},
};
use bitset::Bitset;
use enum_iterator::cardinality;
use thiserror::Error;

use super::{
    moves::{Move, MoveKind},
    tile::Tile,
};

#[derive(Error, Copy, Clone, PartialEq, Eq, Debug)]
pub enum MoveError {
    #[error("Move not legal for piece: {0}")]
    IllegalMove(Move),

    #[error("No piece exists at position {0}")]
    NonexistentPiece(Tile),
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct Board {
    occupancy: Bitset,
    pieces: [Bitset; 12],
}

impl Board {
    const MAX_DIM: u8 = 8;
    const MIN_DIM: u8 = 0;

    pub fn new() -> Self {
        let pieces = [
            // white
            Bitset(0xFF << 8), // pawns
            Bitset::from(Tile { rank: 0, file: 2 }) | Tile { rank: 0, file: 5 }, // bishops,
            Bitset::from(Tile { rank: 0, file: 0 }) | Tile { rank: 0, file: 7 }, // rooks
            Bitset::from(Tile { rank: 0, file: 1 }) | Tile { rank: 0, file: 6 }, // knights
            Bitset::from(Tile { rank: 0, file: 3 }), // queen
            Bitset::from(Tile { rank: 0, file: 4 }), // king,
            // Black
            Bitset(0xFF << 48), // pawns
            Bitset::from(Tile { rank: 7, file: 2 }) | Tile { rank: 7, file: 5 }, // bishops
            Bitset::from(Tile { rank: 7, file: 0 }) | Tile { rank: 7, file: 7 }, // rooks
            Bitset::from(Tile { rank: 7, file: 1 }) | Tile { rank: 7, file: 6 }, // knights
            Bitset::from(Tile { rank: 7, file: 3 }), // queen
            Bitset::from(Tile { rank: 7, file: 4 }), // king
        ];

        let occupancy = Bitset::union(pieces.iter());

        Self { pieces, occupancy }
    }

    fn get_set(&self, piece: Piece) -> Bitset {
        self.pieces[piece.kind as usize + piece.color as usize * 6]
    }

    fn get_pieces(&self, color: Color) -> &[Bitset] {
        match color {
            Color::Black => &self.pieces[..6],
            Color::White => &self.pieces[6..],
        }
    }

    pub fn empty() -> Self {
        Board {
            pieces: [Bitset(0); 12],
            occupancy: Bitset(0),
        }
    }

    fn occupied(&self, tile: &Tile) -> bool {
        (self.occupancy & tile).0 > 0
    }

    fn occupant(&self, tile: &Tile) -> Option<Piece> {
        match self.occupied(tile) {
            false => None,
            true => {
                let kind: PieceKind;
                let color: Color;
                for (i, set) in self.pieces.iter().enumerate() {
                    if !set.is_empty() && !(set & tile).is_empty() {
                        color = match i {
                            ..=5 => Color::White,
                            6..=11 => Color::Black,
                            _ => panic!("{} out of bounds", i),
                        };

                        kind = match i % cardinality::<PieceKind>() {
                            0 => PieceKind::Pawn,
                            1 => PieceKind::Bishop,
                            2 => PieceKind::Rook,
                            3 => PieceKind::Knight,
                            4 => PieceKind::Queen,
                            5 => PieceKind::King,
                            _ => panic!("{} out of bounds", i),
                        };

                        return Some(Piece { kind, color });
                    }
                }
                panic!("")
            }
        }
    }

    pub fn try_move(&mut self, attempt: Move) -> Result<MoveKind, MoveError> {
        let mut kind = Err(MoveError::NonexistentPiece(attempt.start));
        if let Some(actor) = self.occupant(&attempt.start) {
            let moveset = Bitset(0);
            if moveset.contains(&attempt.stop) {
                // Toggle origin bit and set destination bit
                let mut actor_set = self.get_set(actor);
                actor_set.toggle(&attempt.start);
                actor_set.insert(&attempt.stop);

                // Check capture
                kind = match self.occupant(&attempt.stop) {
                    None => Ok(MoveKind::Quiet),
                    Some(target) => {
                        let mut target_set = self.get_set(target);
                        target_set.toggle(&attempt.stop);
                        Ok(MoveKind::Capture(target))
                    }
                };

                // Toggle origin, set destination for occupancy lookup
                self.occupancy.toggle(&attempt.start);
                self.occupancy |= Bitset::from(attempt.stop);

                // TODO: check for other types
            } else {
                // else move not in moveset
                kind = Err(MoveError::IllegalMove(attempt));
            }
        }
        kind
    }

    fn flatten(&self) -> Vec<Option<Piece>> {
        let t = Tile::from_index;
        let f = |i| self.occupant(&t(i));
        (0..64).map(f).collect()
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Board;
    use crate::game::color::Color;
    use crate::{
        game::board::{
            bitset::{Bitset, Offset},
            Tile,
        },
        game::piece::{Piece, PieceKind},
    };
    use std::{collections::HashSet, convert::*};

    fn correct_starting_piece(tile: Tile) -> Option<Piece> {
        let (row, col) = (tile.rank, tile.file);

        let color: Option<Color> = match row {
            0 | 1 => Some(Color::White),
            6 | 7 => Some(Color::Black),
            _ => None,
        };

        if let Some(color) = color {
            let kind = match (row, col) {
                (1, _) | (6, _) => PieceKind::Pawn,
                (_, 0) | (_, 7) => PieceKind::Rook,
                (_, 1) | (_, 6) => PieceKind::Knight,
                (_, 2) | (_, 5) => PieceKind::Bishop,
                (_, 3) => PieceKind::Queen,
                (_, 4) => PieceKind::King,
                _ => panic!("Unmatched row, col"),
            };
            return Some(Piece { kind, color });
        }
        None
    }

    #[test]
    fn test_int_from_tile() {
        assert_eq!(u64::from(Tile { rank: 0, file: 0 }), 0x1); // a1
        assert_eq!(u64::from(Tile { rank: 0, file: 1 }), 0x2); // b1
        assert_eq!(u64::from(Tile { rank: 1, file: 0 }), 0b100000000); // a2
        assert_eq!(u64::from(Tile { rank: 7, file: 7 }), 0x8000000000000000); // h8
    }

    #[test]
    fn test_tile_from_index() {
        assert_eq!(Tile::from_index(0), Tile { rank: 0, file: 0 }); // a1
        assert_eq!(Tile::from_index(1), Tile { rank: 0, file: 1 }); // b1
        assert_eq!(Tile::from_index(8), Tile { rank: 1, file: 0 }); // a2
        assert_eq!(Tile::from_index(63), Tile { rank: 7, file: 7 }); // h8
    }

    #[test]
    fn test_board_equality() {
        let board1 = Board::new();
        let board2 = Board::new();

        assert_eq!(board1, board2);
        assert_ne!(board1, Board::empty());

        let mut board_set = HashSet::new();
        board_set.insert(board1);
        assert!(board_set.contains(&board2));
    }

    #[test]
    fn test_offset() {
        let mut offset = Bitset(0);
        offset.offset(Offset::None);
        assert_eq!(Bitset(0), offset);

        let mut offset = Bitset(0xFF);
        offset.offset(Offset::North);
        assert_eq!(Bitset(0xFF << 8), offset);

        let mut offset = Bitset(0xFF00);
        offset.offset(Offset::South);
        assert_eq!(Bitset(0xFF00 >> 8), offset)
        // TODO: handle edge cases
    }

    #[test]
    fn test_flatten() {
        let board = Board::new();
        let flattened = board.flatten();

        assert_eq!(flattened.len(), 64);
        assert_eq!(flattened.iter().filter(|p| p.is_some()).count(), 32);
        for (i, piece) in flattened.iter().enumerate() {
            let tile = Tile::from_index(i);
            assert_eq!(piece, &correct_starting_piece(tile))
        }
    }

    #[test]
    fn test_placement() {
        let board = Board::new();
        for row in 0..2 {
            for col in 0..Board::MAX_DIM {
                let start = Tile {
                    rank: row as usize,
                    file: col as usize,
                };
                let occupant = board.occupant(&start);

                if row > Board::MIN_DIM + 2 && row < Board::MIN_DIM + 6 {
                    // In between white and black
                    assert!(occupant.is_none())
                } else {
                    assert!(
                        occupant.is_some(),
                        "Could not find any piece at {:?}",
                        start
                    );

                    let desired = correct_starting_piece(start).unwrap();
                    let actual = occupant.unwrap().to_owned();
                    assert_eq!(
                        actual, desired,
                        "Found {:?} at {:?} instead of {:?}",
                        actual, start, desired
                    );
                }
            }
        }
    }
}

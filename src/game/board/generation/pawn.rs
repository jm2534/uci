//! Pawn move generation using bitboard techniques.

use super::super::{Bitset, Board};
use crate::game::{
    board::{bitset::Offset, tile::Tile},
    color::Color,
};

/// Pre-computed pawn attack patterns indexed by [color][square].
pub const PAWN_ATTACKS: [[Bitset; 64]; 2] = generate_pawn_attacks_table();

impl Board {
    /// Generate all pseudo-legal moves assuming a pawn at the given tile.
    pub fn pawn_moves(&self, tile: Tile) -> Bitset {
        // pawn moves are simply single/double pushes + attacks
        self.pawn_pushes(tile) | self.pawn_captures(tile)
    }

    /// Generate pseudo-legal single and double pawn pushes assuming a pawn at the given tile.
    fn pawn_pushes(&self, tile: Tile) -> Bitset {
        let (forward_offset, starting_rank) = match self.to_move {
            Color::White => (Offset::North, Self::RANK_MASKS[1]),
            Color::Black => (Offset::South, Self::RANK_MASKS[6]),
        };

        // single push
        let mut pushes = Bitset(0);
        let all_occupied = self.occupancy[Color::White] | self.occupancy[Color::Black];
        let single_push = tile.as_bitset().offset(forward_offset);
        if (single_push & all_occupied).is_empty() {
            pushes |= single_push;

            // double push from starting rank
            if (tile.as_bitset() & starting_rank) != Bitset(0) {
                let double_push = single_push.offset(forward_offset);
                if (double_push & all_occupied).is_empty() {
                    pushes |= double_push;
                }
            }
        }

        pushes
    }

    /// Generate pseudo-legal pawn capture moves, including en passant captures, assuming
    /// a pawn at the given tile.
    fn pawn_captures(&self, tile: Tile) -> Bitset {
        // TODO: Add en passant captures when board state supports it
        // can only capture squares with enemy pieces
        let enemy_occupied = self.occupancy[!self.to_move];
        let attack_pattern = PAWN_ATTACKS[self.to_move as usize][tile.as_index()];
        attack_pattern & enemy_occupied
    }
}

/// Generate all pawn attack patterns at compile time.
const fn generate_pawn_attacks_table() -> [[Bitset; 64]; 2] {
    let mut table = [[Bitset(0); 64]; 2];

    let mut i = 0;
    while i < 64 {
        let tile = Tile::from_index(i);
        table[Color::Black as usize][i] = generate_black_pawn_attacks(tile);
        table[Color::White as usize][i] = generate_white_pawn_attacks(tile);
        i += 1;
    }

    table
}

const fn generate_white_pawn_attacks(tile: Tile) -> Bitset {
    let mut attacks = 0u64;
    attacks |= tile.as_bitset().0 << 9 & !Board::FILE_MASKS[0].0;
    attacks |= tile.as_bitset().0 << 7 & !Board::FILE_MASKS[7].0;
    Bitset(attacks)
}

const fn generate_black_pawn_attacks(tile: Tile) -> Bitset {
    let mut attacks = 0u64;
    attacks |= tile.as_bitset().0 >> 7 & !Board::FILE_MASKS[0].0;
    attacks |= tile.as_bitset().0 >> 9 & !Board::FILE_MASKS[7].0;
    Bitset(attacks)
}

#[cfg(test)]
mod attack_tests {
    use super::*;
    use crate::game::color::Color;

    #[test]
    fn test_white_pawn_center_attacks() {
        // white pawn on d4 should attack c5 and e5
        let index = Tile::try_from("d4").unwrap().as_index();
        let attacks = PAWN_ATTACKS[Color::White as usize][index];
        let expected = Tile::try_from("c5").unwrap() | Tile::try_from("e5").unwrap();
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_black_pawn_center_attacks() {
        // black pawn on d5 should attack c4 and e4
        let index = Tile::try_from("d5").unwrap().as_index();
        let attacks = PAWN_ATTACKS[Color::Black as usize][index];
        let expected = Tile::try_from("c4").unwrap() | Tile::try_from("e4").unwrap();
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_white_pawn_file_edge_attack() {
        // white pawn on a4 should attack b5
        let index = Tile::try_from("a4").unwrap().as_index();
        let attacks = PAWN_ATTACKS[Color::White as usize][index];
        let expected = Tile::try_from("b5").unwrap().as_bitset();
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_black_pawn_file_edge_attack() {
        // black pawn on a5 should attack b4
        let index = Tile::try_from("a5").unwrap().as_index();
        let attacks = PAWN_ATTACKS[Color::Black as usize][index];
        let expected = Tile::try_from("b4").unwrap().as_bitset();
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_white_pawn_rank_edge_attacks() {
        // white pawn on last rank should not attack
        let index = Tile::try_from("h8").unwrap().as_index();
        let attacks = PAWN_ATTACKS[Color::White as usize][index];
        let expected = Bitset(0);
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_black_pawn_rank_edge_attacks() {
        // black pawn on a5 should attack b4
        let index = Tile::try_from("a1").unwrap().as_index();
        let attacks = PAWN_ATTACKS[Color::Black as usize][index];
        let expected = Bitset(0);
        assert_eq!(attacks, expected);
    }
}

#[cfg(test)]
mod generation_tests {
    use super::*;
    use crate::game::{board::tile::Tile, color::Color};

    #[test]
    fn test_white_pawn_single_push() {
        let mut board = Board::new();
        // Clear all occupancy for clean test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Place a white pawn on e4 (not starting rank)
        let pawn_tile = Tile::try_from("e4").unwrap();
        board.occupancy[Color::White] = pawn_tile.as_bitset();

        let moves = board.pawn_moves(pawn_tile);

        // Should only be able to push to e5 (single push)
        let expected = Tile::try_from("e5").unwrap().as_bitset();
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_white_pawn_double_push() {
        let mut board = Board::new();
        // Clear all occupancy for clean test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Place a white pawn on e2 (starting rank)
        let pawn_tile = Tile::try_from("e2").unwrap();
        board.occupancy[Color::White] = pawn_tile.as_bitset();

        let moves = board.pawn_moves(pawn_tile);

        // Should be able to push to both e3 and e4 (single and double push)
        let expected = Tile::try_from("e3").unwrap() | Tile::try_from("e4").unwrap();
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_black_pawn_single_push() {
        let mut board = Board::new();
        // Clear all occupancy for clean test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::Black;

        // Place a black pawn on d5 (not starting rank)
        let pawn_tile = Tile::try_from("d5").unwrap();
        board.occupancy[Color::Black] = pawn_tile.as_bitset();

        let moves = board.pawn_moves(pawn_tile);

        // Should only be able to push to d4 (single push for black)
        let expected = Tile::try_from("d4").unwrap().as_bitset();
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_pawn_captures() {
        let mut board = Board::new();
        // Clear all occupancy for clean test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Place a white pawn on d4
        let pawn_tile = Tile::try_from("d4").unwrap();
        board.occupancy[Color::White] = pawn_tile.as_bitset();

        // Place black pieces on diagonal capture squares
        let enemy_squares = Tile::try_from("c5").unwrap() | Tile::try_from("e5").unwrap();
        board.occupancy[Color::Black] = enemy_squares;

        let moves = board.pawn_moves(pawn_tile);

        // Should be able to push forward and capture diagonally
        let expected = Tile::try_from("d5").unwrap() | enemy_squares;
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_pawn_blocked_push() {
        let mut board = Board::new();
        // Clear all occupancy for clean test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Place a white pawn on e2 (starting rank)
        let pawn_tile = Tile::try_from("e2").unwrap();
        board.occupancy[Color::White] = pawn_tile.as_bitset();

        // Block the single push square with any piece
        let blocking_square = Tile::try_from("e3").unwrap();
        board.occupancy[Color::Black] = blocking_square.as_bitset();

        let moves = board.pawn_moves(pawn_tile);

        // Should not be able to move at all (blocked)
        assert_eq!(moves, Bitset(0));
    }

    #[test]
    fn test_pawn_edge_captures() {
        let mut board = Board::new();
        // Clear all occupancy for clean test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Test pawn on A file
        let a_pawn_tile = Tile::try_from("a4").unwrap();
        board.occupancy[Color::White] = a_pawn_tile.as_bitset();

        // Place enemy piece on the only valid capture square
        let enemy_square = Tile::try_from("b5").unwrap();
        board.occupancy[Color::Black] = enemy_square.as_bitset();

        let moves = board.pawn_moves(a_pawn_tile);

        // Should be able to push forward and capture to the right only
        let expected = Tile::try_from("a5").unwrap() | enemy_square;
        assert_eq!(moves, expected);

        // Test pawn on H file
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);

        let h_pawn_tile = Tile::try_from("h4").unwrap();
        board.occupancy[Color::White] = h_pawn_tile.as_bitset();

        // Place enemy piece on the only valid capture square
        let enemy_square = Tile::try_from("g5").unwrap();
        board.occupancy[Color::Black] = enemy_square.as_bitset();

        let moves = board.pawn_moves(h_pawn_tile);

        // Should be able to push forward and capture to the left only
        let expected = Tile::try_from("h5").unwrap() | enemy_square;
        assert_eq!(moves, expected);
    }
}

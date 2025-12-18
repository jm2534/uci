use super::super::{Bitset, Board};
use crate::game::board::tile::Tile;

/// Pre-computed king attack patterns indexed by square.
/// Kings move the same regardless of color, so there is no need to distinguish between players.
pub const KING_ATTACKS: [Bitset; 64] = generate_king_attacks_table();

impl Board {
    /// Generate all pseudo-legal moves assuming a king at the given tile.
    pub(super) fn king_moves(&self, tile: Tile) -> Bitset {
        // kings can move to any square in their attack pattern not occupied by own pieces
        KING_ATTACKS[tile.as_index()] & !self.occupancy[self.to_move]
    }
}

/// Generate all king attack patterns at compile time.
const fn generate_king_attacks_table() -> [Bitset; 64] {
    let mut table = [Bitset(0); 64];

    let mut i = 0;
    while i < 64 {
        let tile = Tile::from_index(i);
        table[i] = generate_king_attacks(tile);
        i += 1;
    }

    table
}

const fn generate_king_attacks(tile: Tile) -> Bitset {
    let square = tile.as_bitset().0;
    let not_a_file = !Board::FILE_MASKS[0].0;
    let not_h_file = !Board::FILE_MASKS[7].0;

    let attacks = ((square << 8)) // north
                | ((square >> 8)) // south
                | ((square << 1) & not_a_file) // east
                | ((square >> 1) & not_h_file) // west
                | ((square << 9) & not_a_file) // northeast
                | ((square << 7) & not_h_file) // northwest
                | ((square >> 7) & not_a_file) // southeast
                | ((square >> 9) & not_h_file); // southwest

    Bitset(attacks)
}

#[cfg(test)]
mod attack_tests {
    use super::*;

    #[test]
    fn test_king_center_attacks() {
        // King on d4 should attack 8 squares: c3, d3, e3, c4, e4, c5, d5, e5
        let index = Tile::try_from("d4").unwrap().as_index();
        let attacks = KING_ATTACKS[index];

        let expected = Tile::try_from("c3").unwrap()
            | Tile::try_from("d3").unwrap()
            | Tile::try_from("e3").unwrap()
            | Tile::try_from("c4").unwrap()
            | Tile::try_from("e4").unwrap()
            | Tile::try_from("c5").unwrap()
            | Tile::try_from("d5").unwrap()
            | Tile::try_from("e5").unwrap();

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_king_corner_attacks() {
        // King on a1 should attack only 3 squares: a2, b1, b2
        let index = Tile::try_from("a1").unwrap().as_index();
        let attacks = KING_ATTACKS[index];

        let expected = Tile::try_from("a2").unwrap()
            | Tile::try_from("b1").unwrap()
            | Tile::try_from("b2").unwrap();

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_king_h8_corner_attacks() {
        // King on h8 should attack only 3 squares: g8, g7, h7
        let index = Tile::try_from("h8").unwrap().as_index();
        let attacks = KING_ATTACKS[index];

        let expected = Tile::try_from("g8").unwrap()
            | Tile::try_from("g7").unwrap()
            | Tile::try_from("h7").unwrap();

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_king_edge_attacks() {
        // King on d1 (bottom edge) should attack 5 squares: c1, e1, c2, d2, e2
        let index = Tile::try_from("d1").unwrap().as_index();
        let attacks = KING_ATTACKS[index];

        let expected = Tile::try_from("c1").unwrap()
            | Tile::try_from("e1").unwrap()
            | Tile::try_from("c2").unwrap()
            | Tile::try_from("d2").unwrap()
            | Tile::try_from("e2").unwrap();

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_king_side_edge_attacks() {
        // King on a4 (left edge) should attack 5 squares: a3, a5, b3, b4, b5
        let index = Tile::try_from("a4").unwrap().as_index();
        let attacks = KING_ATTACKS[index];

        let expected = Tile::try_from("a3").unwrap()
            | Tile::try_from("a5").unwrap()
            | Tile::try_from("b3").unwrap()
            | Tile::try_from("b4").unwrap()
            | Tile::try_from("b5").unwrap();

        assert_eq!(attacks, expected);
    }
}

#[cfg(test)]
mod generation_tests {
    use super::*;
    use crate::game::{board::Board, color::Color};

    #[test]
    fn test_king_moves_empty_board() {
        // Test king moves on an empty board - should return all attack squares
        let mut board = Board::new();
        // Clear all occupancy for empty board test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        let king_tile = Tile::try_from("d4").unwrap();
        let moves = board.king_moves(king_tile);

        // Should match the attack pattern exactly since no pieces block
        let expected = KING_ATTACKS[king_tile.as_index()];
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_king_moves_blocked_by_own_pieces() {
        let mut board = Board::new();
        // Clear all occupancy first
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Place white pieces on some potential king destination squares
        let blocked_squares = Tile::try_from("c4").unwrap() | Tile::try_from("d5").unwrap();
        board.occupancy[Color::White] = blocked_squares;

        let king_tile = Tile::try_from("d4").unwrap();
        let moves = board.king_moves(king_tile);

        // King should not be able to move to squares occupied by own pieces
        assert!((moves & blocked_squares).is_empty());

        // But should still be able to move to other squares
        let free_square = Tile::try_from("e4").unwrap();
        assert!((moves & free_square.as_bitset()) != Bitset(0));
    }

    #[test]
    fn test_king_captures_enemy_pieces() {
        let mut board = Board::new();
        // Clear all occupancy first
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Place enemy pieces on some potential king destination squares
        let enemy_squares = Tile::try_from("c4").unwrap() | Tile::try_from("e5").unwrap();
        board.occupancy[Color::Black] = enemy_squares;

        let king_tile = Tile::try_from("d4").unwrap();
        let moves = board.king_moves(king_tile);

        // King should be able to capture enemy pieces
        assert!((moves & enemy_squares) == enemy_squares);
    }
}

use super::super::{Bitset, Board};
use crate::game::board::tile::Tile;

/// Pre-computed knight attack patterns indexed by square.
/// Knights move the same regardless of color, so there is no need to distinguish between players.
pub const KNIGHT_ATTACKS: [Bitset; 64] = generate_knight_attacks_table();

impl Board {
    /// Generate all pseudo-legal moves assuming a knight at the given tile.
    pub(super) fn knight_moves(&self, tile: Tile) -> Bitset {
        // knights can move to any square in their attack pattern not occupied by own pieces
        KNIGHT_ATTACKS[tile.as_index()] & !self.occupancy[self.to_move]
    }
}

/// Generate all knight attack patterns at compile time.
const fn generate_knight_attacks_table() -> [Bitset; 64] {
    let mut table = [Bitset(0); 64];

    let mut i = 0;
    while i < 64 {
        let tile = Tile::from_index(i);
        table[i] = generate_knight_attacks(tile);
        i += 1;
    }

    table
}

const fn generate_knight_attacks(tile: Tile) -> Bitset {
    let square = tile.as_bitset().0;
    let not_a_file = !Board::FILE_MASKS[0].0;
    let not_h_file = !Board::FILE_MASKS[7].0;
    let not_ab_file = !(Board::FILE_MASKS[0].0 | Board::FILE_MASKS[1].0);
    let not_gh_file = !(Board::FILE_MASKS[6].0 | Board::FILE_MASKS[7].0);

    // see https://www.chessprogramming.org/Knight_Pattern
    let attacks = ((square << 17) & not_a_file)   // Up 2, Right 1
                | ((square << 10) & not_ab_file)  // Up 1, Right 2
                | ((square >>  6) & not_ab_file)  // Down 1, Right 2
                | ((square >> 15) & not_a_file)   // Down 2, Right 1
                | ((square << 15) & not_h_file)   // Up 2, Left 1
                | ((square <<  6) & not_gh_file)  // Up 1, Left 2
                | ((square >> 10) & not_gh_file)  // Down 1, Left 2
                | ((square >> 17) & not_h_file); // Down 2, Left 1

    Bitset(attacks)
}

#[cfg(test)]
mod attack_tests {
    use super::*;

    #[test]
    fn test_knight_center_attacks() {
        // Knight on d4 should attack 8 squares: c2, e2, b3, f3, b5, f5, c6, e6
        let index = Tile::try_from("d4").unwrap().as_index();
        let attacks = KNIGHT_ATTACKS[index];

        let expected = Tile::try_from("c2").unwrap()
            | Tile::try_from("e2").unwrap()
            | Tile::try_from("b3").unwrap()
            | Tile::try_from("f3").unwrap()
            | Tile::try_from("b5").unwrap()
            | Tile::try_from("f5").unwrap()
            | Tile::try_from("c6").unwrap()
            | Tile::try_from("e6").unwrap();

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_knight_corner_attacks() {
        // Knight on a1 should attack only 2 squares: b3, c2
        let index = Tile::try_from("a1").unwrap().as_index();
        let attacks = KNIGHT_ATTACKS[index];

        let expected = Tile::try_from("b3").unwrap() | Tile::try_from("c2").unwrap();

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_knight_h8_corner_attacks() {
        // Knight on h8 should attack only 2 squares: f7, g6
        let index = Tile::try_from("h8").unwrap().as_index();
        let attacks = KNIGHT_ATTACKS[index];

        let expected = Tile::try_from("f7").unwrap() | Tile::try_from("g6").unwrap();

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_knight_edge_attacks() {
        // Knight on a4 (edge but not corner) should attack 4 squares: b2, c3, c5, b6
        let index = Tile::try_from("a4").unwrap().as_index();
        let attacks = KNIGHT_ATTACKS[index];

        let expected = Tile::try_from("b2").unwrap()
            | Tile::try_from("c3").unwrap()
            | Tile::try_from("c5").unwrap()
            | Tile::try_from("b6").unwrap();

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_knight_near_edge_attacks() {
        // Knight on b2 should attack 4 squares: a4, c4, d1, d3
        let index = Tile::try_from("b2").unwrap().as_index();
        let attacks = KNIGHT_ATTACKS[index];

        let expected = Tile::try_from("a4").unwrap()
            | Tile::try_from("c4").unwrap()
            | Tile::try_from("d1").unwrap()
            | Tile::try_from("d3").unwrap();

        assert_eq!(attacks, expected);
    }
}

#[cfg(test)]
mod generation_tests {
    use super::*;
    use crate::game::{board::Board, color::Color};

    #[test]
    fn test_knight_moves_empty_board() {
        // Test knight moves on an empty board - should return all attack squares
        let mut board = Board::new();
        // Clear all occupancy for empty board test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        let knight_tile = Tile::try_from("d4").unwrap();
        let moves = board.knight_moves(knight_tile);

        // Should match the attack pattern exactly since no pieces block
        let expected = KNIGHT_ATTACKS[knight_tile.as_index()];
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_knight_moves_blocked_by_own_pieces() {
        let mut board = Board::new();
        // Clear all occupancy first
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Place white pieces on some potential knight destination squares
        let blocked_squares = Tile::try_from("c2").unwrap() | Tile::try_from("f3").unwrap();
        board.occupancy[Color::White] = blocked_squares;

        let knight_tile = Tile::try_from("d4").unwrap();
        let moves = board.knight_moves(knight_tile);

        // Knight should not be able to move to squares occupied by own pieces
        assert!((moves & blocked_squares).is_empty());

        // But should still be able to move to other squares
        let free_square = Tile::try_from("e2").unwrap();
        assert!((moves & free_square.as_bitset()) != Bitset(0));
    }

    #[test]
    fn test_knight_captures_enemy_pieces() {
        let mut board = Board::new();
        // Clear all occupancy first
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        // Place enemy pieces on some potential knight destination squares
        let enemy_squares = Tile::try_from("c2").unwrap() | Tile::try_from("f5").unwrap();
        board.occupancy[Color::Black] = enemy_squares;

        let knight_tile = Tile::try_from("d4").unwrap();
        let moves = board.knight_moves(knight_tile);

        // Knight should be able to capture enemy pieces
        assert!((moves & enemy_squares) == enemy_squares);
    }
}

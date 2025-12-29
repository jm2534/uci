use super::super::{Bitset, Board};
use crate::game::{Color, board::tile::Tile};

impl Board {
    /// Pre-computed knight attack patterns indexed by square.
    /// Knights move the same regardless of color, so there is no need to distinguish between players.
    pub(in crate::game::board) const KNIGHT_MOVES: [Bitset; 64] = generate_knight_moves_table();

    /// Generate all pseudo-legal moves assuming a knight at the given tile.
    pub fn knight_moves(&self, color: Color, tile: Tile) -> Bitset {
        // knights can move to any square in their attack pattern not occupied by own pieces
        Board::KNIGHT_MOVES[tile] & !self.occupancy[color]
    }
}

/// Generate all knight attack patterns at compile time.
const fn generate_knight_moves_table() -> [Bitset; 64] {
    let mut table = [Bitset(0); 64];

    let mut i = 0;
    while i < 64 {
        let tile = Tile::from_index(i);
        table[i] = generate_knight_moves(tile);
        i += 1;
    }

    table
}

const fn generate_knight_moves(tile: Tile) -> Bitset {
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
mod tests {
    use super::*;
    use crate::game::Color;

    #[test]
    fn test_knight_center() {
        // Knight on d4 should attack 8 squares: c2, e2, b3, f3, b5, f5, c6, e6
        let index = Tile::D4;
        let attacks = Board::KNIGHT_MOVES[index];

        let expected =
            Tile::C2 | Tile::E2 | Tile::B3 | Tile::F3 | Tile::B5 | Tile::F5 | Tile::C6 | Tile::E6;
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_knight_corner() {
        // Knight on a1 should attack only 2 squares: b3, c2
        let index = Tile::A1;
        let attacks = Board::KNIGHT_MOVES[index];

        let expected = Tile::B3 | Tile::C2;

        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_knight_edge() {
        // Knight on a4 (edge but not corner) should attack 4 squares: b2, c3, c5, b6
        let index = Tile::A4;
        let attacks = Board::KNIGHT_MOVES[index];

        let expected = Tile::B2 | Tile::C3 | Tile::C5 | Tile::B6;
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_knight_near_edge() {
        // Knight on b2 should attack 4 squares: a4, c4, d1, d3
        let index = Tile::B2;
        let attacks = Board::KNIGHT_MOVES[index];

        let expected = Tile::A4 | Tile::C4 | Tile::D1 | Tile::D3;
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_knight_moves_empty_board() {
        // Test knight moves on an empty board - should return all attack squares
        let mut board = Board::new();
        // Clear all occupancy for empty board test
        board.occupancy[Color::White] = Bitset(0);
        board.occupancy[Color::Black] = Bitset(0);
        board.to_move = Color::White;

        let knight_tile = Tile::D4;
        let moves = board.knight_moves(board.to_move, knight_tile);

        // Should match the attack pattern exactly since no pieces block
        let expected = Board::KNIGHT_MOVES[knight_tile.as_index()];
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_knight_moves_blocked() {
        let mut board = Board::empty();
        let blocked_squares = Tile::C2 | Tile::F3;
        board.occupancy[Color::White] = blocked_squares;

        let knight_tile = Tile::D4;
        let moves = board.knight_moves(board.to_move, knight_tile);

        // Knight should not be able to move to squares occupied by own pieces
        assert!((moves & blocked_squares).is_empty());

        // But should still be able to move to other squares
        let free_square = Tile::E2;
        assert!((moves & free_square.as_bitset()) != Bitset(0));
    }

    #[test]
    fn test_knight_captures() {
        let mut board = Board::empty();
        let enemy_squares = Tile::C2 | Tile::F5;
        board.occupancy[Color::Black] = enemy_squares;

        let knight_tile = Tile::D4;
        let moves = board.knight_moves(board.to_move, knight_tile);

        // Knight should be able to capture enemy pieces
        assert!((moves & enemy_squares) == enemy_squares);
    }
}

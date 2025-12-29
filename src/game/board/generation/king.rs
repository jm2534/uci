use super::super::{Bitset, Board};
use crate::game::{
    Color,
    board::{Right, tile::Tile},
};

impl Board {
    /// Pre-computed king attack patterns indexed by square.
    /// Kings move the same regardless of color, so there is no need to distinguish between players.
    pub(in crate::game::board) const KING_MOVES: [Bitset; 64] = generate_king_moves_table();

    const KING_STARTING_POSITIONS: [Tile; 2] = [Tile::E8, Tile::E1];
    const WHITE_KINGSIDE_PATH: Bitset = Bitset((1 << 5) | (1 << 6)); // f1, g1
    const WHITE_QUEENSIDE_PATH: Bitset = Bitset((1 << 1) | (1 << 2) | (1 << 3)); // b1, c1, d1
    const BLACK_KINGSIDE_PATH: Bitset = Bitset((1 << 61) | (1 << 62)); // f8, g8
    const BLACK_QUEENSIDE_PATH: Bitset = Bitset((1 << 57) | (1 << 58) | (1 << 59)); // b8, c8, d8

    /// Generate pseudo-legal moves assuming a king at the given tile, excluding castling moves.
    pub fn king_moves(&self, color: Color, tile: Tile) -> Bitset {
        // kings can move to any square in their attack pattern not occupied by own pieces
        Self::KING_MOVES[tile] & !self.occupancy[color]
    }

    /// Generate all castling moves assuming a king at the given tile.
    /// Castling conditions are as follows:
    ///
    /// 1. The king and the relevant rook must not have moved previously (encoded in board's castling rights)
    /// 2. the king must not currently be in check
    /// 3. The king must not pass through a square targetable by the other player
    ///
    /// Returns a castling moveset that complies with these requirements without altering
    /// the board's current castling rights.
    pub fn castling_moves(&self, color: Color, tile: Tile) -> Bitset {
        let mut moves = Bitset(0);
        if tile != Board::KING_STARTING_POSITIONS[color] {
            return Bitset(0);
        }

        let rights = self.castling_rights;
        let all_occupancy = self.occupancy[0] | self.occupancy[1];
        if color == Color::Black && !self.is_attacked(tile, Color::White) {
            if rights.has(Right::BlackQueenSide)
                && (Board::BLACK_QUEENSIDE_PATH & all_occupancy).is_empty()
                && !self.is_attacked(Tile::D8, Color::White)
                && !self.is_attacked(Tile::E8, Color::White)
            {
                moves |= Tile::C8;
            }

            if rights.has(Right::BlackKingSide)
                && (Board::BLACK_KINGSIDE_PATH & all_occupancy).is_empty()
                && !self.is_attacked(Tile::F8, Color::White)
                && !self.is_attacked(Tile::G8, Color::White)
            {
                moves |= Tile::G8;
            }
        } else if !self.is_attacked(tile, Color::Black) {
            if rights.has(Right::WhiteQueenSide)
                && (Board::WHITE_QUEENSIDE_PATH & all_occupancy).is_empty()
                && !self.is_attacked(Tile::C1, Color::Black)
                && !self.is_attacked(Tile::D1, Color::Black)
            {
                moves |= Tile::C1;
            }

            if rights.has(Right::WhiteKingSide)
                && (Board::WHITE_KINGSIDE_PATH & all_occupancy).is_empty()
                && !self.is_attacked(Tile::F1, Color::Black)
                && !self.is_attacked(Tile::G1, Color::Black)
            {
                moves |= Tile::G1;
            }
        }

        moves
    }
}

/// Generate all king attack patterns at compile time.
const fn generate_king_moves_table() -> [Bitset; 64] {
    let mut table = [Bitset(0); 64];

    let mut i = 0;
    while i < 64 {
        let tile = Tile::from_index(i);
        table[i] = generate_king_moves(tile);
        i += 1;
    }

    table
}

const fn generate_king_moves(tile: Tile) -> Bitset {
    let square = tile.as_bitset().0;
    let not_a_file = !Board::FILE_MASKS[0].0;
    let not_h_file = !Board::FILE_MASKS[7].0;

    let moves = (square << 8) // north
                | (square >> 8) // south
                | ((square << 1) & not_a_file) // east
                | ((square >> 1) & not_h_file) // west
                | ((square << 9) & not_a_file) // northeast
                | ((square << 7) & not_h_file) // northwest
                | ((square >> 7) & not_a_file) // southeast
                | ((square >> 9) & not_h_file); // southwest

    Bitset(moves)
}

#[cfg(test)]
mod attack_tests {
    use super::*;

    #[test]
    fn test_king_center_attacks() {
        // King on d4 should attack 8 squares: c3, d3, e3, c4, e4, c5, d5, e5
        let index = Tile::D4;
        let attacks = Board::KING_MOVES[index];

        let expected =
            Tile::C3 | Tile::D3 | Tile::E3 | Tile::C4 | Tile::E4 | Tile::C5 | Tile::D5 | Tile::E5;
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_king_corner_attacks() {
        // King on a1 should attack only 3 squares: a2, b1, b2
        let index = Tile::A1;
        let attacks = Board::KING_MOVES[index];

        let expected = Tile::A2 | Tile::B1 | Tile::B2;
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_king_h8_corner_attacks() {
        // King on h8 should attack only 3 squares: g8, g7, h7
        let index = Tile::H8;
        let attacks = Board::KING_MOVES[index];

        let expected = Tile::G8 | Tile::G7 | Tile::H7;
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_king_edge_attacks() {
        // King on d1 (bottom edge) should attack 5 squares: c1, e1, c2, d2, e2
        let index = Tile::D1;
        let attacks = Board::KING_MOVES[index];

        let expected = Tile::C1 | Tile::E1 | Tile::C2 | Tile::D2 | Tile::E2;
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_king_side_edge_attacks() {
        // King on a4 (left edge) should attack 5 squares: a3, a5, b3, b4, b5
        let index = Tile::A4;
        let attacks = Board::KING_MOVES[index];

        let expected = Tile::A3 | Tile::A5 | Tile::B3 | Tile::B4 | Tile::B5;
        assert_eq!(attacks, expected);
    }
}

#[cfg(test)]
mod generation_tests {
    use super::*;
    use crate::game::{
        Move,
        board::Board,
        color::Color,
        piece::{Piece, PieceKind},
    };

    #[test]
    fn test_king_moves_empty_board() {
        // king moves on an empty board should return all attack squares
        let board = Board::empty();
        let king_tile = Tile::D4;
        let moves = board.king_moves(board.to_move, king_tile);
        let expected = Board::KING_MOVES[king_tile];
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_king_moves_blocked_by_own_pieces() {
        let mut board = Board::new();
        Board::initialize();

        // kings pawn for white then black
        board.try_move(Move::new(Tile::E2, Tile::E4)).unwrap();
        board.try_move(Move::new(Tile::E7, Tile::E5)).unwrap();

        assert_eq!(
            Board::KING_MOVES[Tile::E1.as_index()],
            Tile::D1 | Tile::D2 | Tile::E2 | Tile::F2 | Tile::F1
        );
        assert_eq!(
            board.king_moves(board.to_move, Tile::E1),
            Tile::E2.as_bitset()
        );
    }

    #[test]
    fn test_king_captures_enemy_pieces() {
        let mut board = Board::empty();
        board.place_unchecked(
            Piece {
                kind: PieceKind::Pawn,
                color: Color::Black,
            },
            Tile::C4,
        );
        board.place_unchecked(
            Piece {
                kind: PieceKind::Pawn,
                color: Color::Black,
            },
            Tile::E5,
        );

        let king_tile = Tile::D4;
        let moves = board.king_moves(board.to_move, king_tile);

        // king should be able to capture enemy pieces
        let enemy_squares = board.occupancy[Color::Black];
        assert!((moves & enemy_squares) == enemy_squares);
    }
}

#[cfg(test)]
mod castling_tests {
    use super::*;

    #[test]
    fn test_paths() {
        assert_eq!(Board::BLACK_KINGSIDE_PATH, Tile::F8 | Tile::G8);
        assert_eq!(Board::BLACK_QUEENSIDE_PATH, Tile::B8 | Tile::C8 | Tile::D8);
        assert_eq!(Board::WHITE_KINGSIDE_PATH, Tile::F1 | Tile::G1);
        assert_eq!(Board::WHITE_QUEENSIDE_PATH, Tile::B1 | Tile::C1 | Tile::D1);
    }

    #[test]
    fn test_white_king_side_simple() {
        let fen = "8/8/8/8/8/8/PPPPPPP/RNBQK2R w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        Board::initialize();

        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(
            board.castling_moves(board.to_move, king_pos),
            Tile::G1.as_bitset()
        );
    }

    #[test]
    fn test_white_queen_side_simple() {
        let fen = "8/8/8/8/8/8/PPPPPPP/R3KBNR w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        Board::initialize();

        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(
            board.castling_moves(board.to_move, king_pos),
            Tile::C1.as_bitset()
        );
    }

    #[test]
    fn test_white_king_side_blocked() {
        let fen = "8/8/8/8/8/8/PPPPPPP/RNBQKQ1R w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        Board::initialize();

        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(board.king_moves(board.to_move, king_pos), Bitset::empty());
    }

    #[test]
    fn test_white_queen_side_blocked() {
        let fen = "8/8/8/8/8/8/PPPPPPP/R2QKBNR w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        Board::initialize();

        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(board.king_moves(board.to_move, king_pos), Bitset::empty());
    }

    #[test]
    fn test_white_king_side_castle_attacked() {
        Board::initialize();

        // rook targeting destination
        let fen = "8/8/8/8/8/8/PPPPPPrP/RNBQK2R w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(
            board.castling_moves(board.to_move, king_pos),
            Bitset::empty()
        );

        // bishop targeting path
        let fen = "8/8/8/8/8/8/PPPPPPbP/RNBQK2R w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(
            board.castling_moves(board.to_move, king_pos),
            Bitset::empty()
        );

        // queen covering both
        let fen = "8/8/8/8/8/8/PPPPPPqP/RNBQK2R w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(
            board.castling_moves(board.to_move, king_pos),
            Bitset::empty()
        );
    }

    #[test]
    fn test_white_queen_side_castle_attacked() {
        Board::initialize();

        // rook targeting destination
        let fen = "8/8/8/8/8/8/PPrPPPPP/R3KQBNR w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(
            board.castling_moves(board.to_move, king_pos),
            Bitset::empty()
        );

        // bishop targeting path
        let fen = "8/8/8/8/8/8/PPPPbPPP/R3KQBNR w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(
            board.castling_moves(board.to_move, king_pos),
            Bitset::empty()
        );

        // queen covering both
        let fen = "8/8/8/8/8/8/PPqPPPPP/R3KQBNRw KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(
            board.castling_moves(board.to_move, king_pos),
            Bitset::empty()
        );
    }
}

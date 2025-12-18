mod bitset;
mod fen;
mod occupants;
pub mod tile;
pub use fen::ParseBoardError;

use crate::game::{
    color::Color,
    piece::{Piece, PieceKind},
};
use bitset::Bitset;
use std::{
    collections::HashSet,
    ops::{Index, IndexMut},
    sync::LazyLock,
};
use thiserror::Error;

use super::moves::{Move, MoveKind};
use tile::Tile;

static MOVESETS: LazyLock<Vec<Bitset>> = LazyLock::new(|| generate_movesets());

pub(crate) fn generate_movesets() -> Vec<Bitset> {
    let _ = Board::new();
    vec![]
}

#[derive(Error, Copy, Clone, PartialEq, Eq, Debug)]
pub enum MoveError {
    #[error("Move not legal for piece: {0}")]
    IllegalMove(Move),

    #[error("No piece exists at position {0}")]
    NonexistentPiece(Tile),
}

/// Piece-specific sets of positions on the board for both players.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
struct Position([Bitset; 6]);

impl Position {
    /// Standard starting positions.
    pub fn new() -> Self {
        Self([
            // pawns
            Bitset(0xFF << 8) | Bitset(0xFF << 48),
            // knights
            Tile::new(0, 1) | Tile::new(0, 6) | Tile::new(7, 1) | Tile::new(7, 6),
            // bishops
            Tile::new(0, 2) | Tile::new(0, 5) | Tile::new(7, 2) | Tile::new(7, 5),
            // rooks
            Tile::new(0, 0) | Tile::new(0, 7) | Tile::new(7, 0) | Tile::new(7, 7),
            // queens
            Tile::new(0, 3) | Tile::new(7, 3),
            // kings
            Tile::new(0, 4) | Tile::new(7, 4),
        ])
    }

    /// An instance with no pieces placed.
    pub fn empty() -> Self {
        Self([Bitset(0); 6])
    }
}

impl Index<PieceKind> for Position {
    type Output = Bitset;

    fn index(&self, index: PieceKind) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<PieceKind> for Position {
    fn index_mut(&mut self, index: PieceKind) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

impl IntoIterator for Position {
    type Item = (PieceKind, Bitset);
    type IntoIter = std::array::IntoIter<Self::Item, 6>;

    fn into_iter(self) -> Self::IntoIter {
        let values = [
            (PieceKind::Pawn, self[PieceKind::Pawn]),
            (PieceKind::Bishop, self[PieceKind::Bishop]),
            (PieceKind::Rook, self[PieceKind::Rook]),
            (PieceKind::Knight, self[PieceKind::Knight]),
            (PieceKind::Queen, self[PieceKind::Queen]),
            (PieceKind::King, self[PieceKind::King]),
        ];
        values.into_iter()
    }
}

#[derive(Debug, Clone, Hash, Copy, PartialEq, Eq)]
pub struct CastlingRights {
    white_king_side: bool,
    white_queen_side: bool,
    black_king_side: bool,
    black_queen_side: bool,
}

impl CastlingRights {
    pub fn with_all() -> Self {
        Self {
            white_king_side: true,
            white_queen_side: true,
            black_king_side: true,
            black_queen_side: true,
        }
    }

    pub fn with_none() -> Self {
        Self {
            white_king_side: false,
            white_queen_side: false,
            black_king_side: false,
            black_queen_side: false,
        }
    }

    /// Whether no castling rights are present.
    pub fn none(&self) -> bool {
        !self.any()
    }

    /// Whether any castling rights are present.
    pub fn any(&self) -> bool {
        self.white_king_side
            || self.white_queen_side
            || self.black_king_side
            || self.black_queen_side
    }

    /// Whether all castling rights are present.
    pub fn all(&self) -> bool {
        self.white_king_side
            && self.white_queen_side
            && self.black_king_side
            && self.black_queen_side
    }
}

impl Default for CastlingRights {
    fn default() -> Self {
        Self::with_all()
    }
}

/// Core representation of a chess board.
///
/// Implements efficient methods for manipulating and querying the board state.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct Board {
    /// Player's piece-wise positions
    positions: Position,

    /// Player occupancy
    occupancy: [Bitset; 2],

    // "Mailbox" of piece positions
    occupants: [Option<Piece>; 64],

    /// Player to move
    to_move: Color,

    /// Player's castling rights
    castling_rights: CastlingRights,
}

impl Board {
    const MAX_DIM: u8 = 8;
    const MIN_DIM: u8 = 0;
    const RANK_MASKS: [Bitset; Self::MAX_DIM as usize] = [
        Bitset(0x00000000000000FF),
        Bitset(0x000000000000FF00),
        Bitset(0x0000000000FF0000),
        Bitset(0x00000000FF000000),
        Bitset(0x000000FF00000000),
        Bitset(0x0000FF0000000000),
        Bitset(0x00FF000000000000),
        Bitset(0xFF00000000000000),
    ];

    const FILE_MASKS: [Bitset; Self::MAX_DIM as usize] = [
        Bitset(0x0101010101010101),
        Bitset(0x0202020202020202),
        Bitset(0x0303030303030303),
        Bitset(0x0404040404040404),
        Bitset(0x0505050505050505),
        Bitset(0x0606060606060606),
        Bitset(0x0707070707070707),
        Bitset(0x0808080808080808),
    ];

    /// Creates a board in the default starting position.
    pub fn new() -> Self {
        // core bitboards
        let positions = Position::new();

        // populate mailbox from bitboards
        let mut occupants = [None; 64];
        for (kind, bitset) in positions {
            for tile in bitset.tiles() {
                occupants[tile.as_index()] = Some(Piece {
                    kind,
                    color: Color::White,
                });
            }
        }
        for (kind, bitset) in positions {
            for tile in bitset.tiles() {
                occupants[tile.as_index()] = Some(Piece {
                    kind,
                    color: Color::Black,
                });
            }
        }

        // occupancy masks
        let white = Board::RANK_MASKS[0] | Board::RANK_MASKS[1];
        let black = Board::RANK_MASKS[6] | Board::RANK_MASKS[7];

        let board = Self {
            occupants,
            occupancy: [black, white],
            positions: positions,
            to_move: Color::White,
            castling_rights: CastlingRights::default(),
        };

        board
    }

    /// Creates an empty board, i.e. one with no pieces placed and white to move.
    pub fn empty() -> Self {
        Board {
            occupants: [None; 64],
            occupancy: [Bitset(0), Bitset(0)],
            positions: Position::empty(),
            to_move: Color::White,
            castling_rights: CastlingRights::default(),
        }
    }

    /// Traverses all legal game states from the current board position up to
    /// the specified recursion `depth`, returning the number of leaf nodes of
    /// the game tree at that location. Useful for debugging by comparison to
    /// published values.
    pub(crate) fn perft(&self, depth: usize, color: Color) -> usize {
        let mut nodes = 0;
        match depth {
            0 => 1,
            d => {
                for action in self.possible_moves(color) {
                    let mut board = self.clone();
                    board.try_move(action).unwrap();
                    nodes += board.perft(d - 1, !color);
                }
                nodes
            }
        }
    }

    /// The color of the player whose turn it is to move.
    pub fn to_move(&self) -> Color {
        self.to_move
    }

    /// The current castling rights of the players
    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    pub fn possible_captures(&self, color: Color) -> Bitset {
        todo!()
    }

    pub fn possible_moves(&self, color: Color) -> HashSet<Move> {
        HashSet::new()
    }

    pub fn pieces_of(&self, color: Color) -> Vec<(Piece, Tile)> {
        let mut pieces = Vec::with_capacity(64);
        let player_occupancy = self.occupancy[color];
        for (kind, set) in self.positions {
            let player_set = set & player_occupancy;
            for tile in player_set.tiles() {
                pieces.push((Piece { kind, color }, tile))
            }
        }
        pieces
    }

    /// Places `piece` at `tile` without checking legality.
    fn place_unchecked(&mut self, piece: Piece, tile: Tile) {
        let set = &mut self.positions[piece.kind];
        *set |= tile;
        self.occupancy[piece.color] |= tile;
        self.occupants[tile.as_index()] = Some(piece);
    }

    fn generate_movesets(&self) {}

    fn pawn_movesets(&self, color: Color) -> Bitset {
        // match self.occupants[tile.as_index()] {
        //     Some(Piece {
        //         kind: PieceKind::Pawn,
        //         color,
        //     }) => {
        //         // let color = piece.color;
        //         let pawns = self.positions[PieceKind::Pawn];
        //         let not_a = !Board::FILE_MASKS[0];
        //         let not_h = !Board::FILE_MASKS[7];

        //         let attacks = match color {
        //             Color::White => (pawns.0 << 7 & not_h) | (pawns.0 << 9 & not_a),
        //             Color::Black => (pawns.0 >> 7 & not_a) | (pawns.0 >> 9 & not_h),
        //         };

        //         // single and double pushes
        //         let empty = !(self.occupancy[0] & self.occupancy[1]);
        //         // let single = empty & pawns.offset(direction);
        //         // let double = empty & single.offset(direction) & target_row;
        //         // single | double

        //         Some(Bitset(0))
        //     }
        //     _ => None,
        // }
        // let pawns = self.positions[PieceKind::Pawn];

        // single and double pushes
        // let empty = !self.occupancy;
        // let single = empty & pawns.offset(direction);
        // let double = empty & single.offset(direction) & target_row;
        // single | double

        Bitset(0)
    }

    pub fn occupant_moveset(&self, tile: Tile) -> Option<(Piece, Bitset)> {
        // see https://www.chessprogramming.org/Efficient_Generation_of_Sliding_Piece_Attacks#Bitboard_Basics
        if let Some(occupant) = self.occupant(tile) {
            let moveset = match occupant.kind {
                PieceKind::Pawn => self.pawn_movesets(occupant.color),
                PieceKind::Rook => todo!(),
                PieceKind::Bishop => todo!(),
                PieceKind::Knight => todo!(),
                PieceKind::Queen => todo!(),
                PieceKind::King => todo!(),
            };
            Some((occupant, moveset))
        } else {
            None
        }
    }

    /// Returns `true` if `tile` is occupied, and `false` otherwise.
    pub fn occupied(&self, tile: Tile) -> bool {
        !((self.occupancy[0] | self.occupancy[1]) & tile).is_empty()
    }

    /// Returns the piece occupying `tile` if any, and `None` otherwise.
    pub fn occupant(&self, tile: Tile) -> Option<Piece> {
        self.occupants[tile.as_index()]
    }

    /// Tries to make `attempt` on the given board, returning the kind of move (or error) that occurred.
    pub fn try_move(&mut self, attempt: Move) -> Result<MoveKind, MoveError> {
        let mut kind = Err(MoveError::NonexistentPiece(attempt.start()));
        if let Some((occupant, moveset)) = self.occupant_moveset(attempt.start()) {
            if moveset.contains(attempt.stop()) {
                todo!("Implement move logic")
            } else {
                // else move not in moveset
                kind = Err(MoveError::IllegalMove(attempt));
            }
        }
        kind
    }

    /// Returns the winner of the current board, if any. Useful for checking
    /// if a game has ended.
    pub fn winner(&self) -> Option<Color> {
        // TODO: overlap movement sets to determine who is in checkmate
        None
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<&str> for Board {
    type Error = ParseBoardError;

    /// Parses a FEN (Forsyth-Edwards Notation) string into a Board.
    ///
    /// # FEN String Format
    ///
    /// A complete FEN string contains 6 space-separated fields:
    /// 1. **Piece placement** (from white's perspective, rank 8 to rank 1)
    /// 2. **Active color** - "w" (white) or "b" (black)
    /// 3. **Castling availability** - "KQkq" or "-" (K=white kingside, Q=white queenside, k=black kingside, q=black queenside)
    /// 4. **En passant target square** - algebraic notation or "-" if none
    /// 5. **Halfmove clock** - number of halfmoves since last capture or pawn advance (for 50-move rule)
    /// 6. **Fullmove number** - starts at 1, increments after black's move
    ///
    /// ## Piece Placement (Field 1)
    ///
    /// Describes the board from rank 8 (black's back rank) to rank 1 (white's back rank).
    /// - Ranks are separated by "/"
    /// - Each rank is described left to right (file a to file h)
    /// - Pieces are identified by letters:
    ///   - Uppercase = White pieces: K (king), Q (queen), R (rook), B (bishop), N (knight), P (pawn)
    ///   - Lowercase = Black pieces: k, q, r, b, n, p
    /// - Empty squares are indicated by digits 1-8 (count of consecutive empty squares)
    ///
    /// ## Examples
    ///
    /// Starting position:
    /// ```text
    /// rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
    /// ```
    ///
    /// After 1.e4:
    /// ```text
    /// rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1
    /// ```
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        fen::parse(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Board;
    use crate::game::color::Color;
    use crate::{
        game::board::{
            Tile,
            bitset::{Bitset, Offset},
        },
        game::piece::{Piece, PieceKind},
    };
    use std::collections::HashSet;

    fn correct_starting_piece(tile: Tile) -> Option<Piece> {
        let (row, col) = (tile.rank(), tile.file());

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
        let to_offset = Bitset(0);
        assert_eq!(Bitset(0), to_offset.offset(Offset::None));

        let to_offset = Bitset(0xFF);
        assert_eq!(Bitset(0xFF << 8), to_offset.offset(Offset::North));

        let to_offset = Bitset(0xFF00);
        assert_eq!(Bitset(0xFF00 >> 8), to_offset.offset(Offset::South));
        // TODO: handle edge cases
    }

    #[test]
    fn test_placement() {
        let board = Board::new();
        for row in 0..2 {
            for col in 0..Board::MAX_DIM {
                let start = Tile::new(row, col);
                let occupant = board.occupant(start);

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

    #[test]
    fn test_pawn_starting_moveset() {
        let board = Board::new();
        let white = board.pawn_movesets(Color::White);
        assert_eq!(white, Bitset(0x00000000FFFF0000));

        let black = board.pawn_movesets(Color::Black);
        assert_eq!(black, Bitset(0x0000FFFF00000000));
    }
}

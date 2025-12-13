mod bitset;
mod fen;
// mod occupants;
pub use fen::ParseBoardError;

use crate::game::{
    color::Color,
    piece::{Piece, PieceKind},
};
use bitset::{Bitset, Offset};
use std::{collections::HashSet, sync::LazyLock};
use thiserror::Error;

use super::{
    moves::{Move, MoveKind},
    tile::Tile,
};

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

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
struct Placement {
    pawns: Bitset,
    bishops: Bitset,
    rooks: Bitset,
    knights: Bitset,
    queens: Bitset,
    king: Bitset,
}

impl Placement {
    pub fn new(color: Color) -> Self {
        match color {
            Color::White => Self {
                pawns: Bitset(0xFF << 8),
                bishops: Bitset::from(Tile { rank: 0, file: 2 }) | Tile { rank: 0, file: 5 },
                rooks: Bitset::from(Tile { rank: 0, file: 0 }) | Tile { rank: 0, file: 7 },
                knights: Bitset::from(Tile { rank: 0, file: 1 }) | Tile { rank: 0, file: 6 },
                queens: Bitset::from(Tile { rank: 0, file: 3 }),
                king: Bitset::from(Tile { rank: 0, file: 4 }),
            },
            Color::Black => Self {
                pawns: Bitset(0xFF << 48),
                bishops: Bitset::from(Tile { rank: 7, file: 2 }) | Tile { rank: 7, file: 5 },
                rooks: Bitset::from(Tile { rank: 7, file: 0 }) | Tile { rank: 7, file: 7 },
                knights: Bitset::from(Tile { rank: 7, file: 1 }) | Tile { rank: 7, file: 6 },
                queens: Bitset::from(Tile { rank: 7, file: 3 }),
                king: Bitset::from(Tile { rank: 7, file: 4 }),
            },
        }
    }

    pub fn empty() -> Self {
        Self {
            pawns: Bitset(0),
            bishops: Bitset(0),
            rooks: Bitset(0),
            knights: Bitset(0),
            queens: Bitset(0),
            king: Bitset(0),
        }
    }
}

impl IntoIterator for Placement {
    type Item = (PieceKind, Bitset);
    type IntoIter = std::array::IntoIter<Self::Item, 6>;

    fn into_iter(self) -> Self::IntoIter {
        let values = [
            (PieceKind::Pawn, self.pawns),
            (PieceKind::Bishop, self.bishops),
            (PieceKind::Rook, self.rooks),
            (PieceKind::Knight, self.knights),
            (PieceKind::Queen, self.queens),
            (PieceKind::King, self.king),
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

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct Board {
    occupancy: Bitset,
    white: Placement,
    black: Placement,
    to_move: Color,
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
        let white = Placement::new(Color::White);
        let black = Placement::new(Color::Black);
        let occupancy = Bitset::union(white.into_iter().map(|(_, set)| set))
            | Bitset::union(black.into_iter().map(|(_, set)| set));

        let board = Self {
            white,
            black,
            occupancy,
            to_move: Color::White,
            castling_rights: CastlingRights::default(),
        };
        board.generate_movesets();
        board
    }

    /// Creates an empty board, i.e. one with no pieces placed and white to move.
    pub fn empty() -> Self {
        Board {
            white: Placement::empty(),
            black: Placement::empty(),
            occupancy: Bitset(0),
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

    pub fn possible_captures(&self, color: Color) -> HashSet<Move> {
        todo!()
    }

    pub fn possible_moves(&self, color: Color) -> HashSet<Move> {
        HashSet::new()
    }

    pub fn pieces_of(&self, color: Color) -> Vec<(Piece, Tile)> {
        let placement = match color {
            Color::White => self.white,
            Color::Black => self.black,
        };
        let mut pieces = Vec::with_capacity(64);
        for (kind, set) in placement {
            for tile in set.tiles() {
                pieces.push((Piece { kind, color }, tile))
            }
        }
        pieces
    }

    fn set_of(&mut self, piece: Piece) -> &mut Bitset {
        match (piece.kind, piece.color) {
            (PieceKind::Pawn, Color::White) => &mut self.white.pawns,
            (PieceKind::Pawn, Color::Black) => &mut self.black.pawns,
            (PieceKind::Rook, Color::White) => &mut self.white.rooks,
            (PieceKind::Rook, Color::Black) => &mut self.black.rooks,
            (PieceKind::Bishop, Color::White) => &mut self.white.bishops,
            (PieceKind::Bishop, Color::Black) => &mut self.black.bishops,
            (PieceKind::Knight, Color::White) => &mut self.white.knights,
            (PieceKind::Knight, Color::Black) => &mut self.black.knights,
            (PieceKind::Queen, Color::White) => &mut self.white.queens,
            (PieceKind::Queen, Color::Black) => &mut self.black.queens,
            (PieceKind::King, Color::White) => &mut self.white.king,
            (PieceKind::King, Color::Black) => &mut self.black.king,
        }
    }

    /// Places `piece` at `tile` without checking legality.
    fn place_unchecked(&mut self, piece: Piece, tile: Tile) {
        let set = self.set_of(piece);
        *set |= tile;
        self.occupancy |= tile;
    }

    fn index_of(piece: Piece, tile: Tile) -> usize {
        match piece.kind {
            PieceKind::Pawn => todo!(),
            PieceKind::Rook => todo!(),
            PieceKind::Bishop => todo!(),
            PieceKind::Knight => todo!(),
            PieceKind::Queen => todo!(),
            PieceKind::King => todo!(),
        }
    }

    fn generate_movesets(&self) {}

    pub fn pawn_moveset(&self, color: Color, tile: Tile) -> Bitset {
        let (pawns, direction, target_row) = match color {
            Color::White => (self.white.pawns, Offset::North, Board::RANK_MASKS[3]),
            Color::Black => (self.black.pawns, Offset::South, Board::RANK_MASKS[4]),
        };

        let empty = !self.occupancy;

        // single push
        let single = empty & pawns.offset(direction);

        // double push
        let double = empty & single.offset(direction) & target_row;

        single | double
    }

    pub fn pawn_movesets(&self, color: Color) -> Bitset {
        let (pawns, direction, target_row) = match color {
            Color::White => (self.white.pawns, Offset::North, Board::RANK_MASKS[3]),
            Color::Black => (self.black.pawns, Offset::South, Board::RANK_MASKS[4]),
        };

        let empty = !self.occupancy;

        // single push
        let single = empty & pawns.offset(direction);

        // double push
        let double = empty & single.offset(direction) & target_row;

        single | double
    }

    pub fn occupant_moveset(&self, tile: Tile) -> Option<(Piece, HashSet<Tile>)> {
        // see https://www.chessprogramming.org/Efficient_Generation_of_Sliding_Piece_Attacks#Bitboard_Basics
        if let Some(occupant) = self.occupant(tile) {
            let moveset = match occupant.kind {
                PieceKind::Pawn => self.pawn_movesets(occupant.color),
                PieceKind::Rook => todo!(),
                PieceKind::Bishop => todo!(),
                PieceKind::Knight => todo!(),
                PieceKind::Queen => todo!(),
                PieceKind::King => todo!(),
            }
            .tiles()
            .collect();
            Some((occupant, moveset))
        } else {
            None
        }
    }

    fn knight_moveset(&self, color: Color) -> Box<[Bitset; Bitset::MAX_LEN]> {
        fn north_north_east(b: Bitset) -> Bitset {
            (b << 17_u64) & !Board::FILE_MASKS[0]
        }
        fn north_east_east(b: Bitset) -> Bitset {
            (b << 10_u64) & !(Board::FILE_MASKS[0] | Board::FILE_MASKS[1])
        }
        fn south_east_east(b: Bitset) -> Bitset {
            (b >> 6_u64) & !(Board::FILE_MASKS[0] | Board::FILE_MASKS[1])
        }
        // fn south_south_east(b: Bitset) -> Bitset { (b >> 15_u64) & notAFile  }
        // fn north_north_west(b: Bitset) -> Bitset { (b << 15_u64) & notHFile  }
        // fn north_west_west(b: Bitset) -> Bitset { (b <<  6_u64) & notGHFile }
        // fn south_west_west(b: Bitset) -> Bitset { (b >> 10_u64) & notGHFile }
        // fn south_south_west(b: Bitset) -> Bitset { (b >> 17_u64) & notHFile  }

        // fn north_north_east(b: Bitset) -> Bitset { (b & notHFile ) << 17 }
        // fn north_east_east(b: Bitset) -> Bitset { (b & notGHFile) << 10 }
        // fn south_east_east(b: Bitset) -> Bitset { (b & notGHFile) >>  6 }
        // fn south_south_east(b: Bitset) -> Bitset { (b & notHFile ) >> 15 }
        // fn north_north_west(b: Bitset) -> Bitset { (b & notAFile ) << 15 }
        // fn north_west_west(b: Bitset) -> Bitset { (b & notABFile) <<  6 }
        // fn south_west_west(b: Bitset) -> Bitset { (b & notABFile) >> 10 }
        // fn south_south_west(b: Bitset) -> Bitset { (b & notAFile ) >> 17 }
        let board = Bitset(u64::MAX);
        let mut movesets = Box::new([Bitset(0); 64]);
        for (i, tile) in board.tiles().enumerate() {
            movesets[i] = Bitset(0);
        }
        movesets
    }

    /// Returns `true` if `tile` is occupied, and `false` otherwise.
    pub fn occupied(&self, tile: Tile) -> bool {
        !(self.occupancy & tile).is_empty()
    }

    /// Returns the piece occupying` `tile` if any, and `None` otherwise.
    pub fn occupant(&self, tile: Tile) -> Option<Piece> {
        if self.occupied(tile) {
            let index = u64::from(tile);
            let pieces = self.white.into_iter().chain(self.black.into_iter());
            for (pos, (kind, set)) in pieces.enumerate() {
                if (set & index).into() {
                    return Some(Piece {
                        kind,
                        color: Color::from(pos < 6),
                    });
                }
            }
        }
        None
    }

    /// Tries to make `attempt` on the given board, returning the kind of move (or error) that occurred.
    pub fn try_move(&mut self, attempt: Move) -> Result<MoveKind, MoveError> {
        let mut kind = Err(MoveError::NonexistentPiece(attempt.start));
        if let Some((occupant, moveset)) = self.occupant_moveset(attempt.start) {
            if moveset.contains(&attempt.stop) {
                // Toggle origin bit and set destination bit
                let actor_set = self.set_of(occupant);
                actor_set.toggle(attempt.start);
                actor_set.insert(attempt.stop);

                // Check capture
                kind = match self.occupant(attempt.stop) {
                    None => Ok(MoveKind::Quiet),
                    Some(target) => {
                        let target_set = self.set_of(target);
                        target_set.toggle(attempt.stop);
                        Ok(MoveKind::Capture(target))
                    }
                };

                // Toggle origin, set destination for occupancy lookup
                self.occupancy.toggle(attempt.start);
                self.occupancy |= Bitset::from(attempt.stop);

                // TODO: check for other types
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

    fn flatten(&self) -> Vec<Option<Piece>> {
        let f = |i: usize| self.occupant(i.into());
        (0..64).map(f).collect()
    }

    fn moveset(&self, color: Color) -> Bitset {
        Bitset(0)
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
        let to_offset = Bitset(0);
        assert_eq!(Bitset(0), to_offset.offset(Offset::None));

        let to_offset = Bitset(0xFF);
        assert_eq!(Bitset(0xFF << 8), to_offset.offset(Offset::North));

        let to_offset = Bitset(0xFF00);
        assert_eq!(Bitset(0xFF00 >> 8), to_offset.offset(Offset::South));
        // TODO: handle edge cases
    }

    #[test]
    fn test_flatten() {
        let board = Board::new();
        let flattened = board.flatten();

        assert_eq!(flattened.len(), 64);
        assert_eq!(flattened.iter().filter(|p| { p.is_some() }).count(), 32);
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
                    rank: row,
                    file: col,
                };
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

    #[test]
    fn test_knight_starting_moveset() {
        let board = Board::new();
        let white = board.knight_moveset(Color::White);
        for (i, element) in white.iter().enumerate() {
            let position = Tile::from(i);
            for tile in element.tiles() {
                let rank_diff = position.rank.abs_diff(tile.rank);
                let file_diff = position.file.abs_diff(tile.file);
                assert!((rank_diff == 1 && file_diff == 2) | (rank_diff == 2 && file_diff == 1));
            }
        }
    }

    #[test]
    fn test_occupant_moveset() {
        let board = Board::new();

        // e2e4
        let (piece, moveset) = board.occupant_moveset(Tile { rank: 1, file: 4 }).unwrap();
        assert_eq!(
            piece,
            Piece {
                kind: PieceKind::Pawn,
                color: Color::White
            }
        );
        assert_eq!(
            moveset,
            HashSet::from_iter([Tile { rank: 5, file: 4 }, Tile { rank: 6, file: 4 }])
        );
    }
}

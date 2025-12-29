pub mod bitset;
pub mod castling;
mod fen;
mod generation;
mod occupants;
pub mod tile;

pub use castling::{CastlingRights, Right};
pub use fen::ParseBoardError;

use crate::game::{
    board::{
        generation::{bishop, rook},
        tile::Tiles,
    },
    color::Color,
    moves::SpecialMove,
    piece::{Piece, PieceKind},
};
use bitset::Bitset;
use std::ops::{Index, IndexMut};
use thiserror::Error;

use super::moves::{Move, MoveKind};
use tile::Tile;

#[derive(Error, Copy, Clone, PartialEq, Eq, Debug)]
pub enum IllegalMove {
    #[error("Move is not in the moveset for the target piece")]
    NotPossible,

    #[error("Piece {0:?} is not owned by the moving player")]
    UnownedPiece(Piece),

    #[error("No piece exists at position {0}")]
    NonexistentPiece(Tile),

    #[error("Move would put the moving player in check")]
    Check,
}

/// Piece-specific sets of positions on the board for both players.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct Position([Bitset; 6]);

impl Position {
    /// An instance with no pieces placed.
    pub fn empty() -> Self {
        Self([Bitset(0); 6])
    }
}

impl Default for Position {
    fn default() -> Self {
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

/// Core representation of a chess board.
///
/// Implements efficient methods for manipulating and querying the board state.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct Board {
    /// Player's piece-wise positions
    positions: Position,

    /// Player occupancy
    occupancy: [Bitset; 2],

    // "Mailbox" of piece positions
    occupants: [Option<Piece>; 64],

    /// Player to move
    to_move: Color,

    /// Pinned pieces for both players
    pinned: Bitset,

    /// Player's castling rights
    castling_rights: CastlingRights,
}

impl Board {
    pub const MAX_DIM: u8 = 8;
    pub const MIN_DIM: u8 = 0;
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
        Bitset(0x0404040404040404),
        Bitset(0x0808080808080808),
        Bitset(0x1010101010101010),
        Bitset(0x2020202020202020),
        Bitset(0x4040404040404040),
        Bitset(0x8080808080808080),
    ];

    /// Initializes magic bitboard lookup tables for sliding piece move generation.
    /// This must be called before using the board for move generation.
    /// Subsequent calls are no-ops (initialization happens only once).
    pub fn initialize() {
        generation::rook::magic::initialize();
        generation::bishop::magic::initialize();
    }

    /// Creates a board in the default starting position.
    pub fn new() -> Self {
        // core bitboards
        let positions = Position::default();

        // occupancy masks
        let white = Board::RANK_MASKS[0] | Board::RANK_MASKS[1];
        let black = Board::RANK_MASKS[6] | Board::RANK_MASKS[7];

        // populate mailbox from bitboards
        let mut occupants = [None; 64];
        for (kind, bitset) in positions {
            for tile in (bitset & white).tiles() {
                occupants[tile.as_index()] = Some(Piece {
                    kind,
                    color: Color::White,
                });
            }
        }
        for (kind, bitset) in positions {
            for tile in (bitset & black).tiles() {
                occupants[tile.as_index()] = Some(Piece {
                    kind,
                    color: Color::Black,
                });
            }
        }

        Self {
            occupants,
            positions,
            pinned: Bitset(0),
            occupancy: [black, white],
            to_move: Color::White,
            castling_rights: CastlingRights::default(),
        }
    }

    /// Creates an empty board, i.e. one with no pieces placed and white to move.
    pub fn empty() -> Self {
        Board {
            pinned: Bitset(0),
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
    pub fn perft(&self, depth: usize) -> usize {
        let mut nodes = 0;
        match depth {
            0 => 1,
            d => {
                for (_, action) in self.legal_moves(self.to_move) {
                    let mut board = self.clone();
                    board.try_move(action).unwrap();
                    board.to_move = !board.to_move;
                    nodes += board.perft(d - 1);
                }
                nodes
            }
        }
    }

    pub fn positions(&self, kind: PieceKind) -> Bitset {
        self.positions[kind]
    }

    pub fn occupancy(&self, color: Color) -> Bitset {
        self.occupancy[color]
    }

    /// "Mailbox"-style occupancy representation of the board.
    pub fn occupants(&self) -> &[Option<Piece>; 64] {
        &self.occupants
    }

    /// The color of the player whose turn it is to move.
    pub fn to_move(&self) -> Color {
        self.to_move
    }

    /// The current castling rights of the players
    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    /// Yields all pseudo-legal moves for the indicated player `by`. Note that it may not be `by`'s turn.
    fn psuedo_legal_moves<'a>(&'a self, by: Color) -> impl Iterator<Item = (PieceKind, Move)> + 'a {
        enum_iterator::all().flat_map(move |kind| self.psuedo_legal_moves_by_kind(by, kind))
    }

    /// Returns all pseudo-legal moves for pieces of type `kind` for the indicated player `by`.
    /// Note that it may not be `by`'s turn.
    fn psuedo_legal_moves_by_kind(
        &self,
        by: Color,
        kind: PieceKind,
    ) -> impl Iterator<Item = (PieceKind, Move)> {
        (self.positions[kind] & self.occupancy[by])
            .tiles()
            .flat_map(move |source| PieceMoves::new(self, by, kind, source).moves())
    }

    fn pseudo_legal_movesets_from_tile(
        &self,
        by: Color,
        kind: PieceKind,
        tile: Tile,
    ) -> impl Iterator<Item = (PieceKind, Move)> {
        PieceMoves::new(self, by, kind, tile).moves()
    }

    fn filter_legal_moves(mut self, attempt: Move) -> bool {
        match self.try_move(attempt) {
            Ok(_) => true,
            Err(IllegalMove::Check) => false,
            Err(e) => panic!("Engine generated invalid pseudo-legal move {attempt}: {e}"),
        }
    }

    /// Yields all legal moves for the indicated player `by`. Note that it may not be `by`'s turn.
    pub fn legal_moves<'a>(&'a self, by: Color) -> impl Iterator<Item = (PieceKind, Move)> + 'a {
        self.psuedo_legal_moves(by).filter(move |&(_, attempt)| {
            let temp = self.clone();
            temp.filter_legal_moves(attempt)
        })
    }

    /// Yields all legal moves for the indicated player `by`. Note that it may not be `by`'s turn.
    pub fn legal_moves_by_kind<'a>(
        &'a self,
        by: Color,
        kind: PieceKind,
    ) -> impl Iterator<Item = (PieceKind, Move)> + 'a {
        self.psuedo_legal_moves_by_kind(by, kind)
            .filter(move |&(_, attempt)| {
                let temp = self.clone();
                println!("attempt {attempt}");
                temp.filter_legal_moves(attempt)
            })
    }

    pub fn legal_moves_from_tile(
        &self,
        by: Color,
        kind: PieceKind,
        tile: Tile,
    ) -> impl Iterator<Item = (PieceKind, Move)> {
        self.pseudo_legal_movesets_from_tile(by, kind, tile)
            .filter(move |&(_, attempt)| {
                let temp = self.clone();
                temp.filter_legal_moves(attempt)
            })
    }

    /// Places `piece` at `tile` without checking legality.
    fn place_unchecked(&mut self, piece: Piece, tile: Tile) {
        let set = &mut self.positions[piece.kind];
        *set |= tile;
        self.occupancy[piece.color] |= tile;
        self.occupants[tile.as_index()] = Some(piece);
    }

    /// Returns whether the indicated tile is attacked by the indicated player.
    fn is_attacked(&self, tile: Tile, by: Color) -> bool {
        // We model an attacker of a certain type as having already moved to the attacked tile,
        // and determine whether it is able to reach its original position (pseudo-legally, since the ability
        // the return move is "virtual" in the sense that it need only exist in the piece moveset).
        //
        // We do this rather than the more intuitive way of simply checking whether a proposed attack square
        // is in the movesets of the enemy pieces because doing so would require looping through all peices,
        // whereas here we have a fixed number of fast bitwise operations to check attacks.

        // need to flip color to determine if a pawn of `by` could return from `tile`
        if !(Self::PAWN_MOVES[!by][tile] & self.occupancy[by] & self.positions[PieceKind::Pawn])
            .is_empty()
        {
            return true;
        }

        // if a knight could get back to its previous spot from target
        if !(Self::KNIGHT_MOVES[tile] & self.occupancy[by] & self.positions[PieceKind::Knight])
            .is_empty()
        {
            return true;
        }

        // if a rook or queen could get back to one of their occupied spots via straight-line moves
        let all_occupancy = self.occupancy[by] | self.occupancy[!by];
        let straight_attacks = rook::magic::magic_moves(tile, all_occupancy);
        if !(straight_attacks
            & self.occupancy[by]
            & (self.positions[PieceKind::Rook] | self.positions[PieceKind::Queen]))
            .is_empty()
        {
            return true;
        }

        // if a bishop or queen could get back to one of their occupied spots via diagonal moves
        let diagonal_attacks = bishop::magic::magic_moves(tile, all_occupancy);
        if !(diagonal_attacks
            & self.occupancy[by]
            & (self.positions[PieceKind::Bishop] | self.positions[PieceKind::Queen]))
            .is_empty()
        {
            return true;
        }

        // if a king could get back pseudo-legally to its current position
        if !(Self::KING_MOVES[tile] & self.occupancy[by] & self.positions[PieceKind::King])
            .is_empty()
        {
            return true;
        }

        false
    }

    /// Returns `true` if `tile` is occupied, and `false` otherwise.
    pub fn occupied(&self, tile: Tile) -> bool {
        self.occupant(tile).is_some()
    }

    /// Returns the piece occupying `tile` if any, and `None` otherwise.
    pub fn occupant(&self, tile: Tile) -> Option<Piece> {
        self.occupants[tile.as_index()]
    }

    /// Manages castling rights based on `piece` having just made the given move (potentially
    /// with a capture).
    fn manage_castling_rights(&mut self, piece: Piece, attempt: Move, captured: Option<Piece>) {
        const ROOK_STARTS: [Bitset; 2] = [
            Bitset(Tile::A8.as_bitset().0 | Tile::H8.as_bitset().0),
            Bitset(Tile::A1.as_bitset().0 | Tile::H1.as_bitset().0),
        ];

        // maps tiles to the rook caslting rights granted with a rook on the tile
        const ROOK_CASLTING_RIGHTS: [Option<Right>; 64] = generate_rook_castling_rights();
        const fn generate_rook_castling_rights() -> [Option<Right>; 64] {
            let mut rights = [None; 64];
            rights[Tile::A8.as_index()] = Some(Right::BlackQueenSide);
            rights[Tile::H8.as_index()] = Some(Right::BlackKingSide);
            rights[Tile::A1.as_index()] = Some(Right::WhiteQueenSide);
            rights[Tile::H1.as_index()] = Some(Right::WhiteKingSide);
            rights
        }

        if piece.kind == PieceKind::King {
            // any king movement revokes castling rights
            self.castling_rights.unset_color(piece.color);
        } else if piece.kind == PieceKind::Rook {
            // any rook motion from starting position revokes caslting rights on that side
            ROOK_CASLTING_RIGHTS[attempt.start().as_index()]
                .inspect(|&right| self.castling_rights.unset(right));
        } else if let Some(Piece {
            kind: PieceKind::Rook,
            color,
        }) = captured
            // any rook capture
            && !(attempt.stop() & ROOK_STARTS[color]).is_empty()
        {
            if attempt.stop().file() == Board::MAX_DIM - 1 {
                if color == Color::White {
                    self.castling_rights.unset(Right::WhiteKingSide);
                } else {
                    self.castling_rights.unset(Right::BlackKingSide);
                }
            } else if attempt.stop().file() == Board::MIN_DIM {
                if color == Color::White {
                    self.castling_rights.unset(Right::WhiteQueenSide);
                } else {
                    self.castling_rights.unset(Right::BlackQueenSide);
                }
            }
        }
    }

    fn make_move(&mut self, piece: Piece, start: Tile, stop: Tile) -> Option<Piece> {
        // pick up piece
        self.positions[piece.kind] ^= start;
        self.occupancy[piece.color] ^= start;
        self.occupants[start.as_index()] = None;

        // capture other
        let captured = self.occupants[stop].inspect(|c| {
            self.positions[c.kind] ^= stop;
            self.occupancy[c.color] ^= stop;
        });

        // drop piece
        self.positions[piece.kind] |= stop;
        self.occupancy[piece.color] |= stop;
        self.occupants[stop.as_index()] = Some(piece);

        captured
    }

    fn unmake_move(&mut self, piece: Piece, start: Tile, stop: Tile, captured: Option<Piece>) {
        // pick up piece
        self.positions[piece.kind] ^= stop;
        self.occupancy[piece.color] ^= stop;
        self.occupants[stop.as_index()] = None;

        // drop piece
        self.positions[piece.kind] |= start;
        self.occupancy[piece.color] |= start;
        self.occupants[start.as_index()] = Some(piece);

        // uncapture other
        if let Some(captured) = captured {
            self.positions[captured.kind] |= stop;
            self.occupancy[captured.color] |= stop;
            self.occupants[stop.as_index()] = Some(captured);
        }

        // TODO: other state changes
    }

    fn castle(&mut self, color: Color, attempt: Move) {
        debug_assert_eq!(
            attempt.special(),
            Some(SpecialMove::Castle),
            "Attempted a castle on a non-castle attempted move"
        );

        let rank = attempt.start().rank();
        let (rook_start, rook_stop) = if attempt.stop().file() == 6 {
            // Kingside: rook h→f
            (Tile::new(rank, 7), Tile::new(rank, 5))
        } else {
            // Queenside: rook a→d
            (Tile::new(rank, 0), Tile::new(rank, 3))
        };

        debug_assert_eq!(
            self.occupants[rook_start],
            Some(Piece {
                kind: PieceKind::Rook,
                color
            }),
            "Rook not found at start position for castling"
        );

        self.positions[PieceKind::Rook] ^= rook_start;
        self.positions[PieceKind::Rook] |= rook_stop;
        self.occupancy[color] ^= rook_start;

        self.occupancy[color] |= rook_stop;
        self.occupants[rook_start] = None;
        self.occupants[rook_stop] = Some(Piece {
            color,
            kind: PieceKind::Rook,
        });
    }

    /// Tries to make `attempt` on the given board, returning the kind of move (or error) that occurred.
    pub fn try_move(&mut self, attempt: Move) -> Result<MoveKind, IllegalMove> {
        let start = attempt.start();
        let stop = attempt.stop();
        match self.occupants[start.as_index()] {
            Some(
                piece @ Piece {
                    color: player,
                    kind,
                },
            ) if player == self.to_move => {
                // first check that the tile is even in the pseudo-legal moveset
                if self
                    .pseudo_legal_movesets_from_tile(player, kind, start)
                    .any(|(_, m)| m == attempt)
                {
                    // move execution
                    let captured = self.make_move(piece, start, stop);

                    // king must not be left in check
                    if let Some(king) = self.king_of(piece.color)
                        && self.is_attacked(king, !player)
                    {
                        self.unmake_move(piece, start, stop, captured);
                        return Err(IllegalMove::Check);
                    } else if let Some(SpecialMove::Castle) = attempt.special() {
                        self.castle(player, attempt)
                    }

                    self.to_move = !self.to_move;
                    self.manage_castling_rights(piece, attempt, captured);
                    Ok(captured
                        .map(|p| MoveKind::Capture(p.kind))
                        .unwrap_or(MoveKind::Quiet))
                } else {
                    Err(IllegalMove::NotPossible)
                }
            }
            Some(piece) => Err(IllegalMove::UnownedPiece(piece)),
            None => Err(IllegalMove::NonexistentPiece(attempt.stop())),
        }
    }

    fn king_of(&self, color: Color) -> Option<Tile> {
        (self.positions[PieceKind::King] & self.occupancy[color])
            .tiles()
            .next()
    }

    /// Returns the player in check, if any.
    pub fn in_check(&self) -> Option<Color> {
        for color in enum_iterator::all::<Color>() {
            if let Some(tile) = self.king_of(color)
                && self.is_attacked(tile, !color)
            {
                return Some(color);
            }
        }
        None
    }

    /// Returns the winner of the current board, if any. Useful for checking
    /// if a game has ended.
    pub fn winner(&self) -> Option<Color> {
        if let Some(king) = self.king_of(self.to_move)
            && self.is_attacked(king, !self.to_move)
            && self.legal_moves(self.to_move).next().is_none()
        {
            return Some(!self.to_move);
        }

        None
    }

    /// The FEN-string representation of the current board.
    pub fn fen(&self) -> String {
        let mut fen = String::new();

        // Piece placement
        for rank in (0..8).rev() {
            let mut empty_count = 0;
            for file in 0..8 {
                let square = Tile::new(rank, file);
                match self.occupants[square] {
                    Some(piece) => {
                        if empty_count > 0 {
                            fen.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        fen.push(piece.fen_char());
                    }
                    None => empty_count += 1,
                }
            }
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }

        // current move
        fen.push(' ');
        fen.push(
            self.to_move
                .to_string()
                .to_lowercase()
                .chars()
                .next()
                .unwrap(),
        );

        // castling rights
        fen.push(' ');
        if self.castling_rights.none() {
            fen.push('-');
        } else {
            for right in self.castling_rights {
                fen.push(right.into());
            }
        }

        // TODO: placeholder at end
        fen.push_str(" - 0 1");

        fen
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

/// Naive appending of castling flag to all moves. Yields None immediately on non-king piece `kinds`
struct CastlingMoves {
    tiles: Tiles,
    kind: PieceKind,
    source: Tile,
}

impl Iterator for CastlingMoves {
    type Item = (PieceKind, Move);

    fn next(&mut self) -> Option<Self::Item> {
        if self.kind != PieceKind::King {
            return None;
        }

        self.tiles.next().map(|tile| {
            let attempt = Move::new(self.source, tile) | SpecialMove::Castle;
            (self.kind, attempt)
        })
    }
}

/// An interator over the moves of a given piece on a board, used internally by boards when yielding moves.
struct PieceMoves {
    kind: PieceKind,
    tile: Tile,
    moveset: Tiles,
    castling_moves: Tiles,
}

impl PieceMoves {
    fn new(board: &Board, color: Color, kind: PieceKind, tile: Tile) -> Self {
        let bitset = match kind {
            PieceKind::Pawn => board.pawn_moves(tile),
            PieceKind::Knight => board.knight_moves(color, tile),
            PieceKind::Bishop => board.bishop_moves(color, tile),
            PieceKind::Rook => board.rook_moves(color, tile),
            PieceKind::Queen => board.queen_moves(color, tile),
            PieceKind::King => board.king_moves(color, tile),
        };
        let moveset = bitset.tiles();
        let castling_moves = board.castling_moves(color, tile).tiles();
        Self {
            kind,
            tile,
            moveset,
            castling_moves,
        }
    }
}

impl PieceMoves {
    pub fn moves(self) -> impl Iterator<Item = (PieceKind, Move)> {
        self.moveset
            .map(move |tile| (self.kind, Move::new(self.tile, tile)))
            .chain(CastlingMoves {
                kind: self.kind,
                source: self.tile,
                tiles: self.castling_moves,
            })
    }
}

#[cfg(test)]
mod board_tests {
    use super::Board;
    use crate::game::Move;
    use crate::game::board::Right;
    use crate::game::color::Color;
    use crate::game::moves::{MoveKind, SpecialMove};
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
    fn test_try_move() {
        let mut board = Board::new();
        Board::initialize();

        // kings pawn for white then black
        let attempt = Move::new(Tile::E2, Tile::E4);
        board.try_move(attempt).unwrap();

        assert_eq!(board.to_move, Color::Black);
        assert!(!board.occupied(attempt.start()));
        assert_eq!(
            board.occupant(attempt.stop()),
            Some(Piece {
                kind: PieceKind::Pawn,
                color: Color::White
            })
        );
        assert!(board.occupied(attempt.stop()));
        assert_eq!(
            board.occupant(attempt.stop()),
            Some(Piece {
                kind: PieceKind::Pawn,
                color: Color::White
            })
        );
    }

    #[test]
    fn test_white_queen_side_castling() {
        let fen = "8/8/8/8/8/8/PPPPPPPP/R3KBNR w KQkq - 0 1";
        let board = Board::try_from(fen).unwrap();
        Board::initialize();

        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(king_pos, Tile::E1);
        let expected = HashSet::from([
            Move::new(king_pos, Tile::D1),
            Move::new(king_pos, Tile::C1) | SpecialMove::Castle,
        ]);

        let actual = board
            .legal_moves_by_kind(Color::White, PieceKind::King)
            .map(|(_, m)| m)
            .collect::<HashSet<Move>>();
        assert_eq!(actual, expected);

        let actual = board
            .legal_moves(Color::White)
            .filter_map(|(k, m)| if k == PieceKind::King { Some(m) } else { None })
            .collect::<HashSet<Move>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_white_rook_queen_side_capture_removes_castling_right() {
        let fen = "8/8/8/8/8/8/q7/R3K3 b KQkq - 0 1";
        let mut board = Board::try_from(fen).unwrap();
        Board::initialize();

        assert!(board.castling_rights().has(Right::WhiteQueenSide));
        let capture = Move::new(Tile::A2, Tile::A1);
        let result = board.try_move(capture).unwrap();

        assert_eq!(result, MoveKind::Capture(PieceKind::Rook));
        assert!(!board.castling_rights().has(Right::WhiteQueenSide));
    }

    #[test]
    fn test_white_queen_side_castle_rook_movements() {
        let fen = "8/8/8/8/8/8/PPPPPPPP/R3KBNR w KQkq - 0 1";
        let mut board = Board::try_from(fen).unwrap();
        Board::initialize();

        let attempt = Move::new(Tile::E1, Tile::C1) | SpecialMove::Castle;
        board.try_move(attempt).unwrap();

        assert!(board.occupants[Tile::A1].is_none());
        assert_eq!(
            board.occupants[Tile::D1],
            Some(Piece {
                kind: PieceKind::Rook,
                color: Color::White
            })
        );
    }
}

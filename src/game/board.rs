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

macro_rules! assert_board_consistent {
    ($board:expr, $context:expr) => {
        // 1. occupants array matches occupancy bitboards with correct kinds and colors
        for (index, element) in $board.occupants.iter().enumerate() {
            if let Some(occupant) = element {
                let tile = Tile::from_index(index);

                // check  color
                assert!(
                    $board.occupancy[occupant.color].contains(tile),
                    "Context: {}. Occupancy did not contain occupants on tile {} for color {:?}",
                    $context,
                    tile,
                    occupant.color
                );

                // check kind
                assert!(
                    $board.positions[occupant.kind].contains(tile),
                    "Context: {}. No position found on tile {} despite occupant {:?}",
                    $context,
                    tile,
                    occupant
                );
            }
        }

        // 2. every occupancy bit corresponds to an occupant, corroborated by `positions`
        for color in [Color::White, Color::Black] {
            let occupancy = $board.occupancy[color];
            for tile in occupancy.tiles() {
                // check existence of occupant
                if let Some(occupant) = $board.occupants[tile] {
                    // sanity check color of piece
                    assert_eq!(
                        occupant.color,
                        color,
                        "Context: {}. Occupant expected to be of color {} but found {:?} on tile {}",
                        $context,
                        color,
                        occupant,
                        tile
                    );

                    // primary check of positions correspondance
                    assert!(
                        $board.positions[occupant.kind].contains(tile),
                        "Context: {}. Occupant at {} expected to be {:?} but was {:?}",
                        $context,
                        tile,
                        Piece { color, kind: occupant.kind },
                        occupant,
                    );
                } else {
                    panic!("Context: {}. Occupant missing for tile {}", $context, tile);
                }
            }
        }

        // 3. every position bit corresponds to an occupant of the correct color and type
        for kind in enum_iterator::all::<PieceKind>() {
            let positions = $board.positions[kind];
            for tile in positions.tiles() {
                if let Some(occupant) = $board.occupants[tile] {
                    // check kind
                    assert!(
                        occupant.kind == kind,
                        "Context: {}. Positions implied {} on tile {}, but occupant was found to be {:?}",
                        $context,
                        kind,
                        tile,
                        occupant
                    );

                    // check color
                    assert!(
                        $board.occupancy[occupant.color].contains(tile),
                        "Context: {}. Occupancy for piece {:?} on tile {} was expected but not found",
                        $context,
                        occupant,
                        tile
                    );
                } else {
                    panic!(
                        "Context: {}. Occupant of type {} missing for tile {} when implied by positions",
                        $context,
                        kind,
                        tile
                    );
                }
            }
        }
    };
}

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
/// Information needed to unmake a move
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct UndoInfo {
    move_made: Move,
    captured: Option<Piece>,
    castling_rights: CastlingRights,
    castled_rook: Option<(Tile, Tile)>, // (from, to) if this move was castling
}

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

    /// Undo stack for make/unmake operations
    pub undo_stack: Vec<UndoInfo>,
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
            undo_stack: Vec::with_capacity(64),
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
            undo_stack: Vec::new(),
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
                let mut temp = self.clone();
                let mut moves = Vec::new();
                temp.populate_legal_moves(temp.to_move(), &mut moves);
                for action in moves {
                    let mut board = self.clone();
                    board.make_move(action).unwrap();
                    board.to_move = !board.to_move;
                    nodes += board.perft(d - 1);
                }
                nodes
            }
        }
    }

    #[inline]
    pub fn positions(&self, kind: PieceKind) -> Bitset {
        self.positions[kind]
    }

    #[inline]
    pub fn occupancy(&self, color: Color) -> Bitset {
        self.occupancy[color]
    }

    /// "Mailbox"-style occupancy representation of the board.
    pub fn occupants(&self) -> &[Option<Piece>; 64] {
        &self.occupants
    }

    /// The color of the player whose turn it is to move.
    #[inline]
    pub fn to_move(&self) -> Color {
        self.to_move
    }

    /// The current castling rights of the players
    #[inline]
    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    #[inline]
    fn pseudo_legal_movesets_from_tile(
        &self,
        by: Color,
        kind: PieceKind,
        tile: Tile,
    ) -> impl Iterator<Item = (PieceKind, Move)> + use<> {
        let moveset = match kind {
            PieceKind::Pawn => self.pawn_moves(tile),
            PieceKind::Knight => self.knight_moves(by, tile),
            PieceKind::Bishop => self.bishop_moves(by, tile),
            PieceKind::Rook => self.rook_moves(by, tile),
            PieceKind::Queen => self.queen_moves(by, tile),
            PieceKind::King => self.king_moves(by, tile),
        }
        .tiles();
        let caslting_moves = self.castling_moves(by, tile).tiles();

        PieceMoves {
            kind,
            tile,
            moveset,
            castling_moves: caslting_moves,
        }
        .moves()
    }

    #[inline]
    fn filter_legal_moves(&mut self, attempt: Move) -> bool {
        match self.make_move(attempt) {
            Ok(_) => {
                self.unmake_move();
                true
            }
            Err(IllegalMove::Check) => false,
            Err(e) => panic!("Engine generated invalid pseudo-legal move {attempt}: {e}"),
        }
    }

    /// Yields all legal moves for the indicated player `by`. Note that it may not be `by`'s turn.
    #[inline]
    pub fn populate_legal_moves(&mut self, by: Color, moves: &mut Vec<Move>) {
        for kind in enum_iterator::all() {
            let occupancy = self.positions[kind] & self.occupancy[by];
            for tile in occupancy.tiles() {
                let pseudo_legal = self.pseudo_legal_movesets_from_tile(by, kind, tile);
                for (_, attempt) in pseudo_legal {
                    if self.filter_legal_moves(attempt) {
                        moves.push(attempt);
                    }
                }
            }
        }
    }

    #[inline]
    pub fn populate_legal_moves_from_tile(
        &mut self,
        by: Color,
        kind: PieceKind,
        tile: Tile,
        moves: &mut Vec<Move>,
    ) {
        moves.clear();
        for (_, attempt) in self.pseudo_legal_movesets_from_tile(by, kind, tile) {
            if self.filter_legal_moves(attempt) {
                moves.push(attempt);
            }
        }
    }

    /// Places `piece` at `tile` without checking legality.
    fn place_unchecked(&mut self, piece: Piece, tile: Tile) {
        let set = &mut self.positions[piece.kind];
        *set |= tile;
        self.occupancy[piece.color] |= tile;
        self.occupants[tile.as_index()] = Some(piece);
    }

    /// Returns whether the indicated tile is attacked by the indicated player.
    #[inline]
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
    #[inline]
    pub fn occupied(&self, tile: Tile) -> bool {
        self.occupant(tile).is_some()
    }

    /// Returns the piece occupying `tile` if any, and `None` otherwise.
    #[inline]
    pub fn occupant(&self, tile: Tile) -> Option<Piece> {
        self.occupants[tile.as_index()]
    }

    /// Manages castling rights based on `piece` having just made the given move (potentially
    /// with a capture).
    #[inline]
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

    #[inline]
    pub fn make_move(&mut self, attempt: Move) -> Result<MoveKind, IllegalMove> {
        let start = attempt.start();
        let piece = match self.occupants[start] {
            Some(p) if p.color == self.to_move => Ok(p),
            Some(p) => Err(IllegalMove::UnownedPiece(p)),
            None => Err(IllegalMove::NonexistentPiece(start)),
        }?;

        // check moveset
        match self
            .pseudo_legal_movesets_from_tile(self.to_move, piece.kind, start)
            .find(|&(_, m)| m == attempt)
        {
            Some((_, _)) => Ok(()),
            None => Err(IllegalMove::NotPossible),
        }?;

        self.make_move_unchecked(attempt)
    }

    #[inline]
    pub fn make_move_unchecked(&mut self, attempt: Move) -> Result<MoveKind, IllegalMove> {
        #[cfg(debug_assertions)]
        assert_board_consistent!(self, "make_move_unchecked start");

        // piece being moved
        let start = attempt.start();
        let stop = attempt.stop();
        let piece = match self.occupants[start] {
            Some(p) if p.color == self.to_move => Ok(p),
            Some(p) => Err(IllegalMove::UnownedPiece(p)),
            None => Err(IllegalMove::NonexistentPiece(start)),
        }?;

        // save state
        let mut undo = UndoInfo {
            move_made: attempt,
            captured: None,
            castling_rights: self.castling_rights,
            castled_rook: None,
        };

        // 1. pick up piece
        self.positions[piece.kind] &= !start;
        self.occupancy[piece.color] &= !start;
        self.occupants[start] = None;

        #[cfg(debug_assertions)]
        assert_board_consistent!(self, "make_move after piece pickup");

        // 2. capture at destination (if any)
        undo.captured = self.occupants[stop].inspect(|c| {
            self.positions[c.kind] &= !stop;
            self.occupancy[c.color] &= !stop;
            self.occupants[stop] = None;

            #[cfg(debug_assertions)]
            assert_board_consistent!(self, "make_move after piece capture");
        });

        // 3. drop piece at destination
        self.positions[piece.kind] |= stop;
        self.occupancy[piece.color] |= stop;
        self.occupants[stop] = Some(piece);

        #[cfg(debug_assertions)]
        assert_board_consistent!(self, "make_move after piece placement");

        // postprocessing: handle castle
        if let Some(SpecialMove::Castle) = attempt.special() {
            let start_rank = start.rank();
            let stop_rank = stop.rank();

            debug_assert!(
                (start_rank == Board::MIN_DIM || start_rank == Board::MAX_DIM - 1)
                    && stop_rank == start_rank
            );
            debug_assert_eq!(
                piece,
                Piece {
                    kind: PieceKind::King,
                    color: self.to_move
                }
            );

            let (rook_start, rook_stop) = if stop.file() == 6 {
                // kingside
                (Tile::new(start_rank, 7), Tile::new(start_rank, 5))
            } else {
                // queenside
                (Tile::new(start_rank, 0), Tile::new(start_rank, 3))
            };

            undo.castled_rook = Some((rook_start, rook_stop));

            // actually move the rook
            debug_assert!(self.occupancy[self.to_move].contains(rook_start));
            debug_assert!(
                !(self.occupancy[self.to_move] & !self.occupancy[!self.to_move])
                    .contains(rook_stop)
            );
            debug_assert_eq!(
                self.occupants[rook_start],
                Some(Piece {
                    color: self.to_move,
                    kind: PieceKind::Rook,
                })
            );
            debug_assert_eq!(self.occupants[rook_stop], None);

            self.positions[PieceKind::Rook] &= !rook_start;
            self.positions[PieceKind::Rook] |= rook_stop;
            self.occupancy[piece.color] &= !rook_start;
            self.occupancy[piece.color] |= rook_stop;
            self.occupants[rook_start] = None;
            self.occupants[rook_stop] = Some(Piece {
                color: piece.color,
                kind: PieceKind::Rook,
            });

            #[cfg(debug_assertions)]
            assert_board_consistent!(self, "make_move after castle");
        }

        // final postprocessing
        self.manage_castling_rights(piece, attempt, undo.captured);
        self.to_move = !self.to_move;
        let captured = undo.captured;
        self.undo_stack.push(undo);

        // now, have to check legality of board state
        if let Some(king) = self.king_of(piece.color)
            && self.is_attacked(king, !piece.color)
        {
            self.unmake_move();

            #[cfg(debug_assertions)]
            assert_board_consistent!(self, "make_move after undo capture");

            return Err(IllegalMove::Check);
        }

        // move was legal: determine move kind, then push to undo stack and moves vector
        let move_kind = captured
            .map(|p| MoveKind::Capture(p.kind))
            .unwrap_or(MoveKind::Quiet);

        #[cfg(debug_assertions)]
        assert_board_consistent!(self, "make_move end on legal move");

        Ok(move_kind)
    }

    pub fn unmake_move(&mut self) {
        #[cfg(debug_assertions)]
        assert_board_consistent!(self, "unmake_move start");

        let undo = self
            .undo_stack
            .pop()
            .expect("unmake_move called with empty undo_stack");

        let attempt = undo.move_made;
        let start = attempt.start();
        let stop = attempt.stop();

        // get the piece that was moved (it's at stop now)
        let piece = self.occupants[stop].unwrap_or_else(|| {
            panic!(
                "No piece at destination during unmake\n\
                 Move being unmade: {}\n\
                 Start: {} (has piece: {})\n\
                 Stop: {} (has piece: {})\n\
                 Current turn: {:?}\n\
                 Castling rights in undo: {:?}",
                attempt,
                start,
                self.occupants[start.as_index()].is_some(),
                stop,
                self.occupants[stop.as_index()].is_some(),
                self.to_move,
                undo.castling_rights
            )
        });

        // other state
        self.to_move = !self.to_move;
        self.castling_rights = undo.castling_rights;

        // un-castle rook if this was a castling move
        if let Some((rook_start, rook_stop)) = undo.castled_rook {
            self.positions[PieceKind::Rook] &= !rook_stop;
            self.positions[PieceKind::Rook] |= rook_start;
            self.occupancy[piece.color] &= !rook_stop;
            self.occupancy[piece.color] |= rook_start;
            self.occupants[rook_stop] = None;
            self.occupants[rook_start] = Some(Piece {
                color: piece.color,
                kind: PieceKind::Rook,
            });

            #[cfg(debug_assertions)]
            assert_board_consistent!(self, "unmake_move after undo castle");
        }

        // pick up piece from stop
        self.positions[piece.kind] &= !stop;
        self.occupancy[piece.color] &= !stop;
        self.occupants[stop] = None;

        #[cfg(debug_assertions)]
        assert_board_consistent!(self, "unmake_move after undo place piece");

        // Drop piece at start
        self.positions[piece.kind] |= start;
        self.occupancy[piece.color] |= start;
        self.occupants[start] = Some(piece);

        #[cfg(debug_assertions)]
        assert_board_consistent!(self, "unmake_move after undo pick up piece");

        // Restore captured piece (if any)
        if let Some(captured) = undo.captured {
            self.positions[captured.kind] |= stop;
            self.occupancy[captured.color] |= stop;
            self.occupants[stop] = Some(captured);

            #[cfg(debug_assertions)]
            assert_board_consistent!(self, "unmake_move after undo capture");
        }

        #[cfg(debug_assertions)]
        assert_board_consistent!(self, "unmake_move end");
    }

    #[inline]
    fn king_of(&self, color: Color) -> Option<Tile> {
        (self.positions[PieceKind::King] & self.occupancy[color])
            .tiles()
            .next()
    }

    /// Returns the player in check, if any.
    #[inline]
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
    #[inline]
    pub fn winner(&self) -> Option<Color> {
        if let Some(king) = self.king_of(self.to_move)
            && self.is_attacked(king, !self.to_move)
        {
            let mut temp = self.clone();
            let mut moves = Vec::new();
            temp.populate_legal_moves(temp.to_move(), &mut moves);
            if moves.is_empty() {
                return Some(!self.to_move);
            }
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
#[derive(Clone)]
struct PieceMoves {
    kind: PieceKind,
    tile: Tile,
    moveset: Tiles,
    castling_moves: Tiles,
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
        board.make_move(attempt).unwrap();

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
    fn test_make_unmake_simple() {
        let mut board = Board::new();
        Board::initialize();
        let before = board.clone();

        let moves = [
            Move::new(Tile::E2, Tile::E4),
            Move::new(Tile::E7, Tile::E5),
            Move::new(Tile::F1, Tile::A6),
            Move::new(Tile::B7, Tile::A6),
            Move::new(Tile::G1, Tile::F3),
            Move::new(Tile::C7, Tile::C5),
            Move::new(Tile::E1, Tile::G1) | SpecialMove::Castle,
        ];
        for &move_ in &moves {
            board.make_move(move_).unwrap();
        }

        for _ in &moves {
            board.unmake_move();
        }
        assert_eq!(before, board);
    }

    #[test]
    fn test_make_failure() {
        let mut board = Board::new();
        Board::initialize();
        let before = board.clone();

        // setup board
        let moves = [
            Move::new(Tile::E2, Tile::E4),
            Move::new(Tile::E7, Tile::E5),
            Move::new(Tile::F1, Tile::A6),
            Move::new(Tile::B7, Tile::A6),
            Move::new(Tile::G1, Tile::F3),
            Move::new(Tile::C7, Tile::C5),
            Move::new(Tile::E1, Tile::G1) | SpecialMove::Castle,
        ];
        for &move_ in &moves {
            board.make_move(move_).unwrap();
        }
        let middle = board.clone();

        // have failing move
        assert!(board.make_move(Move::new(Tile::G1, Tile::A1)).is_err());
        assert_eq!(board, middle);

        for _ in &moves {
            board.unmake_move();
        }
        assert_eq!(board, before);
    }

    #[test]
    fn test_make_failure_on_castle() {
        let mut board = Board::new();
        Board::initialize();
        let before = board.clone();

        // setup board
        let moves = [Move::new(Tile::E2, Tile::E4), Move::new(Tile::E7, Tile::E5)];
        for &move_ in &moves {
            board.make_move(move_).unwrap();
        }
        let middle = board.clone();

        // have failing move
        assert!(
            board
                .make_move(Move::new(Tile::D1, Tile::G1) | SpecialMove::Castle,)
                .is_err()
        );
        assert_eq!(board, middle);

        for _ in &moves {
            board.unmake_move();
        }
        assert_eq!(board, before);
    }

    #[test]
    fn test_white_queen_side_castling() {
        let fen = "8/8/8/8/8/8/PPPPPPPP/R3KBNR w KQkq - 0 1";
        let mut board = Board::try_from(fen).unwrap();
        Board::initialize();

        let king_pos = board.king_of(Color::White).unwrap();
        assert_eq!(king_pos, Tile::E1);

        let expected = HashSet::from([
            Move::new(king_pos, Tile::D1),
            Move::new(king_pos, Tile::C1) | SpecialMove::Castle,
        ]);

        let mut move_vec = Vec::new();
        board.populate_legal_moves(Color::White, &mut move_vec);
        let actual = move_vec
            .into_iter()
            .filter(|m| {
                board
                    .occupant(m.start())
                    .is_some_and(|p| p.kind == PieceKind::King)
            })
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
        let result = board.make_move(capture).unwrap();

        assert_eq!(result, MoveKind::Capture(PieceKind::Rook));
        assert!(!board.castling_rights().has(Right::WhiteQueenSide));
    }

    #[test]
    fn test_white_queen_side_castle_rook_movements() {
        let fen = "8/8/8/8/8/8/PPPPPPPP/R3KBNR w KQkq - 0 1";
        let mut board = Board::try_from(fen).unwrap();
        Board::initialize();

        let attempt = Move::new(Tile::E1, Tile::C1) | SpecialMove::Castle;
        board.make_move(attempt).unwrap();

        assert!(board.occupants[Tile::A1].is_none());
        assert_eq!(
            board.occupants[Tile::D1],
            Some(Piece {
                kind: PieceKind::Rook,
                color: Color::White
            })
        );
    }

    #[test]
    fn test_king_attacked_moveset() {
        // must dodge or take
        let mut board = Board::try_from("rnb1kQnr/ppp2ppp/8/8/8/8/8/8 b Kkq - 0 1").unwrap();
        Board::initialize();

        assert!(board.is_attacked(Tile::E8, Color::White));

        let pseudo_legal_moves = board
            .pseudo_legal_movesets_from_tile(Color::Black, PieceKind::King, Tile::E8)
            .map(|(k, m)| {
                assert_eq!(k, PieceKind::King);
                m
            })
            .collect::<HashSet<_>>();

        let expected = HashSet::from_iter([
            Move::new(Tile::E8, Tile::D8),
            Move::new(Tile::E8, Tile::F8),
            Move::new(Tile::E8, Tile::E7),
            Move::new(Tile::E8, Tile::D7),
        ]);
        assert_eq!(pseudo_legal_moves, expected);

        let legal_moves = pseudo_legal_moves
            .into_iter()
            .filter_map(|m| {
                if board.filter_legal_moves(m) {
                    Some(m.stop())
                } else {
                    None
                }
            })
            .collect::<Bitset>();

        // take or dodge
        assert_eq!(legal_moves, Tile::F8 | Tile::D7)
    }
}

#[cfg(test)]
mod consistency_tests {
    use super::*;

    #[test]
    fn test_consistency_startpos() {
        let board = Board::new();
        assert_board_consistent!(board, "startpos");
    }

    #[test]
    #[should_panic(
        expected = "Context: positions. Positions implied Pawn on tile a1, but occupant was found to be Piece { kind: Rook, color: White }"
    )]
    fn test_consistency_positions_added() {
        let mut board = Board::new();
        board.positions[PieceKind::Pawn].toggle(Tile::A1);
        assert_board_consistent!(board, "positions");
    }

    #[test]
    #[should_panic(
        expected = "No position found on tile a1 despite occupant Piece { kind: Rook, color: White }"
    )]
    fn test_consistency_positions_removed() {
        let mut board = Board::new();
        board.positions[PieceKind::Rook].toggle(Tile::A1);
        assert_board_consistent!(board, "positions");
    }

    #[test]
    #[should_panic(expected = "Occupancy did not contain occupants on tile d4 for color White")]
    fn test_consistency_occupants_added() {
        let mut board = Board::new();
        board.occupants[Tile::D4] = Some(Piece {
            kind: PieceKind::Pawn,
            color: Color::White,
        });
        assert_board_consistent!(board, "occupants");
    }

    #[test]
    #[should_panic(expected = "Context: occupants. Occupant missing for tile a1")]
    fn test_consistency_occupants_removed() {
        let mut board = Board::new();
        board.occupants[Tile::A1] = None;
        assert_board_consistent!(board, "occupants");
    }

    #[test]
    #[should_panic(expected = "Context: occupants. Occupant missing for tile d4")]
    fn test_occupancy_added() {
        let mut board = Board::new();
        board.occupancy[Color::White].toggle(Tile::D4);
        assert_board_consistent!(board, "occupants");
    }

    #[test]
    #[should_panic(
        expected = "Context: occupants. Occupancy did not contain occupants on tile a1 for color White"
    )]
    fn test_occupancy_removed() {
        let mut board = Board::new();
        board.occupancy[Color::White].toggle(Tile::A1);
        assert_board_consistent!(board, "occupants");
    }
}

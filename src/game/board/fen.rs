/// Fun typestate implementation for parsing a FEN string into a board state.
use crate::game::{
    board::{self, Board, tile::Tile},
    piece::{ParsePieceError, Piece},
};
use std::{
    iter::Peekable,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    str::Chars,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseBoardError {
    #[error("Unrecognized fenstring character `{0}` at position {1}")]
    Unrecognized(char, usize),

    #[error("Unexpected end of string")]
    UnexpectedEnd,

    #[error("Empty string")]
    EmptyString,

    #[error("{0}")]
    InvalidPiece(#[from] ParsePieceError),
}

/// Tries parsing a FEN string into a board.
pub(super) fn parse(source: impl AsRef<str>) -> Result<Board, ParseBoardError> {
    let parser = FenParser::new(source.as_ref());
    parser
        .place_pieces()?
        .check_to_move()?
        .check_castling_rights()
}

/// FEN parsing state on which to condition the parser
trait FenParseState {}

struct PiecePlacement;
impl FenParseState for PiecePlacement {}

struct ToMove;
impl FenParseState for ToMove {}

struct CastlingRights;
impl FenParseState for CastlingRights {}

struct _EnPassant;
impl FenParseState for _EnPassant {}

struct _HalfMoveClock;
impl FenParseState for _HalfMoveClock {}

struct _FullMoveNumber;
impl FenParseState for _FullMoveNumber {}

/// A simple scanner for FEN strings.
struct Scanner<'a> {
    inner: Peekable<Chars<'a>>,
    depth: usize,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            inner: source.trim_start().chars().peekable(),
            depth: 0,
        }
    }

    fn depth(&self) -> usize {
        self.depth
    }

    /// Trims leading whitespace from the scanner.
    fn trim(mut self) -> Self {
        while let Some(c) = self.inner.peek()
            && c.is_whitespace()
        {
            self.inner.next();
            self.depth += 1;
        }
        Scanner {
            inner: self.inner,
            depth: self.depth,
        }
    }
}

impl Iterator for Scanner<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.depth += 1;
        self.inner.next()
    }
}

impl<'a> Deref for Scanner<'a> {
    type Target = Peekable<Chars<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> DerefMut for Scanner<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// FEN string parser, conditioned on a current parsing state
struct FenParser<'a, S: FenParseState> {
    state: PhantomData<S>,
    board: Board,
    source: Scanner<'a>,
}

impl<'a> FenParser<'a, PiecePlacement> {
    fn new(source: &'a str) -> Self {
        let mut board = Board::empty();
        let castling_rights = board::CastlingRights::with_none();
        board.castling_rights = castling_rights;

        Self {
            board,
            source: Scanner::new(source).trim(),
            state: PhantomData,
        }
    }

    fn place_pieces(mut self) -> Result<FenParser<'a, ToMove>, ParseBoardError> {
        let mut rank = 7;
        let mut file = 0;

        while let Some(ch) = self.source.next()
            && (file < Board::MAX_DIM || rank > Board::MIN_DIM)
        {
            let mut spaces = 1; // number of files to step this iteration
            match ch {
                '/' => {
                    // next row
                    rank -= 1;
                    file = 0;
                    spaces = 0;
                }
                new_spaces @ '1'..='8' => spaces = new_spaces.to_digit(10).unwrap() as u8,
                ch => {
                    let piece = Piece::try_from(ch)?;
                    let tile = Tile::new(rank, file);
                    self.board.place_unchecked(piece, tile);
                }
            }
            file += spaces;
        }

        Ok(FenParser {
            board: self.board,
            source: self.source,
            state: PhantomData,
        })
    }
}

impl<'a> FenParser<'a, ToMove> {
    fn check_to_move(mut self) -> Result<FenParser<'a, CastlingRights>, ParseBoardError> {
        let mut source = self.source.trim();
        let to_move = match source.next() {
            Some('w') => Ok(board::Color::White),
            Some('b') => Ok(board::Color::Black),
            Some(ch) => Err(ParseBoardError::Unrecognized(ch, source.depth())),
            None => Err(ParseBoardError::UnexpectedEnd),
        }?;
        self.board.to_move = to_move;
        Ok(FenParser {
            board: self.board,
            source,
            state: PhantomData,
        })
    }
}

impl<'a> FenParser<'a, CastlingRights> {
    fn check_castling_rights(mut self) -> Result<Board, ParseBoardError> {
        let mut source = self.source.trim();
        let mut castling_rights = board::CastlingRights::with_none();
        while let Some(ch) = source.next()
            && !ch.is_whitespace()
        {
            match ch {
                '-' if castling_rights.none() => break,
                'K' => castling_rights.white_king_side = true,
                'Q' => castling_rights.white_queen_side = true,
                'k' => castling_rights.black_king_side = true,
                'q' => castling_rights.black_queen_side = true,
                ch => return Err(ParseBoardError::Unrecognized(ch, source.depth())),
            }
        }
        self.board.castling_rights = castling_rights;

        // TODO: Implement rest of parsing logic
        Ok(self.board)
    }
}

#[cfg(test)]
mod tests {
    use crate::game::color::Color;

    use super::*;
    use anyhow::Result;

    #[test]
    fn test_scanner_collect() -> Result<()> {
        let source = "abc123";
        let scanner = Scanner::new(source);
        let tokens = scanner.collect::<String>();
        assert_eq!(tokens.as_str(), source);
        Ok(())
    }

    #[test]
    fn test_scanner_trim() -> Result<()> {
        let source = "   abc123";
        let scanner = Scanner::new(source);
        let tokens = scanner.trim().collect::<String>();
        assert_eq!(tokens.as_str(), source.trim());
        Ok(())
    }

    #[test]
    fn test_invalid_fenstring() -> Result<()> {
        let fenstring = "invalid fen string";
        let board = Board::try_from(fenstring);
        assert!(board.is_err());
        Ok(())
    }

    #[test]
    fn test_startpos_fenstring() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board = Board::try_from(fenstring)?;
        assert_eq!(board, Board::new());
        Ok(())
    }

    #[test]
    fn test_inprogress_fenstring() -> Result<()> {
        let fenstring = "rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let board = Board::try_from(fenstring)?;

        // moves e2e4, c7c5, g1f3
        assert_eq!(
            board.occupant(Tile::E4).unwrap(),
            Piece::try_from('P').unwrap()
        );
        assert_eq!(
            board.occupant(Tile::C5).unwrap(),
            Piece::try_from('p').unwrap()
        );
        assert_eq!(
            board.occupant(Tile::F3).unwrap(),
            Piece::try_from('N').unwrap()
        );

        Ok(())
    }

    #[test]
    fn test_white_to_move() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board = Board::try_from(fenstring)?;
        assert!(board.to_move() == Color::White);
        Ok(())
    }

    #[test]
    fn test_black_to_move() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
        let board = Board::try_from(fenstring)?;
        assert!(board.to_move() == Color::Black);
        Ok(())
    }

    #[test]
    fn test_all_castle() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board = Board::try_from(fenstring)?;
        assert!(
            board.castling_rights()
                == board::CastlingRights {
                    white_king_side: true,
                    white_queen_side: true,
                    black_king_side: true,
                    black_queen_side: true,
                }
        );
        Ok(())
    }

    #[test]
    fn test_no_castle() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1";
        let board = Board::try_from(fenstring)?;
        assert!(
            board.castling_rights()
                == board::CastlingRights {
                    white_king_side: false,
                    white_queen_side: false,
                    black_king_side: false,
                    black_queen_side: false,
                }
        );
        Ok(())
    }

    #[test]
    fn test_white_castle() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1";
        let board = Board::try_from(fenstring)?;
        assert!(
            board.castling_rights()
                == board::CastlingRights {
                    white_king_side: true,
                    white_queen_side: true,
                    black_king_side: false,
                    black_queen_side: false,
                }
        );
        Ok(())
    }

    #[test]
    fn test_black_castle() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b kq - 0 1";
        let board = Board::try_from(fenstring)?;
        assert!(
            board.castling_rights()
                == board::CastlingRights {
                    white_king_side: false,
                    white_queen_side: false,
                    black_king_side: true,
                    black_queen_side: true,
                }
        );
        Ok(())
    }

    #[test]
    fn test_white_castle_queen_side() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w K - 0 1";
        let board = Board::try_from(fenstring)?;
        assert!(
            board.castling_rights()
                == board::CastlingRights {
                    white_king_side: true,
                    white_queen_side: false,
                    black_king_side: false,
                    black_queen_side: false,
                }
        );
        Ok(())
    }

    #[test]
    fn test_black_castle_queen_side() -> Result<()> {
        let fenstring = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b k - 0 1";
        let board = Board::try_from(fenstring)?;
        assert!(
            board.castling_rights()
                == board::CastlingRights {
                    white_king_side: false,
                    white_queen_side: false,
                    black_king_side: true,
                    black_queen_side: false,
                }
        );
        Ok(())
    }
}

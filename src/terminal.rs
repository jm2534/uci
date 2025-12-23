use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{
    QueueableCommand,
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use inquire::{InquireError, Select};
use std::io::{self, stdout};
use thiserror::Error;
use uci::command::Command;
use uci::{
    engine::{Engine, strategy::minimax::Minimax},
    game::{
        Color,
        board::{Board, tile::Tile},
        moves::Move,
        piece::Piece,
    },
};

use crate::terminal::render::draw_interactive_board;

/// Prompts the user for the color they intend to play as.
/// This temporarily switches back to normal terminal mode for the prompt.
pub fn prompt_color() -> Color {
    let options: Vec<&str> = vec!["White", "Black", "Random"];
    let ans: Result<&str, InquireError> = Select::new("Choose your color:", options).prompt();
    ans.unwrap().parse::<Color>().expect("Color parsing error")
}

#[derive(Debug)]
pub enum SelectionResult {
    Selected(Piece),
    Deselected,
    MoveMade(Move),
    InvalidPiece,
    EmptySquare,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    MoveCursor(Direction),
    Select,
    Quit,
}

#[derive(Debug, Error)]
pub enum ActionParseError {
    #[error("Unrecognized event: {0:?}")]
    Unrecognized(Event),
}

impl TryFrom<Event> for Action {
    type Error = ActionParseError;

    fn try_from(value: Event) -> Result<Self, Self::Error> {
        match value {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Esc, ..
            }) => Ok(Action::Quit),

            Event::Key(KeyEvent {
                code: KeyCode::Up, ..
            }) => Ok(Action::MoveCursor(Direction::Up)),

            Event::Key(KeyEvent {
                code: KeyCode::Down,
                ..
            }) => Ok(Action::MoveCursor(Direction::Down)),

            Event::Key(KeyEvent {
                code: KeyCode::Left,
                ..
            }) => Ok(Action::MoveCursor(Direction::Left)),

            Event::Key(KeyEvent {
                code: KeyCode::Right,
                ..
            }) => Ok(Action::MoveCursor(Direction::Right)),

            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                ..
            }) => Ok(Action::Select),

            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => Ok(Action::Quit),
            e => Err(ActionParseError::Unrecognized(e)),
        }
    }
}

pub struct Game {
    state: InteractiveState,
    engine: Engine<Minimax>,
}

impl Game {
    pub fn new(player: Color) -> Self {
        let engine = Engine::default();
        let state = InteractiveState::new(player);
        Self { state, engine }
    }

    pub fn finished(&self) -> bool {
        !self.state.active
    }

    pub fn handle(&mut self, action: Action) {
        match action {
            Action::MoveCursor(direction) => {
                self.state.move_cursor(direction);
            }
            Action::Select => {
                match self.state.toggle_selection(&self.engine.board) {
                    SelectionResult::Selected(_) => {
                        // Successfully selected a piece - visual feedback already handled by render
                    }
                    SelectionResult::Deselected => {
                        // Deselected current piece - visual feedback already handled by render
                    }
                    SelectionResult::MoveMade(chess_move) => {
                        // Attempt to make the move through the engine
                        match self.engine.handle(Command::Position(vec![chess_move])) {
                            Ok(_) => {
                                self.state.clear_selection();
                                if self.engine.finished() {
                                    self.state.active = false;
                                }
                            }
                            Err(e) => {
                                // Move was invalid - show error and maintain selection
                                // For now, just clear selection; in a full implementation,
                                // you might want to show an error message
                                self.state.clear_selection();
                                eprintln!("Invalid move: {}", e);
                            }
                        }
                    }
                    SelectionResult::InvalidPiece => {
                        // Tried to select opponent's piece or invalid piece
                        // Visual feedback: maybe flash red or show message
                    }
                    SelectionResult::EmptySquare => {
                        // Tried to select empty square
                        // Visual feedback: maybe show message
                    }
                }
            }
            Action::Quit => {
                self.state.active = false;
            }
        }
    }

    pub fn draw(&self) -> io::Result<()> {
        render::draw_interactive_board(&self.engine.board, &self.state)
    }
}

/// Represents the current state of the interactive UI
#[derive(Debug, Clone)]
pub struct InteractiveState {
    /// Current cursor position on the board
    pub cursor_pos: Tile,

    /// Currently selected piece position, if any
    pub selected_pos: Option<Tile>,

    /// Available moves for the selected piece
    pub available_moves: Vec<Move>,

    /// Player color (for board orientation)
    pub player: Color,

    /// Whether the game is active
    pub active: bool,
}

impl InteractiveState {
    pub fn new(player: Color) -> Self {
        Self {
            cursor_pos: match player {
                Color::White => Tile::A1,
                Color::Black => Tile::H8,
            },
            selected_pos: None,
            available_moves: Vec::new(),
            player,
            active: true,
        }
    }

    /// Move cursor in the specified direction
    pub fn move_cursor(&mut self, direction: Direction) {
        let (rank, file) = (self.cursor_pos.rank(), self.cursor_pos.file());

        let new_pos = match direction {
            Direction::Up => {
                let new_rank = if self.player == Color::White {
                    (rank.saturating_add(1)).min(7u8)
                } else {
                    rank.saturating_sub(1)
                };
                Tile::new(new_rank, file)
            }
            Direction::Down => {
                let new_rank = if self.player == Color::White {
                    rank.saturating_sub(1)
                } else {
                    (rank.saturating_add(1)).min(7u8)
                };
                Tile::new(new_rank, file)
            }
            Direction::Left => {
                let new_file = if self.player == Color::White {
                    file.saturating_sub(1)
                } else {
                    (file.saturating_add(1)).min(7u8)
                };
                Tile::new(rank, new_file)
            }
            Direction::Right => {
                let new_file = if self.player == Color::White {
                    (file.saturating_add(1)).min(7u8)
                } else {
                    file.saturating_sub(1)
                };
                Tile::new(rank, new_file)
            }
        };

        self.cursor_pos = new_pos;
    }

    /// Select or deselect piece at cursor position
    pub fn toggle_selection(&mut self, board: &Board) -> SelectionResult {
        if let Some(selected) = self.selected_pos {
            // If we have a selection, either move or deselect
            if selected == self.cursor_pos {
                // deselect current piece
                self.selected_pos = None;
                self.available_moves.clear();
                SelectionResult::Deselected
            } else {
                // try to make a move
                let attempted_move = Move::new(selected, self.cursor_pos);
                if self.available_moves.contains(&attempted_move) {
                    self.selected_pos = None;
                    self.available_moves.clear();
                    SelectionResult::MoveMade(attempted_move)
                } else {
                    // try to select piece at cursor
                    self.try_select_piece(board)
                }
            }
        } else {
            // No current selection, try to select piece at cursor
            self.try_select_piece(board)
        }
    }

    fn try_select_piece(&mut self, board: &Board) -> SelectionResult {
        if let Some(piece) = board.occupant(self.cursor_pos) {
            if piece.color == board.to_move() {
                // Valid piece to select
                self.selected_pos = Some(self.cursor_pos);
                self.available_moves = board
                    .moves_from_tile(self.cursor_pos, piece.kind)
                    .tiles()
                    .map(|to| Move::new(self.cursor_pos, to))
                    .filter(|mv| board.possible_moves().any(|legal_move| legal_move == *mv))
                    .collect();
                SelectionResult::Selected(piece)
            } else {
                SelectionResult::InvalidPiece
            }
        } else {
            SelectionResult::EmptySquare
        }
    }

    /// Clear current selection
    pub fn clear_selection(&mut self) {
        self.selected_pos = None;
        self.available_moves.clear();
    }

    /// Check if a tile is highlighted as an available move
    pub fn is_available_move(&self, tile: Tile) -> bool {
        self.available_moves.iter().any(|mv| mv.stop() == tile)
    }
}

/// Enhanced rendering with highlighting support
mod render {
    use crossterm::style;

    use super::*;
    use std::io::Write;
    use uci::game::{
        Color,
        board::Board,
        piece::{Piece, PieceKind},
    };

    /// Represents different visual states for board squares
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum SquareState {
        Normal,
        Cursor,
        Selected,
        AvailableMove,
        CursorOnSelected,
        CursorOnAvailableMove,
    }

    /// Clears the terminal screen
    pub fn clear() -> io::Result<()> {
        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))
    }

    /// Draws the board with interactive highlighting
    pub fn draw_interactive_board(board: &Board, state: &InteractiveState) -> io::Result<()> {
        clear()?;

        let mut stdout = stdout();

        // title and instructions
        stdout.queue(MoveTo(0, 0))?;
        stdout.queue(Print("Interactive Chess Engine"))?;
        stdout.queue(MoveTo(0, 1))?;
        stdout.queue(Print(
            "Arrow keys: move cursor | Enter/Space: select/move | Q/Esc: quit",
        ))?;
        stdout.queue(MoveTo(0, 2))?;
        stdout.queue(Print(format!("Turn: {:?}", board.to_move())))?;

        // top-left corner of board display (note: not board itself)
        let start_row = 5;
        let start_col = 4;

        // column labels, shifted to accomodate row labels and board edge
        stdout.queue(MoveTo(start_col + 3, start_row - 1))?;
        let file_labels = if state.player == Color::White {
            "a b c d e f g h"
        } else {
            "h g f e d c b a"
        };
        stdout.queue(Print(file_labels))?;

        // top border
        stdout.queue(MoveTo(start_col, start_row))?;
        stdout.queue(Print("  ┌─────────────────┐"))?;

        // board rows
        let board_array = board.occupants();
        let mut visual_board = [None; 64];
        visual_board.copy_from_slice(board_array);
        if state.player == Color::White {
            // rendering board top-down, so pieces (bottom-up) need to be reversed for white's perspective
            visual_board.reverse();
        }

        for (visual_row, pieces) in visual_board.chunks(8).enumerate() {
            let board_row = start_row + 1 + visual_row as u16;

            // row label
            stdout.queue(MoveTo(start_col, board_row))?;
            let rank_label = match state.player {
                Color::White => 8 - visual_row,
                Color::Black => visual_row + 1,
            };
            stdout.queue(Print(format!("{} │", rank_label)))?;

            // pieces in this row
            let pieces_to_draw = if state.player == Color::White {
                pieces.to_vec()
            } else {
                let mut rev = pieces.to_vec();
                rev.reverse();
                rev
            };

            // TODO: fix
            for (file_idx, piece) in pieces_to_draw.iter().enumerate() {
                let tile_rank = match state.player {
                    Color::White => (7 - visual_row) as u8,
                    Color::Black => visual_row as u8,
                };
                let tile_file = match state.player {
                    Color::White => file_idx as u8,
                    Color::Black => (7 - file_idx) as u8,
                };
                let current_tile = Tile::new(tile_rank, tile_file);

                // Determine square state
                let square_state = get_square_state(current_tile, state);

                // Set colors based on state
                apply_square_colors(&mut stdout, square_state)?;

                // Draw piece or empty square
                let piece_char = piece.map_or('\u{00B7}', to_unicode);
                stdout.queue(Print(format!("{} ", piece_char)))?;

                stdout.queue(ResetColor)?;
            }

            // right border
            stdout.queue(Print("│"))?;
        }

        // bottom border
        stdout.queue(MoveTo(start_col, start_row + 9))?;
        stdout.queue(Print("  └─────────────────┘"))?;

        // status information
        stdout.queue(MoveTo(0, start_row + 11))?;
        if let Some(selected) = state.selected_pos {
            stdout.queue(Print(format!(
                "Selected: {} ({} moves available)",
                selected,
                state.available_moves.len()
            )))?;
        } else if let Some(piece) = board.occupants()[state.cursor_pos] {
            stdout.queue(Print(format!("Piece: {} {}", piece.color, piece.kind)))?;
        } else {
            stdout.queue(Print("No piece selected"))?;
        }

        stdout.queue(MoveTo(0, start_row + 12))?;
        stdout.queue(Print(format!("Cursor: {}", state.cursor_pos)))?;

        stdout.flush()?;
        Ok(())
    }

    fn get_square_state(tile: Tile, state: &InteractiveState) -> SquareState {
        let is_cursor = tile == state.cursor_pos;
        let is_selected = state.selected_pos == Some(tile);
        let is_available = state.is_available_move(tile);

        match (is_cursor, is_selected, is_available) {
            (true, true, _) => SquareState::CursorOnSelected,
            (true, false, true) => SquareState::CursorOnAvailableMove,
            (true, false, false) => SquareState::Cursor,
            (false, true, _) => SquareState::Selected,
            (false, false, true) => SquareState::AvailableMove,
            (false, false, false) => SquareState::Normal,
        }
    }

    fn apply_square_colors(stdout: &mut io::Stdout, state: SquareState) -> io::Result<()> {
        match state {
            SquareState::Normal => {
                stdout.queue(ResetColor)?;
            }
            SquareState::Cursor => {
                stdout.queue(SetBackgroundColor(style::Color::Blue))?;
                stdout.queue(SetForegroundColor(style::Color::White))?;
            }
            SquareState::Selected => {
                stdout.queue(SetBackgroundColor(style::Color::Yellow))?;
                stdout.queue(SetForegroundColor(style::Color::Black))?;
            }
            SquareState::AvailableMove => {
                stdout.queue(SetBackgroundColor(style::Color::Green))?;
                stdout.queue(SetForegroundColor(style::Color::White))?;
            }
            SquareState::CursorOnSelected => {
                stdout.queue(SetBackgroundColor(style::Color::Magenta))?;
                stdout.queue(SetForegroundColor(style::Color::White))?;
            }
            SquareState::CursorOnAvailableMove => {
                stdout.queue(SetBackgroundColor(style::Color::Cyan))?;
                stdout.queue(SetForegroundColor(style::Color::Black))?;
            }
        }
        Ok(())
    }

    /// Returns the unicode representation of a piece
    fn to_unicode(piece: Piece) -> char {
        // NOTE: when comparing these mappings to their Unicode names, Unicode
        // "black" vs. "white" indicates fill, not piece color as used below.
        let unicode_hex = match (piece.kind, piece.color) {
            (PieceKind::King, Color::Black) => 0x2654,
            (PieceKind::King, Color::White) => 0x265A,
            (PieceKind::Queen, Color::Black) => 0x2655,
            (PieceKind::Queen, Color::White) => 0x265B,
            (PieceKind::Rook, Color::Black) => 0x2656,
            (PieceKind::Rook, Color::White) => 0x265C,
            (PieceKind::Knight, Color::Black) => 0x2658,
            (PieceKind::Knight, Color::White) => 0x265E,
            (PieceKind::Bishop, Color::Black) => 0x2657,
            (PieceKind::Bishop, Color::White) => 0x265D,
            (PieceKind::Pawn, Color::Black) => 0x2659,
            (PieceKind::Pawn, Color::White) => 0x265F,
        };
        char::from_u32(unicode_hex).unwrap_or('?')
    }
}

/// Initialize terminal for interactive use
pub fn init_terminal() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(stdout(), Hide)?;
    Ok(())
}

/// Restore terminal to normal state
pub fn cleanup_terminal() -> io::Result<()> {
    execute!(stdout(), Show, ResetColor)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

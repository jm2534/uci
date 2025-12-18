use inquire::{InquireError, Select};
use std::{
    io::{Write, stdin, stdout},
    thread, time,
};
use uci::{
    command::Command,
    engine::Engine,
    game::{Color, Move, board::Board},
};

fn draw(board: &Board, moves: &[Move], player_color: Color) {
    clear();
    render::draw(board);
    println!("\n{:?}", moves);

    if board.to_move() != player_color {
        let duration = time::Duration::from_millis(500);
        thread::sleep(duration);
    }
}

pub fn clear() {
    print!("\x1B[2J");
}

fn main() {
    let mut engine = Engine::default();
    let mut moves = Vec::new();
    while !engine.finished() {
        draw(&engine.board, &moves, Color::White);
        let mut input = terminal::prompt_move().unwrap();
        moves.push(input);
        while let Err(_) = engine.handle(Command::Position(moves.clone())) {
            moves.pop();
            input = terminal::prompt_move().unwrap();
            moves.push(input);
        }
    }
}

mod terminal {
    use inquire::{InquireError, Select};
    use std::io::{Write, stdin, stdout};
    use uci::game::{Color, Move};

    /// Prompts the user for the color they intend to play as.
    pub fn prompt_color() -> Color {
        let options: Vec<&str> = vec!["White", "Black", "Random"];
        let ans: Result<&str, InquireError> = Select::new("Choose your color:", options).prompt();
        ans.unwrap().parse::<Color>().expect("Color parsing error")
    }

    pub fn prompt_move() -> Option<Move> {
        let mut input = String::new();
        stdout().flush().expect("Could not flush");
        stdin().read_line(&mut input).expect("Could not parse");
        Move::try_from(input.as_str()).ok()
    }
}

mod render {
    use std::io::Write;
    use std::{char, io};
    use uci::game::{
        Color,
        board::Board,
        piece::{Piece, PieceKind},
    };

    struct Line {
        content: String,
        row: u8,
    }

    impl Line {
        fn convert(entity: &Option<Piece>) -> char {
            match *entity {
                Some(piece) => to_unicode(piece),
                None => char::from_u32(0x00B7).unwrap(),
            }
        }

        fn new(pieces: &[Option<Piece>], row: u8) -> Self {
            let mut content: String = pieces
                .iter()
                .map(Line::convert)
                .map(|c| c.to_string() + " ")
                .collect();
            content.insert_str(0, &format!("{} |", row));
            content.push('|');
            Self { content, row }
        }
    }

    /// Returns the unicode representation of `piece`, or a placeholder symbol
    /// if none is found.
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

    /// Locks standard output and prints the state of the current `Game` from
    /// the perspective of the player of color `color`.
    pub fn draw(board: &Board) {
        let color = board.to_move();
        let mut board: [Option<Piece>; 64] = board.occupants().to_owned();
        if color == Color::White {
            // initial reverse to handle the fact that white's rows come first
            // in the iterator but need to be drawn last from white's view
            board.reverse();
        }

        // Top/bottom lines
        let mut border_line = String::from_utf8(vec![b'-'; 16]).unwrap();
        border_line.insert_str(0, "   ");

        let stdout = io::stdout();
        let mut lock = stdout.lock();
        writeln!(lock, "{}", border_line).unwrap();

        let chunk_size: usize = (Board::MAX_DIM - Board::MIN_DIM).into();
        let rows = board.chunks(chunk_size);
        for (row, pieces) in rows.enumerate() {
            // Parse row according to player's perspective as given by color
            let line = match color {
                Color::Black => Line::new(pieces, (row + 1) as u8),
                Color::White => {
                    let mut pieces = pieces.to_vec();
                    pieces.reverse();

                    let row = Board::MAX_DIM - (row as u8);
                    Line::new(&pieces, row)
                }
            };

            writeln!(lock, "{}", line.content).unwrap();
        }
        writeln!(lock, "{}", border_line).unwrap();
        let alpha_line: String = ('a'..='h').map(|c| c.to_string() + " ").collect();
        writeln!(lock, "   {}", alpha_line).unwrap();
    }
}

use std::io;
mod terminal;
use uci::game::board::Board;

fn main() -> io::Result<()> {
    #[cfg(feature = "logging")]
    {
        use tracing_appender::{non_blocking, rolling};
        use tracing_subscriber::{self, EnvFilter};
        let file_appender = rolling::daily("logs", "termchess.log");
        let (file_writer, _guard) = non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(file_writer)
            .with_ansi(false)
            .init();
    }

    terminal::render::clear()?;
    let color = terminal::prompt_color();
    terminal::init_terminal()?;
    let mut game = terminal::Game::new(color);
    Board::initialize();

    // main thread handles user input and renders, spawned threads run engine
    // TODO: CPU moves via engine.step()
    while !game.finished() {
        game.draw()?;
        let action;
        loop {
            if let Ok(result) = crossterm::event::read()?.try_into() {
                action = result;
                break;
            }
        }
        game.handle(action);
    }

    terminal::cleanup_terminal()?;
    println!("\nGame finished!");
    Ok(())
}

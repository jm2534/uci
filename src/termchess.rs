use std::io;

mod terminal;

fn main() -> io::Result<()> {
    let color = terminal::prompt_color();
    terminal::init_terminal()?;
    let mut game = terminal::Game::new(color);

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

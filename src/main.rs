use anyhow::Result;
use std::io::{self, BufRead};
use uci::{
    command::{Command, CommandError},
    engine::Engine,
};

fn main() -> Result<()> {
    let mut command_buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    let mut engine = Engine::default();
    loop {
        handle.read_line(&mut command_buffer)?;
        if !command_buffer.trim().is_empty() {
            match Command::try_from(&command_buffer[..]) {
                Ok(Command::Quit) => break,
                Ok(cmd) => match engine.handle(cmd) {
                    Ok(None) => (),
                    Ok(Some(response)) => println!("{response}"),
                    Err(e) => eprintln!("Illegal move: {e}"),
                },
                Err(CommandError::UnrecognizedCommand(c))
                | Err(CommandError::UnrecognizedArgument(c)) => eprintln!("Unknown command: {c}"),
            };
        }
        command_buffer.clear();
    }
    Ok(())
}

use anyhow::Result;
use std::io::{self, BufRead};
use uci::command::{Command, CommandError};

fn main() -> Result<()> {
    let mut command_buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        handle.read_line(&mut command_buffer)?;
        let command = Command::try_from(&command_buffer[..]);
        if let Ok(Command::Uci) = command {
            //
        }
    }
    Ok(())
}

mod kb_switcher;
use clap::Parser;
use kb_switcher::KbSwitcherCmd;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let command = KbSwitcherCmd::parse();
    command.process()?;
    Ok(())
}

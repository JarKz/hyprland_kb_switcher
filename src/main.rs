mod kb_switcher;
use kb_switcher::KbSwitcherCmd;

use clap::Parser;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let command = KbSwitcherCmd::parse();
    command.process()?;
    Ok(())
}

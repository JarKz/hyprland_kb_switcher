mod kb_switcher;
use kb_switcher::KbSwitcherCmd;

use clap::Parser;

fn main() -> std::io::Result<()> {
    let command = KbSwitcherCmd::parse();
    command.process()
}

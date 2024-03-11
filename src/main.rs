mod kb_switcher;
use kb_switcher::KbSwitcherCmd;

use clap::Parser;
use tokio;

#[tokio::main]
async fn main() -> hyprland::Result<()> {
    let command = KbSwitcherCmd::parse();
    command.process().await
}

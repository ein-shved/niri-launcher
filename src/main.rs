use niri_launcher::{Launcher, Parser, error::Result};

fn main() -> Result<()> {
    let args = Launcher::parse();

    args.run()
}

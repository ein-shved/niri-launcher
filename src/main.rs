use std::io;

use niri_launcher::{Launcher, Parser};

fn main() -> io::Result<()> {
    let args = Launcher::parse();

    args.run()
}

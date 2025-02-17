use niri_launcher::{Launcher, Parser};

fn main() {
    let args = Launcher::parse();

    args.run();
}

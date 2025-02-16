use niri_launcher::{Args, Parser};

fn main() {
    let args = Args::parse();

    args.run();
}

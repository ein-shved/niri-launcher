//!
//! Basic utility types. The [Args] is core type which both handles command line
//! arguments and executes process. The main argument is
//! [command](Args::command). The command specify which operation to perform.
//!
//! Each [command's](Command) emum type implements [Parser] and [Runner] traits
//! to parse arguments from one side and to perform action from another.
//!
#![warn(missing_docs)]

use clap::Subcommand;
pub use clap::{Parser, ValueEnum};
use niri_multi_socket::MultiSocket;
use std::path::PathBuf;

mod niri_multi_socket;

/// Top-level arguments structure
#[derive(Parser, Debug)]
#[command(
    author = "Yury Shvedov (github:ein-shved)",
    version = "0.1",
    about = "Niri launcher",
    long_about = "Simple utility to smartly launch several tools withing niri."
)]
pub struct Args {
    /// The procedure to run
    #[command(subcommand)]
    command: Command,

    /// Optional path to niri socket
    #[arg(short, long, help = "Path to niri socket")]
    path: Option<PathBuf>,
}

/// The list of supported commands
#[derive(Subcommand, Debug, Clone)]
#[command(about, long_about)]
pub enum Command {
    /// Check niri availability.
    ///
    /// Exits with success if niri is available and panics if niri is
    /// unavailable.
    #[command(about, long_about)]
    Test(TestSocket),
}

/// The trait for subcommand
pub trait Runner {
    /// The [Args] will create socket for niri and pass it here
    fn run(self, socket: MultiSocket);
}

impl Args {
    /// Run chosen subcommand
    pub fn run(self) {
        let socket = if let Some(path) = self.path {
            MultiSocket::connect_to(&path)
        } else {
            MultiSocket::connect().unwrap()
        };
        match self.command {
            Command::Test(cmd) => cmd.run(socket),
        }
    }
}

/// Check niri availability.
#[derive(Parser, Debug, Clone)]
pub struct TestSocket {}

impl Runner for TestSocket {
    fn run(self, socket: MultiSocket) {
        // Will panic if niri socket is unavailable
        socket.get_socket().unwrap();
    }
}

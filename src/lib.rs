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
use niri_ipc::{Request, Response};
use niri_multi_socket::MultiSocket;
use regex;
use std::{ffi::OsString, io, os::unix::process::CommandExt, path::PathBuf};

mod kitty;
mod niri_multi_socket;

/// Top-level arguments structure
#[derive(Parser, Debug)]
#[command(
    author = "Yury Shvedov (github:ein-shved)",
    version = "0.1",
    about = "Niri launcher",
    long_about = "Simple utility to smartly launch several tools withing niri."
)]
pub struct Launcher {
    /// The procedure to run
    #[command(subcommand)]
    command: Command,

    /// Optional path to niri socket
    #[arg(short, long, help = "Path to niri socket")]
    path: Option<PathBuf>,

    /// Optional template of kitty socket
    ///
    /// Will accept environment variables in view `${ENV}` and `{pid}` construction
    /// which will be replaced with pid of target kitty process
    #[arg(short, long, default_value = "${XDG_RUNTIME_DIR}/kitty-{pid}")]
    kitty_socket: String,
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
    Test,

    /// Run new kitty instance.
    ///
    /// If current focused window have usable environment data (e.g. another kitty
    /// window) - the newly running window will inherit this environment (e.g. cwd).
    #[command(about, long_about)]
    Kitty,
}

impl Launcher {
    /// Run chosen subcommand
    pub fn run(self) {
        let socket = if let Some(path) = self.path.as_ref() {
            MultiSocket::connect_to(path)
        } else {
            MultiSocket::connect().unwrap()
        };
        match self.command {
            Command::Test => {
                socket.get_socket().unwrap();
            }
            Command::Kitty => {
                self.run_kitty(socket);
            }
        }
    }

    fn run_kitty(&self, socket: MultiSocket) {
        let mut res = Err(io::Error::new(io::ErrorKind::Other, ""));
        if let Some(window) = Self::get_focused_window(&socket) {
            res = self.run_from_kitty(window);
        }
        if res.is_err() {
            Self::run_kitty_fresh()
        }
    }

    fn get_socket(&self, pid: i32) -> io::Result<kitty::KittySocket> {
        let pidre = regex::Regex::new(r"\{pid\}").unwrap();
        let envre = regex::Regex::new(r"\$\{([^\{\}\s]*)\}").unwrap();

        let path =
            envre.replace_all(&self.kitty_socket, |caps: &regex::Captures| {
                let var = std::env::var_os(&caps[1].to_string())
                    .unwrap_or(OsString::from(""));
                String::from(var.to_str().unwrap())
            });

        let path = pidre.replace_all(&path, format!("{pid}"));

        kitty::KittySocket::connect(PathBuf::from(path.to_string()))
    }

    fn run_from_kitty(&self, window: niri_ipc::Window) -> io::Result<()> {
        let class = window.app_id;
        let pid = window.pid;
        let mut socket = None;
        if let Some(class) = class {
            if class == "kitty" {
                if let Some(pid) = pid {
                    socket = Some(self.get_socket(pid)?);
                }
            }
        }
        if let Some(mut socket) = socket {
            let r = kitty::Command::Ls(kitty::Ls::default());
            let r = socket.request(r).unwrap();
            let windows: Vec<kitty::OsWindow> =
                serde_json::from_value(r).unwrap();
            if let Some(window) = Self::get_focused_kitty_window(windows) {
                let mut r = kitty::Launch::default();
                r.launch_type = Some(kitty::LaunchType::OsWindow);
                r.cwd = Some(window.cwd);
                r.copy_env = Some(true);
                r.env = Some(vec!["SHLVL=1".into()]);
                socket.send(kitty::Command::Launch(r))?;
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::Other, ""))
            }
        } else {
            Err(io::Error::new(io::ErrorKind::Other, ""))
        }
    }

    fn run_kitty_fresh() {
        std::process::Command::new("kitty").exec();
    }

    fn get_focused_kitty_window(
        windows: Vec<kitty::OsWindow>,
    ) -> Option<kitty::Window> {
        for window in windows {
            if window.is_focused {
                for tab in window.tabs {
                    if tab.is_focused {
                        for window in tab.windows {
                            if window.is_focused {
                                return Some(window);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn get_focused_window(socket: &MultiSocket) -> Option<niri_ipc::Window> {
        if let Response::FocusedWindow(window) =
            socket.send(Request::FocusedWindow).unwrap().0.unwrap()
        {
            window
        } else {
            panic!("Unexpected response to FocusedWindow")
        }
    }
}

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
use std::{
    ffi::{OsStr, OsString},
    fmt::Display,
    io, iter,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
};

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

    /// Whenever to launch tool regardless to current focused window
    ///
    /// Launching tool will be run with default cwd withing default environment
    #[arg(short, long, default_value = "false")]
    fresh: bool,
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

    /// Print env for launching command.
    ///
    /// If current focused window have usable environment data (e.g. kitty
    /// window) - this will print environment to use with new window. Usable for development
    /// purposes.
    #[command(about, long_about)]
    Env,

    /// Run new vim instance.
    ///
    /// If current focused window have usable environment data (e.g. kitty
    /// window) - the newly running window will inherit this environment (e.g. cwd).
    #[command(about, long_about)]
    Vim,
}

impl Launcher {
    /// Run chosen subcommand
    pub fn run(self) -> io::Result<()> {
        let socket = if let Some(path) = self.path.as_ref() {
            MultiSocket::connect_to(path)
        } else {
            MultiSocket::connect().unwrap()
        };
        match self.command {
            Command::Test => {
                socket.get_socket()?;
                Ok(())
            }
            Command::Kitty => self.run_kitty(socket),
            Command::Env => self.print_env(socket),
            Command::Vim => self.run_vim(socket),
        }
    }

    fn run_kitty(&self, mut socket: MultiSocket) -> io::Result<()> {
        let mut res = Err(io::Error::new(io::ErrorKind::Other, ""));

        if res.is_err() {
            if self.fresh {
                res = Self::run_kitty_fresh();
            }
        }
        if res.is_err() {
            res = self.run_kitty_from_kitty(&mut socket);
        }
        if res.is_err() {
            res = Self::run_kitty_fresh()
        }

        res
    }

    fn print_env(&self, mut socket: MultiSocket) -> io::Result<()> {
        if let Ok(window) = self.get_kitty_focused_window(&mut socket) {
            for (name, val) in window.env {
                println!("{name}=\"{val}\"");
            }
        }
        Ok(())
    }

    fn run_vim(&self, mut socket: MultiSocket) -> io::Result<()> {
        let mut res = Err(io::Error::new(io::ErrorKind::Other, ""));

        if res.is_err() {
            if self.fresh {
                res = Self::run_vim_fresh();
            }
        }
        if res.is_err() {
            res = self.run_vim_from_kitty(&mut socket);
        }
        if res.is_err() {
            res = Self::run_vim_fresh()
        }

        res
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

    fn run_kitty_from_kitty(&self, socket: &mut MultiSocket) -> io::Result<()> {
        let window = self.get_kitty_focused_window(socket)?;
        let env = window
            .env
            .into_iter()
            .chain(iter::once(("SHLVL".into(), "1".into())));

        Self::run_kitty_intsance(window.cwd.to_str(), Some(env))
    }

    fn run_vim_from_kitty(&self, socket: &mut MultiSocket) -> io::Result<()> {
        let window = self.get_kitty_focused_window(socket)?;

        Self::run_vim_intsance(
            window.cwd.to_str(),
            Some(window.env.into_iter()),
        )
    }

    fn get_kitty_focused_window(
        &self,
        socket: &mut MultiSocket,
    ) -> io::Result<kitty::Window> {
        let window = Self::get_focused_window(&socket).ok_or(
            io::Error::new(io::ErrorKind::NotFound, "No focused niri window"),
        )?;
        let class = window.app_id.ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Focused niri window does not have class",
        ))?;
        if class == "kitty" {
            let pid = window.pid.ok_or(io::Error::new(
                io::ErrorKind::NotFound,
                "Focused niri window does not have pid",
            ))?;
            let mut socket = self.get_socket(pid)?;
            let r = kitty::Command::Ls(kitty::Ls::default());
            let r = socket.request(r)?;
            let windows: Vec<kitty::OsWindow> =
                serde_json::from_value(r).unwrap();
            Self::find_kitty_focused_window(windows).ok_or(io::Error::new(
                io::ErrorKind::NotFound,
                "No focused kitty window",
            ))
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Focused niri window is not kitty",
            ))
        }
    }

    fn run_kitty_intsance(
        workdir: Option<impl Into<String>>,
        env: Option<impl Iterator<Item = (impl Display, impl Display)>>,
    ) -> io::Result<()> {
        let mut proc = std::process::Command::new("kitty");

        if let Some(env) = env {
            env.fold(&mut proc, |proc, (name, val)| {
                proc.arg("-o").arg(format!("env={name}={val}"))
            });
        };

        workdir.map(|workdir| {
            proc.arg("-d").arg(format!("{}", workdir.into()));
        });

        Err(proc.exec())
    }

    fn run_kitty_fresh() -> io::Result<()> {
        Self::run_kitty_intsance(
            None as Option<String>,
            None as Option<iter::Empty<(String, String)>>,
        )
    }

    fn run_vim_intsance(
        workdir: Option<impl AsRef<Path>>,
        env: Option<
            impl Iterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
        >,
    ) -> io::Result<()> {
        let mut proc = std::process::Command::new("neovide");

        if let Some(env) = env {
            env.fold(&mut proc, |proc, (name, val)| proc.env(name, val));
        };

        workdir.map(|workdir| {
            proc.current_dir(workdir);
        });

        Err(proc.exec())
    }

    fn run_vim_fresh() -> io::Result<()> {
        Self::run_vim_intsance(
            None as Option<String>,
            None as Option<iter::Empty<(String, String)>>,
        )
    }

    fn find_kitty_focused_window(
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

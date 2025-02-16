use niri_ipc::{socket::Socket, Event, Reply, Request};
use std::{
    env, io,
    path::{Path, PathBuf},
};

/// Name of the environment variable containing the niri IPC socket path.
pub const SOCKET_PATH_ENV: &str = "NIRI_SOCKET";

/// Wrapper on [Socket] which allows to reuse single object for many `send` calls.
pub struct MultiSocket {
    path: PathBuf,
}

impl MultiSocket {
    /// Equivalent to [Socket::connect]
    ///
    /// This stores path taken from [`SOCKET_PATH_ENV`] environment variable.
    pub fn connect() -> io::Result<Self> {
        let socket_path = env::var_os(SOCKET_PATH_ENV).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{SOCKET_PATH_ENV} is not set, are you running this within niri?"),
            )
        })?;
        Ok(Self::connect_to(socket_path))
    }

    /// Equivalent to [Socket::connect_to]
    ///
    /// This stores path passed from argument.
    pub fn connect_to(path_ref: impl AsRef<Path>) -> Self {
        let mut path = PathBuf::new();
        path.push(path_ref);
        Self { path }
    }

    /// Wrapper on [Socket::send]
    ///
    /// Creates temporary [Socket] object, calls its [`send`](Socket::send) method
    /// and returns its result
    pub fn send(
        &self,
        request: Request,
    ) -> io::Result<(Reply, impl FnMut() -> io::Result<Event>)> {
        self.get_socket()?.send(request)
    }

    /// Returns [Socket]
    ///
    /// Uses stored socket path to create and return [Socket] object
    pub fn get_socket(&self) -> io::Result<Socket> {
        Socket::connect_to(&self.path)
    }
}

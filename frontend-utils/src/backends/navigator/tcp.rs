//! WaOS port: a movie's TCP connection, by the system that makes it: how it is opened within
//! the movie's timeout, read and written, and what its whole life runs as.

#[cfg(not(target_os = "wasi"))]
mod desktop;
#[cfg(target_os = "wasi")]
mod waos;

#[cfg(not(target_os = "wasi"))]
pub(super) use desktop::{Connection, start, until_either_ends};
#[cfg(target_os = "wasi")]
pub(super) use waos::{Connection, start, until_either_ends};

//! WaOS port: a movie's TCP connection, by the system that makes it: how it is opened within
//! the movie's timeout, read and written, and what its whole life runs as.

mod desktop;

pub(super) use desktop::{Connection, start, until_either_ends};

//! A connection of tokio's, on tokio's runtime: its whole life runs as a task there, since
//! tokio's sockets are served by that runtime alone.

use crate::backends::navigator::FutureSpawner;
use async_io::Timer;
use futures_lite::FutureExt;
use ruffle_core::loader::Error;
use std::future::Future;
use std::io;
use std::io::ErrorKind;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{ReadHalf, WriteHalf};

/// A connection made.
pub struct Connection(TcpStream);

/// What a connection's peer sends.
pub struct Received<'a>(ReadHalf<'a>);

/// What a connection sends its peer.
pub struct Sent<'a>(WriteHalf<'a>);

impl Connection {
    /// Connects to `host` at `port`; fails with `ErrorKind::TimedOut` once `timeout` has passed.
    pub async fn open(host: String, port: u16, timeout: Duration) -> io::Result<Connection> {
        let timeout = async {
            Timer::after(timeout).await;
            Result::<TcpStream, io::Error>::Err(io::Error::new(ErrorKind::TimedOut, ""))
        };
        let stream = TcpStream::connect((host, port)).or(timeout).await?;
        Ok(Connection(stream))
    }

    pub fn split(&mut self) -> (Received<'_>, Sent<'_>) {
        let (read, write) = self.0.split();
        (Received(read), Sent(write))
    }

    /// Tells the peer nothing more is sent.
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.0.shutdown().await
    }
}

impl Received<'_> {
    /// Reads what has arrived into `buffer`, and answers how much: nothing once the peer is done.
    pub async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read(buffer).await
    }
}

impl Sent<'_> {
    /// Sends the start of `bytes`, and answers how much of it went.
    pub async fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes).await
    }
}

/// Runs `receiving` and `sending` until either ends, which ends the other.
pub async fn until_either_ends(
    receiving: impl Future<Output = ()>,
    sending: impl Future<Output = ()>,
) {
    tokio::select! {
       _ = receiving => {},
       _ = sending => {},
    }
}

/// Starts `connection`, a connection's whole life, as a task.
pub fn start(
    _spawner: &impl FutureSpawner<Error>,
    connection: impl Future<Output = ()> + Send + 'static,
) {
    tokio::spawn(connection);
}

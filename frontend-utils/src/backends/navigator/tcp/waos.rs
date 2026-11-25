//! A connection of WASI's sockets, which WaOS serves: the name is looked up, each of its
//! addresses tried in turn, and then one stream carries what is sent and another what arrives.
//! Its whole life runs as one of the navigator's own tasks, on the app's one thread.
//!
//! The invariant: no task is started here with `wasdk::spawn`, which would hand it to whichever
//! wait runs next on the app's thread and hold that wait until the connection ends.

use crate::backends::navigator::FutureSpawner;
use futures_lite::FutureExt;
use ruffle_core::loader::Error;
use std::future::Future;
use std::io;
use std::io::ErrorKind;
use std::time::Duration;
use wasdk::sockets::ip_name_lookup;
use wasdk::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
    TcpSocket,
};
use wasdk::{FutureReader, StreamReader, StreamResult, StreamWriter, monotonic_clock, wit_stream};

/// How a stream ended, as WaOS tells it once the stream is done.
type Ending = FutureReader<Result<(), ErrorCode>>;

/// A connection made.
pub struct Connection {
    /// Closes the connection once dropped; the streams are read and written meanwhile.
    _socket: TcpSocket,
    received: StreamReader<u8>,
    receiving: Option<Ending>,
    /// None once the peer was told nothing more is sent.
    sent: Option<StreamWriter<u8>>,
    sending: Option<Ending>,
}

/// What a connection's peer sends.
pub struct Received<'a> {
    stream: &'a mut StreamReader<u8>,
    ending: &'a mut Option<Ending>,
}

/// What a connection sends its peer.
pub struct Sent<'a>(&'a mut Option<StreamWriter<u8>>);

impl Connection {
    /// Connects to `host` at `port`; fails with `ErrorKind::TimedOut` once `timeout` has passed.
    pub async fn open(host: String, port: u16, timeout: Duration) -> io::Result<Connection> {
        let timed_out = async {
            let nanoseconds = timeout.as_nanos().try_into().unwrap_or(u64::MAX);
            monotonic_clock::wait_for(nanoseconds).await;
            Err(io::Error::new(ErrorKind::TimedOut, ""))
        };
        Connection::to_first_address(host, port).or(timed_out).await
    }

    /// Connects to the first of `host`'s addresses that takes a connection at `port`; fails as
    /// the last one refused.
    async fn to_first_address(host: String, port: u16) -> io::Result<Connection> {
        let addresses = ip_name_lookup::resolve_addresses(host)
            .await
            .map_err(lookup_error)?;
        let mut refusal = io::Error::new(ErrorKind::NotFound, "the name has no address");
        for address in addresses {
            match Connection::to(socket_address(address, port)).await {
                Ok(connection) => return Ok(connection),
                Err(error) => refusal = error,
            }
        }
        Err(refusal)
    }

    async fn to(address: IpSocketAddress) -> io::Result<Connection> {
        let family = match address {
            IpSocketAddress::Ipv4(_) => IpAddressFamily::Ipv4,
            IpSocketAddress::Ipv6(_) => IpAddressFamily::Ipv6,
        };
        let socket = TcpSocket::create(family).map_err(socket_error)?;
        socket.connect(address).await.map_err(socket_error)?;
        let (sent, taken_by_socket) = wit_stream::new::<u8>();
        let sending = socket.send(taken_by_socket);
        let (received, receiving) = socket.receive();
        Ok(Connection {
            _socket: socket,
            received,
            receiving: Some(receiving),
            sent: Some(sent),
            sending: Some(sending),
        })
    }

    pub fn split(&mut self) -> (Received<'_>, Sent<'_>) {
        let received = Received {
            stream: &mut self.received,
            ending: &mut self.receiving,
        };
        (received, Sent(&mut self.sent))
    }

    /// Tells the peer nothing more is sent, once what was written has gone: the stream ends,
    /// which WaOS passes on.
    pub async fn shutdown(&mut self) -> io::Result<()> {
        drop(self.sent.take());
        match self.sending.take() {
            Some(sending) => sending.await.map_err(socket_error),
            None => Ok(()),
        }
    }
}

impl Received<'_> {
    /// Reads what has arrived into `buffer`, and answers how much: nothing once the peer is done.
    pub async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let room = Vec::with_capacity(buffer.len());
            let (result, bytes) = self.stream.read(room).await;
            if !bytes.is_empty() {
                buffer[..bytes.len()].copy_from_slice(&bytes);
                return Ok(bytes.len());
            }
            if result != StreamResult::Complete(0) {
                return self.ended().await;
            }
        }
    }

    /// Nothing more arrives: why, where the connection failed.
    async fn ended(&mut self) -> io::Result<usize> {
        match self.ending.take() {
            Some(ending) => ending.await.map(|()| 0).map_err(socket_error),
            None => Ok(0),
        }
    }
}

impl Sent<'_> {
    /// Sends the start of `bytes`, and answers how much of it went.
    pub async fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(stream) = self.0.as_mut() else {
            return Err(ErrorKind::NotConnected.into());
        };
        let left = stream.write_all(bytes.to_vec()).await;
        if left.len() == bytes.len() && !bytes.is_empty() {
            return Err(ErrorKind::BrokenPipe.into());
        }
        Ok(bytes.len() - left.len())
    }
}

/// Runs `receiving` and `sending` until either ends, which ends the other.
pub async fn until_either_ends(
    receiving: impl Future<Output = ()>,
    sending: impl Future<Output = ()>,
) {
    receiving.or(sending).await;
}

/// Starts `connection`, a connection's whole life, as a task of the navigator's.
pub fn start(spawner: &impl FutureSpawner<Error>, connection: impl Future<Output = ()> + 'static) {
    spawner.spawn(Box::pin(async move {
        connection.await;
        Ok(())
    }));
}

fn socket_address(address: IpAddress, port: u16) -> IpSocketAddress {
    match address {
        IpAddress::Ipv4(address) => IpSocketAddress::Ipv4(Ipv4SocketAddress { port, address }),
        IpAddress::Ipv6(address) => IpSocketAddress::Ipv6(Ipv6SocketAddress {
            port,
            flow_info: 0,
            address,
            scope_id: 0,
        }),
    }
}

fn socket_error(code: ErrorCode) -> io::Error {
    let kind = match code {
        ErrorCode::Timeout => ErrorKind::TimedOut,
        ErrorCode::AccessDenied => ErrorKind::PermissionDenied,
        ErrorCode::ConnectionRefused => ErrorKind::ConnectionRefused,
        ErrorCode::ConnectionReset => ErrorKind::ConnectionReset,
        ErrorCode::ConnectionAborted => ErrorKind::ConnectionAborted,
        _ => ErrorKind::Other,
    };
    io::Error::new(kind, format!("WaOS answered {code:?}"))
}

fn lookup_error(code: ip_name_lookup::ErrorCode) -> io::Error {
    let kind = match code {
        ip_name_lookup::ErrorCode::AccessDenied => ErrorKind::PermissionDenied,
        _ => ErrorKind::Other,
    };
    io::Error::new(kind, format!("WaOS answered the lookup {code:?}"))
}

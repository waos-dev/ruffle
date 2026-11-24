//! What the player asks of the world outside the movie: files, links, and the futures its loads
//! run as.
//!
//! In this step a movie reaches only the program's own files, which is where WaOS mounts what its
//! user picks: a `file:` URL is read from there, and a relative URL is taken from the movie's
//! folder. Fetching over the network, opening a link and connecting a socket are refused, each
//! saying so.
//!
//! The invariant: a future the player spawns runs as a task of wasdk's executor, never on the
//! caller's stack. It locks the player, which the caller holds (`program`'s invariant).

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_channel::{Receiver, Sender};
use ruffle_core::backend::navigator::{
    ErrorResponse, NavigationMethod, NavigatorBackend, OwnedFuture, Request, SuccessResponse,
    fetch_path, resolve_url_with_relative_base_path,
};
use ruffle_core::indexmap::IndexMap;
use ruffle_core::loader::Error;
use ruffle_core::socket::{ConnectionState, SocketAction, SocketHandle};
use url::{ParseError, Url};

/// How `fetch_path` names this navigator when it refuses a URL that is not a file's.
const NAME: &str = "Ruffle on WaOS, which has no network fetch yet,";

pub struct WaosNavigatorBackend {
    /// The folder of the movie, which its relative URLs are taken from.
    movie_folder: PathBuf,
}

impl WaosNavigatorBackend {
    /// A navigator for the movie at `movie_path`, whose relative URLs name files beside it.
    pub fn beside(movie_path: &Path) -> Self {
        let movie_folder = movie_path.parent().unwrap_or(Path::new("/")).to_path_buf();
        WaosNavigatorBackend { movie_folder }
    }
}

impl NavigatorBackend for WaosNavigatorBackend {
    fn navigate_to_url(
        &self,
        url: &str,
        _target: &str,
        _vars_method: Option<(NavigationMethod, IndexMap<String, String>)>,
    ) {
        tracing::warn!("the movie asked to open {url}: opening links is not supported yet");
    }

    fn fetch(&self, request: Request) -> OwnedFuture<Box<dyn SuccessResponse>, ErrorResponse> {
        fetch_path(self, NAME, request.url(), None)
    }

    fn resolve_url(&self, url: &str) -> Result<Url, ParseError> {
        resolve_url_with_relative_base_path(self, self.movie_folder.clone(), url)
    }

    fn spawn_future(&mut self, future: OwnedFuture<(), Error>) {
        wasdk::spawn(async move {
            if let Err(error) = future.await {
                tracing::error!("a load of the movie's failed: {error}");
            }
        });
    }

    fn pre_process_url(&self, url: Url) -> Url {
        url
    }

    fn connect_socket(
        &mut self,
        host: String,
        port: u16,
        _timeout: Duration,
        handle: SocketHandle,
        _receiver: Receiver<Vec<u8>>,
        sender: Sender<SocketAction>,
    ) {
        tracing::warn!(
            "the movie asked for a socket to {host}:{port}: sockets are not supported yet"
        );
        // The player holds the receiving end for as long as it lives, so the send only fails
        // once nobody is left to hear that the connection failed.
        let _ = sender.try_send(SocketAction::Connect(handle, ConnectionState::Failed));
    }
}

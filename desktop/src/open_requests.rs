//! WaOS port: what the user opens with Ruffle once it runs, a `.swf` file or a link opened with it
//! from WaOS or the system. On a desktop a movie to open arrives as the command line's argument,
//! and only at the start; on WaOS each arrives as an open request, the first one too, which WaOS
//! holds until Ruffle listens. Each opens as a movie picked in the Open dialog does, with the
//! options the preferences hold then.

use crate::custom_event::{RuffleEvent, RuffleEventSender};
use crate::player::LaunchOptions;
use crate::preferences::GlobalPreferences;
use ruffle_frontend_utils::content::ContentDescriptor;
use std::path::Path;
use url::Url;
use wasdk::lifecycle::{self, OpenRequest};

/// Opens the movie of each request from now on, until the program ends.
pub async fn open_each(event_loop: RuffleEventSender, preferences: GlobalPreferences) {
    let mut requests = lifecycle::open_requests();
    while let Some(request) = requests.next().await {
        let Some(movie) = movie_of(&request) else {
            continue;
        };
        let options = Box::new(LaunchOptions::from(&preferences));
        if event_loop
            .send_event(RuffleEvent::Open(movie, options))
            .is_err()
        {
            return;
        }
    }
}

/// The movie `request` opens: its first file, or its first link where it has no file. Ruffle
/// plays one movie at a time, so the others are left.
fn movie_of(request: &OpenRequest) -> Option<ContentDescriptor> {
    let opened = request.files.len() + request.links.len();
    if opened > 1 {
        tracing::warn!("{opened} movies were opened at once; Ruffle plays the first");
    }
    let file = request.files.first().map(|file| {
        ContentDescriptor::new_local(Path::new(file), None).ok_or_else(|| file.clone())
    });
    let link = request.links.first().map(|link| {
        Url::parse(link)
            .map(|url| ContentDescriptor {
                url,
                root_content_path: None,
            })
            .map_err(|_| link.clone())
    });
    match file.or(link)? {
        Ok(movie) => Some(movie),
        Err(unreadable) => {
            tracing::warn!("{unreadable} cannot be opened as a movie");
            None
        }
    }
}

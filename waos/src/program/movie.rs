//! Which movie the program plays: the file its first argument names, or else the one its user
//! picks in WaOS's file dialog.
//!
//! Either way the movie is a file among the program's own files, which is where WaOS mounts what
//! the user picks, so it is read with `std::fs` and its URL is a `file:` URL of that path.

use std::path::{Path, PathBuf};

use ruffle_core::tag_utils::SwfMovieData;
use url::Url;
use wasdk::file_dialogs::{self, DialogError, OpenOptions};

/// The path of the movie to play; none when the user was asked and picked nothing.
pub async fn chosen_path() -> Result<Option<PathBuf>, String> {
    match std::env::args().nth(1) {
        Some(argument) => Ok(Some(PathBuf::from(argument))),
        None => picked_path().await,
    }
}

/// The movie in the file at `path`.
pub fn read(path: &Path) -> Result<SwfMovieData, String> {
    let shown = path.display();
    let data = std::fs::read(path).map_err(|error| format!("could not read {shown}: {error}"))?;
    let url = Url::from_file_path(path).map_err(|()| {
        format!("{shown} is not a whole path from the root of the program's files")
    })?;
    SwfMovieData::from_data(&data, url.into(), None, None)
        .map_err(|error| format!("{shown} is not a movie Ruffle reads: {error}"))
}

/// What to call the movie at `path`: its file's name.
pub fn name(path: &Path) -> String {
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => path.display().to_string(),
    }
}

/// Asks the user for a movie in WaOS's file dialog.
async fn picked_path() -> Result<Option<PathBuf>, String> {
    let dialogs =
        file_dialogs::get_file_dialogs().ok_or("no movie given, and WaOS has no file dialog")?;
    let options = OpenOptions::new();
    options.add_type(
        "Flash movie",
        &["swf".to_owned()],
        &["application/x-shockwave-flash".to_owned()],
    );
    match dialogs.open_file(None, options).await {
        Ok(picked) => Ok(picked.map(PathBuf::from)),
        Err(DialogError::Failed(reason)) => Err(format!("the file dialog failed: {reason}")),
    }
}

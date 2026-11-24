//! What movies save, Flash's shared objects, as files in the program's `/data`, which WaOS keeps
//! from one run to the next.
//!
//! The invariant: a shared object's name, which a movie chooses, names a file inside the shared
//! objects' folder and nowhere else: a name with `..` or a root in it is refused.

use std::fs;
use std::path::{Component, Path, PathBuf};

use ruffle_core::backend::storage::StorageBackend;

const SHARED_OBJECTS_FOLDER: &str = "/data/shared_objects";

pub struct WaosStorageBackend {
    folder: PathBuf,
}

impl WaosStorageBackend {
    pub fn new() -> Self {
        WaosStorageBackend {
            folder: PathBuf::from(SHARED_OBJECTS_FOLDER),
        }
    }

    /// The file of the shared object `name`; none, with a warning, for a name that would leave
    /// the folder.
    fn file_of(&self, name: &str) -> Option<PathBuf> {
        let relative = PathBuf::from(format!("{name}.sol"));
        if is_inside_its_folder(&relative) {
            Some(self.folder.join(relative))
        } else {
            tracing::warn!("refused the shared object name {name:?}");
            None
        }
    }
}

impl StorageBackend for WaosStorageBackend {
    fn get(&self, name: &str) -> Option<Vec<u8>> {
        fs::read(self.file_of(name)?).ok()
    }

    fn put(&mut self, name: &str, value: &[u8]) -> bool {
        let Some(file) = self.file_of(name) else {
            return false;
        };
        match write_with_folders(&file, value) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!("could not save the shared object {name:?}: {error}");
                false
            }
        }
    }

    fn remove_key(&mut self, name: &str) {
        if let Some(file) = self.file_of(name) {
            // A shared object never saved has no file to remove.
            let _ = fs::remove_file(file);
        }
    }
}

/// Whether `relative` stays inside the folder it is joined to: every part of it a plain name.
fn is_inside_its_folder(relative: &Path) -> bool {
    relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
}

/// Writes `value` as the file at `path`, making the folders above it first.
fn write_with_folders(path: &Path, value: &[u8]) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, value)
}

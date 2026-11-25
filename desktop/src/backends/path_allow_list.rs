use ruffle_common::url_ext::UrlExt;
use ruffle_frontend_utils::content::ContentDescriptor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// TODO Make this more generic, maybe a manager?
#[derive(Clone)]
pub struct PathAllowList {
    allowed_path_prefixes: Arc<Mutex<Vec<PathBuf>>>,
}

impl PathAllowList {
    pub fn new(content_descriptor: &ContentDescriptor) -> Self {
        let mut allowed_path_prefixes = Vec::new();

        if let Some(movie_path) = content_descriptor.url.to_local_file_path() {
            if let Some(parent) = movie_path.parent() {
                // TODO Remove it after integrating recents & bookmarks with
                //   opening a directory.
                allowed_path_prefixes.push(parent.to_path_buf());
            }
            allowed_path_prefixes.push(movie_path);
        }

        if let Some(root_content_path) = &content_descriptor.root_content_path {
            allowed_path_prefixes.push(root_content_path.clone());
        }

        Self {
            allowed_path_prefixes: Arc::new(Mutex::new(allowed_path_prefixes)),
        }
    }

    /// Checks whether the user already allowed access to the file.
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        for path_prefix in self
            .allowed_path_prefixes
            .lock()
            .expect("Non-poisoned lock")
            .as_slice()
        {
            let Ok(path_prefix) = path_prefix.canonicalize() else {
                continue;
            };
            let Ok(path) = path.canonicalize() else {
                continue;
            };
            if path.starts_with(path_prefix) {
                return true;
            }
        }
        false
    }

    pub fn add_allowed_path_prefix(&self, path_prefix: PathBuf) {
        self.allowed_path_prefixes
            .lock()
            .expect("Non-poisoned lock")
            .push(path_prefix);
    }
}

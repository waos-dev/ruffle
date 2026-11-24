//! The program's window: asked of WaOS, which answers with the view it holds.

use wasdk::views::{self, View, ViewChange};
use wasdk::windows::{Size, WindowConfig};
use wasdk_wgpu::SharedView;

/// Asks WaOS for a window called `title`, `size` logical points large, and waits for its view.
pub async fn open(title: &str, size: Size) -> Result<SharedView, String> {
    // The stream is opened before the request, so the view that arrives for it is not missed.
    let mut changes = views::view_changes();
    let config = WindowConfig::new();
    config.set_title(title);
    config.set_size(size);
    views::request_view(&config, None).map_err(|error| format!("WaOS gave no window: {error}"))?;
    while let Some(change) = changes.next().await {
        if let Some(arrived) = arrived_view(change) {
            return arrived.map(SharedView::new);
        }
    }
    Err("the program's views ended before its window arrived".to_owned())
}

/// The view `change` brought, or why the request got none; none for a change that says neither.
fn arrived_view(change: ViewChange) -> Option<Result<View, String>> {
    match change {
        ViewChange::Added(view) => Some(Ok(view)),
        ViewChange::RequestFailed(failure) => {
            Some(Err(format!("WaOS gave no window: {}", failure.error)))
        }
        ViewChange::Removed(_) => None,
    }
}

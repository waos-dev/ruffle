//! Custom event type for desktop ruffle

use ruffle_core::events::PlayerNotification;
use ruffle_frontend_utils::content::ContentDescriptor;
use std::sync::Arc;

use crate::gui::DialogDescriptor;
use crate::player::{LaunchOptions, PlayerRunnable};

pub enum OpenType {
    File,
    Directory,
}

/// Sends [`RuffleEvent`]s to the app's event loop, from the GUI, the player and its backends.
///
/// They send through this instead of naming the windowing library's own proxy, so none of
/// them depends on which event loop the app runs.
#[derive(Clone)]
pub struct RuffleEventSender(Arc<SendEvent>);

type SendEvent = dyn Fn(RuffleEvent) -> Result<(), EventLoopClosed> + Send + Sync;

/// The event loop is gone, so the event was not delivered.
#[derive(Debug, thiserror::Error)]
#[error("The event loop is closed")]
pub struct EventLoopClosed;

impl RuffleEventSender {
    pub fn new(
        send: impl Fn(RuffleEvent) -> Result<(), EventLoopClosed> + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(send))
    }

    pub fn send_event(&self, event: RuffleEvent) -> Result<(), EventLoopClosed> {
        (self.0)(event)
    }
}

/// User-defined events.
pub enum RuffleEvent {
    /// Indicates that a task is ready to be polled.
    TaskPoll(PlayerRunnable),

    /// Indicates that an asynchronous SWF metadata load has been completed.
    OnMetadata(ruffle_core::swf::HeaderExt),

    /// The user requested to pick and then open a file.
    BrowseAndOpen(Box<LaunchOptions>, OpenType),

    /// The user requested to open a movie.
    Open(ContentDescriptor, Box<LaunchOptions>),

    /// The user requested to close the current SWF.
    CloseFile,

    /// The user requested to enter full screen.
    EnterFullScreen,

    /// The user requested to exit full screen.
    ExitFullScreen,

    /// The user requested to exit Ruffle.
    ExitRequested,

    /// The user selected an item in the right-click context menu.
    ContextMenuItemClicked(usize),

    /// The movie wants to open a dialog.
    OpenDialog(DialogDescriptor),

    /// Ruffle core has a notification to handle.
    PlayerNotification(PlayerNotification),

    /// Export Ruffle Bundle from currently playing content and open save dialog.
    ExportBundle,
}

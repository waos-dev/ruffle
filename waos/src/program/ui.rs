//! What the player asks of the window around the movie.
//!
//! In this step that is the pointer's shape, set on the view, and the log. The rest answers as a
//! platform without it: the clipboard is empty and keeps nothing, full screen is refused, no
//! device font is found beyond the one Ruffle carries, and no file dialog opens for a movie.

use std::sync::Arc;

use ruffle_core::backend::ui::{
    DialogResultFuture, FileFilter, FontDefinition, FullscreenError, LanguageIdentifier,
    MouseCursor, MultiDialogResultFuture, US_ENGLISH, UiBackend,
};
use ruffle_core::font::FontQuery;
use url::Url;
use wasdk::views::{Cursor, View};

pub struct WaosUiBackend {
    view: Arc<View>,
    cursor: MouseCursor,
    cursor_visible: bool,
}

impl WaosUiBackend {
    pub fn new(view: Arc<View>) -> Self {
        WaosUiBackend {
            view,
            cursor: MouseCursor::Arrow,
            cursor_visible: true,
        }
    }

    /// Shows the pointer over the view as the movie last asked: its shape, or none while hidden.
    fn show_cursor(&self) {
        let cursor = match self.cursor {
            _ if !self.cursor_visible => Cursor::None,
            MouseCursor::Arrow => Cursor::Default,
            MouseCursor::Hand => Cursor::Pointer,
            MouseCursor::IBeam => Cursor::Text,
            MouseCursor::Grab => Cursor::Grab,
        };
        self.view.set_cursor(cursor);
    }
}

impl UiBackend for WaosUiBackend {
    fn mouse_visible(&self) -> bool {
        self.cursor_visible
    }

    fn set_mouse_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
        self.show_cursor();
    }

    fn set_mouse_cursor(&mut self, cursor: MouseCursor) {
        self.cursor = cursor;
        self.show_cursor();
    }

    /// Not supported yet: the clipboard reads as empty.
    fn clipboard_content(&mut self) -> String {
        String::new()
    }

    /// Not supported yet: what a movie copies goes nowhere.
    fn set_clipboard_content(&mut self, _content: String) {}

    fn set_fullscreen(&mut self, _is_full: bool) -> Result<(), FullscreenError> {
        Err("Full screen is not supported on WaOS yet".into())
    }

    fn display_root_movie_download_failed_message(&self, _invalid_swf: bool, fetch_error: String) {
        tracing::error!("the movie did not load: {fetch_error}");
    }

    fn message(&self, message: &str) {
        tracing::info!("{message}");
    }

    /// Not supported yet: a movie's text fields are typed in from a keyboard with keys.
    fn open_virtual_keyboard(&self) {}

    fn close_virtual_keyboard(&self) {}

    /// Not supported yet: movies are told the user reads American English.
    fn language(&self) -> LanguageIdentifier {
        US_ENGLISH.clone()
    }

    fn display_unsupported_video(&self, url: Url) {
        tracing::warn!("the video at {url} is in a format Ruffle does not play");
    }

    /// Not supported yet: no font of the system's is found, so text in a device font is drawn in
    /// the font Ruffle carries.
    fn load_device_font(&self, _query: &FontQuery, _register: &mut dyn FnMut(FontDefinition)) {}

    fn sort_device_fonts(
        &self,
        _query: &FontQuery,
        _register: &mut dyn FnMut(FontDefinition),
    ) -> Vec<FontQuery> {
        Vec::new()
    }

    /// Not supported yet: answers that no dialog can be shown.
    fn display_file_open_dialog(
        &mut self,
        _filters: Vec<FileFilter>,
    ) -> Option<DialogResultFuture> {
        None
    }

    /// Not supported yet: answers that no dialog can be shown.
    fn display_file_open_dialog_multiple(
        &mut self,
        _filters: Vec<FileFilter>,
    ) -> Option<MultiDialogResultFuture> {
        None
    }

    /// Not supported yet: answers that no dialog can be shown.
    fn display_file_save_dialog(
        &mut self,
        _file_name: String,
        _title: String,
    ) -> Option<DialogResultFuture> {
        None
    }

    fn close_file_dialog(&mut self) {}
}

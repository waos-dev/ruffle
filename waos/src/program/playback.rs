//! Runs the player from its view's events: a frame of the screen advances the movie and draws it,
//! the pointer and the keyboard reach the movie, a resize changes what it is drawn into, and the
//! window's close ends it.
//!
//! The view's frames are the player's clock. Each frame asks for the next, so they come at the
//! screen's rate, and the time from one to the next is what the player advances by, running the
//! movie's own frames as they fall due. WaOS sends no frame while the window is out of sight, so
//! the movie stands still there.
//!
//! The invariant: the player's lock is taken for one event and let go before the next is awaited
//! (`program`'s invariant).

use std::sync::Mutex;
use std::time::Duration;

use ruffle_core::{FloatDuration, Player, PlayerEvent, ViewportDimensions};
use wasdk::views::{Metrics, ViewEvent, ViewFocusState};
use wasdk_wgpu::SharedView;

use super::keyboard::Keyboard;
use super::pointer::Pointer;

/// Plays `player` on `view` until the view is gone, or its user closes a view that has no window.
pub async fn play(player: &Mutex<Player>, view: &SharedView) -> Result<(), String> {
    let mut events = view
        .events()
        .ok_or("the window's events were taken already")?;
    let mut frame_clock = FrameClock::default();
    let mut pointer = Pointer::new();
    let mut keyboard = Keyboard::default();
    view.request_frame();
    while let Some(event) = events.next().await {
        let mut player = player
            .lock()
            .map_err(|_| "Ruffle's player stopped in a panic")?;
        match event {
            ViewEvent::Frame(frame) => {
                advance(&mut player, frame_clock.time_since_last(frame.time));
                view.request_frame();
            }
            ViewEvent::Metrics(metrics) => player.set_viewport_dimensions(viewport(&metrics)),
            ViewEvent::Pointer(event) => pointer.tell(&mut player, &event),
            ViewEvent::Key(key) => keyboard.tell(&mut player, key),
            ViewEvent::Focus(focus) => {
                player.handle_event(focus_event(focus.state));
            }
            ViewEvent::CloseRequested => match view.window() {
                Some(window) => window.close(),
                None => return Ok(()),
            },
            _ => {}
        }
    }
    Ok(())
}

/// Advances `player` by `elapsed`, and draws it when that changed its picture.
fn advance(player: &mut Player, elapsed: FloatDuration) {
    player.tick(elapsed);
    if player.needs_render() {
        player.render();
    }
}

/// What the player draws into for a view of `metrics`.
fn viewport(metrics: &Metrics) -> ViewportDimensions {
    ViewportDimensions {
        width: metrics.width,
        height: metrics.height,
        scale_factor: metrics.device_pixel_ratio,
    }
}

fn focus_event(state: ViewFocusState) -> PlayerEvent {
    match state {
        ViewFocusState::Focused => PlayerEvent::FocusGained,
        ViewFocusState::Unfocused => PlayerEvent::FocusLost,
    }
}

/// When the last frame was, on the clock a frame's `time` is on, in nanoseconds; none before the
/// first frame.
#[derive(Default)]
struct FrameClock {
    last_frame_time: Option<u64>,
}

impl FrameClock {
    /// How long after the last frame the frame at `time` is; no time at all for the first.
    fn time_since_last(&mut self, time: u64) -> FloatDuration {
        let last = self.last_frame_time.replace(time).unwrap_or(time);
        FloatDuration::from_std(Duration::from_nanos(time.saturating_sub(last)))
    }
}

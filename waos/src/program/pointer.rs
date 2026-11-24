//! WaOS's pointer as Ruffle's mouse.
//!
//! WaOS reports a pointer as Flutter does: where it is and every button held, with `down` for the
//! first button pressed and `up` for the last one released. Ruffle hears each button on its own,
//! so the buttons it was last told of are kept, and each event tells it the buttons that changed.
//! A touch is a mouse with its primary button held.
//!
//! The invariant: `held` is exactly what Ruffle has been told is held, so every press it hears is
//! followed by that button's release.

use ruffle_core::events::{MouseButton, MouseWheelDelta};
use ruffle_core::{Player, PlayerEvent};
use wasdk::views::{PointerButtons, PointerChange, PointerEvent, PointerSignalKind};

/// The buttons of WaOS's pointer that Ruffle knows, with Ruffle's name for each.
const BUTTONS: [(PointerButtons, MouseButton); 3] = [
    (PointerButtons::PRIMARY, MouseButton::Left),
    (PointerButtons::SECONDARY, MouseButton::Right),
    (PointerButtons::TERTIARY, MouseButton::Middle),
];

pub struct Pointer {
    /// The buttons Ruffle was last told are held.
    held: PointerButtons,
}

impl Pointer {
    pub fn new() -> Self {
        Pointer {
            held: PointerButtons::empty(),
        }
    }

    /// Tells `player` what `event` says the pointer did.
    pub fn tell(&mut self, player: &mut Player, event: &PointerEvent) {
        if event.signal == PointerSignalKind::Scroll {
            player.handle_event(wheel_event(event));
            return;
        }
        match event.change {
            PointerChange::Add => player.set_mouse_in_stage(true),
            PointerChange::Hover | PointerChange::Move => {
                player.handle_event(PlayerEvent::MouseMove {
                    x: event.x,
                    y: event.y,
                });
            }
            PointerChange::Remove => {
                player.set_mouse_in_stage(false);
                player.handle_event(PlayerEvent::MouseLeave);
            }
            _ => {}
        }
        self.tell_changed_buttons(player, event);
    }

    /// Tells `player` of each button `event` holds that Ruffle was not told of, and of each it
    /// was told of that `event` no longer holds.
    fn tell_changed_buttons(&mut self, player: &mut Player, event: &PointerEvent) {
        let (x, y) = (event.x, event.y);
        for (flag, button) in BUTTONS {
            let now_held = event.buttons.contains(flag);
            if now_held == self.held.contains(flag) {
                continue;
            }
            self.held.set(flag, now_held);
            player.handle_event(if now_held {
                PlayerEvent::MouseDown {
                    x,
                    y,
                    button,
                    index: None,
                }
            } else {
                PlayerEvent::MouseUp { x, y, button }
            });
        }
    }
}

/// Ruffle's wheel event for a scroll. WaOS counts a scroll towards the end as positive, Ruffle a
/// turn of the wheel away from its user, which scrolls towards the start.
fn wheel_event(scroll: &PointerEvent) -> PlayerEvent {
    PlayerEvent::MouseWheel {
        delta: MouseWheelDelta::Pixels(-scroll.scroll_delta_y),
    }
}

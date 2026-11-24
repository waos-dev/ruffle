//! WaOS's keyboard as Ruffle's: each key as Ruffle's key event, and with a key going down what
//! it means to a text field, the edit it asks for or else the text it typed.
//!
//! Every key is the movie's first. The player takes a key the movie did not handle too, since a
//! game's keys are its own whether or not a script claims them, except one pressed with Control
//! or Command held: that one is left to WaOS, whose menus' shortcut it may be.
//!
//! The invariant: the modifiers held are followed from every key WaOS sends, before the key is
//! read. WaOS names no modifiers on a key, so they are known only from the modifier keys
//! themselves, the synthesized ones included, which correct what the program missed while
//! another window had the keyboard.

use std::collections::HashSet;

use ruffle_core::events::TextControlCode;
use ruffle_core::{Player, PlayerEvent};
use wasdk::keys::logical;
use wasdk::views::{KeyEvent, KeyInput, KeyKind};

use super::keys;

const SHIFT_KEYS: [u64; 2] = [logical::SHIFT_LEFT, logical::SHIFT_RIGHT];

/// Control and Command: whichever of them a system's shortcuts are made with.
const COMMAND_KEYS: [u64; 4] = [
    logical::CONTROL_LEFT,
    logical::CONTROL_RIGHT,
    logical::META_LEFT,
    logical::META_RIGHT,
];

#[derive(Default)]
pub struct Keyboard {
    /// The Shift, Control and Command keys held now, by their logical keys.
    held_modifiers: HashSet<u64>,
}

impl Keyboard {
    /// Tells `player` of `key`, and answers WaOS whether the key was taken.
    pub fn tell(&mut self, player: &mut Player, key: KeyInput) {
        let KeyInput { event, reply } = key;
        self.follow_modifier(&event);
        let handled_by_movie = match event.kind {
            KeyKind::Down | KeyKind::Repeat => self.tell_key_down(player, &event),
            KeyKind::Up => player.handle_event(PlayerEvent::KeyUp {
                key: keys::descriptor(&event),
            }),
        };
        reply.handled(handled_by_movie || !self.command_held());
    }

    /// Tells `player` a key went down, then what it means to a text field: an edit, or else the
    /// text it typed. Answers whether the movie handled any of it.
    fn tell_key_down(&self, player: &mut Player, event: &KeyEvent) -> bool {
        let mut handled = player.handle_event(PlayerEvent::KeyDown {
            key: keys::descriptor(event),
        });
        if let Some(code) = self.text_control(event.logical) {
            handled |= player.handle_event(PlayerEvent::TextControl { code });
        } else if !self.command_held() {
            for codepoint in typed_characters(event) {
                handled |= player.handle_event(PlayerEvent::TextInput { codepoint });
            }
        }
        handled
    }

    /// Keeps `held_modifiers` right when `event` is a modifier key's.
    fn follow_modifier(&mut self, event: &KeyEvent) {
        if !SHIFT_KEYS.contains(&event.logical) && !COMMAND_KEYS.contains(&event.logical) {
            return;
        }
        if event.kind == KeyKind::Up {
            self.held_modifiers.remove(&event.logical);
        } else {
            self.held_modifiers.insert(event.logical);
        }
    }

    fn shift_held(&self) -> bool {
        SHIFT_KEYS
            .iter()
            .any(|key| self.held_modifiers.contains(key))
    }

    fn command_held(&self) -> bool {
        COMMAND_KEYS
            .iter()
            .any(|key| self.held_modifiers.contains(key))
    }

    /// The edit the key `logical` asks of a text field, with the modifiers held now; none for a
    /// key that asks for none.
    fn text_control(&self, logical: u64) -> Option<TextControlCode> {
        let code = match (logical, self.command_held(), self.shift_held()) {
            (logical::ENTER | logical::NUMPAD_ENTER, _, _) => TextControlCode::Enter,
            (logical::KEY_A, true, _) => TextControlCode::SelectAll,
            (logical::KEY_C, true, _) => TextControlCode::Copy,
            (logical::KEY_V, true, _) => TextControlCode::Paste,
            (logical::KEY_X, true, _) => TextControlCode::Cut,
            (logical::BACKSPACE, true, _) => TextControlCode::BackspaceWord,
            (logical::BACKSPACE, false, _) => TextControlCode::Backspace,
            (logical::DELETE, true, _) => TextControlCode::DeleteWord,
            (logical::DELETE, false, _) => TextControlCode::Delete,
            (logical::ARROW_LEFT, true, true) => TextControlCode::SelectLeftWord,
            (logical::ARROW_LEFT, true, false) => TextControlCode::MoveLeftWord,
            (logical::ARROW_LEFT, false, true) => TextControlCode::SelectLeft,
            (logical::ARROW_LEFT, false, false) => TextControlCode::MoveLeft,
            (logical::ARROW_RIGHT, true, true) => TextControlCode::SelectRightWord,
            (logical::ARROW_RIGHT, true, false) => TextControlCode::MoveRightWord,
            (logical::ARROW_RIGHT, false, true) => TextControlCode::SelectRight,
            (logical::ARROW_RIGHT, false, false) => TextControlCode::MoveRight,
            (logical::HOME, true, true) => TextControlCode::SelectLeftDocument,
            (logical::HOME, true, false) => TextControlCode::MoveLeftDocument,
            (logical::HOME, false, true) => TextControlCode::SelectLeftLine,
            (logical::HOME, false, false) => TextControlCode::MoveLeftLine,
            (logical::END, true, true) => TextControlCode::SelectRightDocument,
            (logical::END, true, false) => TextControlCode::MoveRightDocument,
            (logical::END, false, true) => TextControlCode::SelectRightLine,
            (logical::END, false, false) => TextControlCode::MoveRightLine,
            _ => return None,
        };
        Some(code)
    }
}

/// The characters `event`'s key typed, without the control characters some keys carry as text.
fn typed_characters(event: &KeyEvent) -> impl Iterator<Item = char> {
    event
        .character
        .iter()
        .flat_map(|text| text.chars())
        .filter(|character| !character.is_control())
}

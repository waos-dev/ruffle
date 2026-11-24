//! WaOS's names of keys as Ruffle's.
//!
//! A key of WaOS carries two numbers, Flutter's: where the key is, its USB HID usage, and what it
//! means under the user's layout, a character's code point or Flutter's number for a named key.
//! Ruffle describes a key by the same two, in the web's names, with which of two like keys it is.
//! A key Ruffle has no name for is unknown to it, which Ruffle ignores.

use ruffle_core::events::{KeyDescriptor, KeyLocation, LogicalKey, NamedKey, PhysicalKey};
use wasdk::keys::{logical, physical};
use wasdk::views::KeyEvent;

/// The numbers Flutter gives the number pad's keys: this plus the character on the key.
const NUMPAD_KEYS: u64 = logical::FLUTTER_PLANE | 0x200;

/// How Ruffle describes the key of `event`.
pub fn descriptor(event: &KeyEvent) -> KeyDescriptor {
    KeyDescriptor {
        physical_key: physical_key(event.physical),
        logical_key: logical_key(event),
        key_location: location(event.logical),
    }
}

/// What the key of `event` means: its name, for a key with one, or else its character. The
/// character is the one the key typed, which the held modifiers changed, as Flash reports it;
/// a key that typed none, as every key does coming up, has the character of its logical key.
fn logical_key(event: &KeyEvent) -> LogicalKey {
    if let Some(name) = named_key(event.logical) {
        return LogicalKey::Named(name);
    }
    let typed = event
        .character
        .as_deref()
        .and_then(|text| text.chars().last());
    match typed.or_else(|| character_of(event.logical)) {
        Some(character) => LogicalKey::Character(character),
        None => LogicalKey::Unknown,
    }
}

/// The character the logical key `logical` stands for: its own code point, or for a key of the
/// number pad the character on it.
fn character_of(logical: u64) -> Option<char> {
    let code_point = if is_on_numpad(logical) {
        logical & 0xff
    } else if logical & logical::PLANE_MASK == logical::UNICODE_PLANE {
        logical & logical::VALUE_MASK
    } else {
        return None;
    };
    char::from_u32(code_point as u32)
}

fn is_on_numpad(logical: u64) -> bool {
    logical & !0xff == NUMPAD_KEYS
}

/// Which of two like keys the logical key `logical` is.
fn location(logical: u64) -> KeyLocation {
    match logical {
        logical::SHIFT_LEFT | logical::CONTROL_LEFT | logical::ALT_LEFT | logical::META_LEFT => {
            KeyLocation::Left
        }
        logical::SHIFT_RIGHT
        | logical::CONTROL_RIGHT
        | logical::ALT_RIGHT
        | logical::META_RIGHT => KeyLocation::Right,
        _ if is_on_numpad(logical) => KeyLocation::Numpad,
        _ => KeyLocation::Standard,
    }
}

/// Ruffle's name for the logical key `logical`; none for a key that is a character, or that
/// Ruffle has no name for.
fn named_key(logical: u64) -> Option<NamedKey> {
    let name = match logical {
        logical::ALT_LEFT | logical::ALT_RIGHT => NamedKey::Alt,
        logical::ALT_GRAPH => NamedKey::AltGraph,
        logical::CAPS_LOCK => NamedKey::CapsLock,
        logical::CONTROL_LEFT | logical::CONTROL_RIGHT => NamedKey::Control,
        logical::FN => NamedKey::Fn,
        logical::FN_LOCK => NamedKey::FnLock,
        logical::META_LEFT | logical::META_RIGHT | logical::SUPER_KEY => NamedKey::Super,
        logical::NUM_LOCK => NamedKey::NumLock,
        logical::SCROLL_LOCK => NamedKey::ScrollLock,
        logical::SHIFT_LEFT | logical::SHIFT_RIGHT => NamedKey::Shift,
        logical::SYMBOL => NamedKey::Symbol,
        logical::SYMBOL_LOCK => NamedKey::SymbolLock,
        logical::ENTER | logical::NUMPAD_ENTER => NamedKey::Enter,
        logical::TAB => NamedKey::Tab,
        logical::ARROW_DOWN => NamedKey::ArrowDown,
        logical::ARROW_LEFT => NamedKey::ArrowLeft,
        logical::ARROW_RIGHT => NamedKey::ArrowRight,
        logical::ARROW_UP => NamedKey::ArrowUp,
        logical::END => NamedKey::End,
        logical::HOME => NamedKey::Home,
        logical::PAGE_DOWN => NamedKey::PageDown,
        logical::PAGE_UP => NamedKey::PageUp,
        logical::BACKSPACE => NamedKey::Backspace,
        logical::CLEAR => NamedKey::Clear,
        logical::COPY => NamedKey::Copy,
        logical::CR_SEL => NamedKey::CrSel,
        logical::CUT => NamedKey::Cut,
        logical::DELETE => NamedKey::Delete,
        logical::ERASE_EOF => NamedKey::EraseEof,
        logical::EX_SEL => NamedKey::ExSel,
        logical::INSERT => NamedKey::Insert,
        logical::PASTE => NamedKey::Paste,
        logical::REDO => NamedKey::Redo,
        logical::UNDO => NamedKey::Undo,
        logical::CONTEXT_MENU => NamedKey::ContextMenu,
        logical::ESCAPE => NamedKey::Escape,
        logical::PAUSE => NamedKey::Pause,
        logical::PLAY => NamedKey::Play,
        logical::SELECT => NamedKey::Select,
        logical::ZOOM_IN => NamedKey::ZoomIn,
        logical::ZOOM_OUT => NamedKey::ZoomOut,
        logical::PRINT_SCREEN => NamedKey::PrintScreen,
        logical::F1 => NamedKey::F1,
        logical::F2 => NamedKey::F2,
        logical::F3 => NamedKey::F3,
        logical::F4 => NamedKey::F4,
        logical::F5 => NamedKey::F5,
        logical::F6 => NamedKey::F6,
        logical::F7 => NamedKey::F7,
        logical::F8 => NamedKey::F8,
        logical::F9 => NamedKey::F9,
        logical::F10 => NamedKey::F10,
        logical::F11 => NamedKey::F11,
        logical::F12 => NamedKey::F12,
        logical::F13 => NamedKey::F13,
        logical::F14 => NamedKey::F14,
        logical::F15 => NamedKey::F15,
        logical::F16 => NamedKey::F16,
        logical::F17 => NamedKey::F17,
        logical::F18 => NamedKey::F18,
        logical::F19 => NamedKey::F19,
        logical::F20 => NamedKey::F20,
        logical::F21 => NamedKey::F21,
        logical::F22 => NamedKey::F22,
        logical::F23 => NamedKey::F23,
        logical::F24 => NamedKey::F24,
        _ => return None,
    };
    Some(name)
}

/// Ruffle's name for the key at the USB HID usage `usage`.
fn physical_key(usage: u64) -> PhysicalKey {
    match usage {
        physical::BACKQUOTE => PhysicalKey::Backquote,
        physical::DIGIT0 => PhysicalKey::Digit0,
        physical::DIGIT1 => PhysicalKey::Digit1,
        physical::DIGIT2 => PhysicalKey::Digit2,
        physical::DIGIT3 => PhysicalKey::Digit3,
        physical::DIGIT4 => PhysicalKey::Digit4,
        physical::DIGIT5 => PhysicalKey::Digit5,
        physical::DIGIT6 => PhysicalKey::Digit6,
        physical::DIGIT7 => PhysicalKey::Digit7,
        physical::DIGIT8 => PhysicalKey::Digit8,
        physical::DIGIT9 => PhysicalKey::Digit9,
        physical::MINUS => PhysicalKey::Minus,
        physical::EQUAL => PhysicalKey::Equal,
        physical::INTL_YEN => PhysicalKey::IntlYen,
        physical::KEY_A => PhysicalKey::KeyA,
        physical::KEY_B => PhysicalKey::KeyB,
        physical::KEY_C => PhysicalKey::KeyC,
        physical::KEY_D => PhysicalKey::KeyD,
        physical::KEY_E => PhysicalKey::KeyE,
        physical::KEY_F => PhysicalKey::KeyF,
        physical::KEY_G => PhysicalKey::KeyG,
        physical::KEY_H => PhysicalKey::KeyH,
        physical::KEY_I => PhysicalKey::KeyI,
        physical::KEY_J => PhysicalKey::KeyJ,
        physical::KEY_K => PhysicalKey::KeyK,
        physical::KEY_L => PhysicalKey::KeyL,
        physical::KEY_M => PhysicalKey::KeyM,
        physical::KEY_N => PhysicalKey::KeyN,
        physical::KEY_O => PhysicalKey::KeyO,
        physical::KEY_P => PhysicalKey::KeyP,
        physical::KEY_Q => PhysicalKey::KeyQ,
        physical::KEY_R => PhysicalKey::KeyR,
        physical::KEY_S => PhysicalKey::KeyS,
        physical::KEY_T => PhysicalKey::KeyT,
        physical::KEY_U => PhysicalKey::KeyU,
        physical::KEY_V => PhysicalKey::KeyV,
        physical::KEY_W => PhysicalKey::KeyW,
        physical::KEY_X => PhysicalKey::KeyX,
        physical::KEY_Y => PhysicalKey::KeyY,
        physical::KEY_Z => PhysicalKey::KeyZ,
        physical::BRACKET_LEFT => PhysicalKey::BracketLeft,
        physical::BRACKET_RIGHT => PhysicalKey::BracketRight,
        physical::BACKSLASH => PhysicalKey::Backslash,
        physical::SEMICOLON => PhysicalKey::Semicolon,
        physical::QUOTE => PhysicalKey::Quote,
        physical::INTL_BACKSLASH => PhysicalKey::IntlBackslash,
        physical::COMMA => PhysicalKey::Comma,
        physical::PERIOD => PhysicalKey::Period,
        physical::SLASH => PhysicalKey::Slash,
        physical::INTL_RO => PhysicalKey::IntlRo,
        physical::BACKSPACE => PhysicalKey::Backspace,
        physical::TAB => PhysicalKey::Tab,
        physical::CAPS_LOCK => PhysicalKey::CapsLock,
        physical::ENTER => PhysicalKey::Enter,
        physical::SHIFT_LEFT => PhysicalKey::ShiftLeft,
        physical::SHIFT_RIGHT => PhysicalKey::ShiftRight,
        physical::CONTROL_LEFT => PhysicalKey::ControlLeft,
        physical::META_LEFT => PhysicalKey::SuperLeft,
        physical::ALT_LEFT => PhysicalKey::AltLeft,
        physical::SPACE => PhysicalKey::Space,
        physical::ALT_RIGHT => PhysicalKey::AltRight,
        physical::META_RIGHT => PhysicalKey::SuperRight,
        physical::CONTEXT_MENU => PhysicalKey::ContextMenu,
        physical::CONTROL_RIGHT => PhysicalKey::ControlRight,
        physical::INSERT => PhysicalKey::Insert,
        physical::DELETE => PhysicalKey::Delete,
        physical::HOME => PhysicalKey::Home,
        physical::END => PhysicalKey::End,
        physical::PAGE_UP => PhysicalKey::PageUp,
        physical::PAGE_DOWN => PhysicalKey::PageDown,
        physical::ARROW_UP => PhysicalKey::ArrowUp,
        physical::ARROW_LEFT => PhysicalKey::ArrowLeft,
        physical::ARROW_DOWN => PhysicalKey::ArrowDown,
        physical::ARROW_RIGHT => PhysicalKey::ArrowRight,
        physical::NUM_LOCK => PhysicalKey::NumLock,
        physical::NUMPAD_DIVIDE => PhysicalKey::NumpadDivide,
        physical::NUMPAD_MULTIPLY => PhysicalKey::NumpadMultiply,
        physical::NUMPAD_SUBTRACT => PhysicalKey::NumpadSubtract,
        physical::NUMPAD7 => PhysicalKey::Numpad7,
        physical::NUMPAD8 => PhysicalKey::Numpad8,
        physical::NUMPAD9 => PhysicalKey::Numpad9,
        physical::NUMPAD4 => PhysicalKey::Numpad4,
        physical::NUMPAD5 => PhysicalKey::Numpad5,
        physical::NUMPAD6 => PhysicalKey::Numpad6,
        physical::NUMPAD1 => PhysicalKey::Numpad1,
        physical::NUMPAD2 => PhysicalKey::Numpad2,
        physical::NUMPAD3 => PhysicalKey::Numpad3,
        physical::NUMPAD0 => PhysicalKey::Numpad0,
        physical::NUMPAD_ADD => PhysicalKey::NumpadAdd,
        physical::NUMPAD_COMMA => PhysicalKey::NumpadComma,
        physical::NUMPAD_ENTER => PhysicalKey::NumpadEnter,
        physical::NUMPAD_DECIMAL => PhysicalKey::NumpadDecimal,
        physical::ESCAPE => PhysicalKey::Escape,
        physical::F1 => PhysicalKey::F1,
        physical::F2 => PhysicalKey::F2,
        physical::F3 => PhysicalKey::F3,
        physical::F4 => PhysicalKey::F4,
        physical::F5 => PhysicalKey::F5,
        physical::F6 => PhysicalKey::F6,
        physical::F7 => PhysicalKey::F7,
        physical::F8 => PhysicalKey::F8,
        physical::F9 => PhysicalKey::F9,
        physical::F10 => PhysicalKey::F10,
        physical::F11 => PhysicalKey::F11,
        physical::F12 => PhysicalKey::F12,
        physical::F13 => PhysicalKey::F13,
        physical::F14 => PhysicalKey::F14,
        physical::F15 => PhysicalKey::F15,
        physical::F16 => PhysicalKey::F16,
        physical::F17 => PhysicalKey::F17,
        physical::F18 => PhysicalKey::F18,
        physical::F19 => PhysicalKey::F19,
        physical::F20 => PhysicalKey::F20,
        physical::F21 => PhysicalKey::F21,
        physical::F22 => PhysicalKey::F22,
        physical::F23 => PhysicalKey::F23,
        physical::F24 => PhysicalKey::F24,
        physical::FN => PhysicalKey::Fn,
        physical::FN_LOCK => PhysicalKey::FnLock,
        physical::PRINT_SCREEN => PhysicalKey::PrintScreen,
        physical::SCROLL_LOCK => PhysicalKey::ScrollLock,
        physical::PAUSE => PhysicalKey::Pause,
        _ => PhysicalKey::Unknown,
    }
}

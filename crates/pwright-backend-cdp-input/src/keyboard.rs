//! Bounded Playwright-compatible US keyboard layout resolution.
//!
//! This is a static-data adapter. It performs no browser I/O and allocates no
//! collections, so malformed or unknown key expressions fail before an input
//! transaction starts.

use std::fmt::{self, Display, Formatter};

use pwright_input_model::INPUT_CHORD_MAX_KEYS;

use crate::input::CdpModifiers;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedKey {
    pub key: &'static str,
    pub code: &'static str,
    pub windows_virtual_key_code: i64,
    pub text: &'static str,
    pub location: i64,
    pub modifier: Option<CdpModifiers>,
}

impl ResolvedKey {
    pub const fn is_keypad(self) -> bool {
        self.location == 3
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnknownKey<'a>(pub &'a str);

impl Display for UnknownKey<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown key: {:?}", self.0)
    }
}

#[derive(Debug)]
pub struct KeyTokens<'a> {
    values: [&'a str; INPUT_CHORD_MAX_KEYS],
    len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyChordParseError {
    EmptyToken { token: usize },
    TooManyTokens { tokens: usize, maximum: usize },
}

impl Display for KeyChordParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyToken { token } => write!(formatter, "key chord token {token} is empty"),
            Self::TooManyTokens { tokens, maximum } => write!(
                formatter,
                "key chord has {tokens} tokens, exceeding maximum {maximum}"
            ),
        }
    }
}

impl std::error::Error for KeyChordParseError {}

impl<'a> KeyTokens<'a> {
    pub fn parse(value: &'a str) -> Result<Self, KeyChordParseError> {
        let mut values = [""; INPUT_CHORD_MAX_KEYS];
        let mut len = 0;
        let mut start = 0;
        let mut building = false;
        for (offset, character) in value.char_indices() {
            if character == '+' && building {
                if len < INPUT_CHORD_MAX_KEYS {
                    values[len] = &value[start..offset];
                }
                len += 1;
                start = offset + character.len_utf8();
                building = false;
            } else {
                building = true;
            }
        }
        if !building {
            return Err(KeyChordParseError::EmptyToken { token: len });
        }
        if len >= INPUT_CHORD_MAX_KEYS {
            return Err(KeyChordParseError::TooManyTokens {
                tokens: len + 1,
                maximum: INPUT_CHORD_MAX_KEYS,
            });
        }
        values[len] = &value[start..];
        len += 1;
        Ok(Self { values, len })
    }

    pub fn as_slice(&self) -> &[&'a str] {
        &self.values[..self.len]
    }
}

#[derive(Clone, Copy)]
struct KeyDefinition {
    code: &'static str,
    key: &'static str,
    key_code_without_location: i64,
    shift_key: Option<&'static str>,
    text: &'static str,
    location: i64,
}

impl KeyDefinition {
    const fn basic(code: &'static str, key: &'static str, key_code: i64) -> Self {
        Self {
            code,
            key,
            key_code_without_location: key_code,
            shift_key: None,
            text: "",
            location: 0,
        }
    }

    const fn shifted(mut self, shift_key: &'static str) -> Self {
        self.shift_key = Some(shift_key);
        self
    }

    const fn text(mut self, text: &'static str) -> Self {
        self.text = text;
        self
    }

    const fn located(mut self, location: i64, key_code_without_location: i64) -> Self {
        self.location = location;
        self.key_code_without_location = key_code_without_location;
        self
    }
}

const LETTERS: [(&str, &str, i64); 26] = [
    ("KeyA", "a", 65),
    ("KeyB", "b", 66),
    ("KeyC", "c", 67),
    ("KeyD", "d", 68),
    ("KeyE", "e", 69),
    ("KeyF", "f", 70),
    ("KeyG", "g", 71),
    ("KeyH", "h", 72),
    ("KeyI", "i", 73),
    ("KeyJ", "j", 74),
    ("KeyK", "k", 75),
    ("KeyL", "l", 76),
    ("KeyM", "m", 77),
    ("KeyN", "n", 78),
    ("KeyO", "o", 79),
    ("KeyP", "p", 80),
    ("KeyQ", "q", 81),
    ("KeyR", "r", 82),
    ("KeyS", "s", 83),
    ("KeyT", "t", 84),
    ("KeyU", "u", 85),
    ("KeyV", "v", 86),
    ("KeyW", "w", 87),
    ("KeyX", "x", 88),
    ("KeyY", "y", 89),
    ("KeyZ", "z", 90),
];

const DIGITS: [(&str, &str, &str, i64); 10] = [
    ("Digit0", "0", ")", 48),
    ("Digit1", "1", "!", 49),
    ("Digit2", "2", "@", 50),
    ("Digit3", "3", "#", 51),
    ("Digit4", "4", "$", 52),
    ("Digit5", "5", "%", 53),
    ("Digit6", "6", "^", 54),
    ("Digit7", "7", "&", 55),
    ("Digit8", "8", "*", 56),
    ("Digit9", "9", "(", 57),
];

pub fn resolve_key(
    token: &str,
    shift_pressed: bool,
    is_mac: bool,
) -> Result<ResolvedKey, UnknownKey<'_>> {
    let token = if token == "ControlOrMeta" {
        if is_mac { "Meta" } else { "Control" }
    } else {
        token
    };
    let (code, force_shifted) = key_code_for_token(token).ok_or(UnknownKey(token))?;
    let definition = definition_for_code(code).ok_or(UnknownKey(token))?;
    let shifted = force_shifted || (shift_pressed && definition.shift_key.is_some());
    let key = if shifted {
        definition.shift_key.unwrap_or(definition.key)
    } else {
        definition.key
    };
    let text = if shifted {
        definition.shift_key.unwrap_or(definition.text)
    } else if definition.key.chars().count() == 1 {
        definition.key
    } else {
        definition.text
    };
    let modifier = match key {
        "Alt" => Some(CdpModifiers::ALT),
        "Control" => Some(CdpModifiers::CONTROL),
        "Meta" => Some(CdpModifiers::META),
        "Shift" => Some(CdpModifiers::SHIFT),
        _ => None,
    };
    Ok(ResolvedKey {
        key,
        code: definition.code,
        windows_virtual_key_code: definition.key_code_without_location,
        text,
        location: definition.location,
        modifier,
    })
}

fn key_code_for_token(token: &str) -> Option<(&'static str, bool)> {
    let alias = match token {
        "Shift" => Some("ShiftLeft"),
        "Control" => Some("ControlLeft"),
        "Alt" => Some("AltLeft"),
        "Meta" => Some("MetaLeft"),
        "\n" | "\r" => Some("Enter"),
        _ => None,
    };
    if let Some(code) = alias {
        return Some((code, false));
    }
    if definition_for_code(token).is_some() {
        return Some((canonical_code(token)?, false));
    }
    character_code(token)
}

fn canonical_code(token: &str) -> Option<&'static str> {
    definition_for_code(token).map(|definition| definition.code)
}

fn character_code(token: &str) -> Option<(&'static str, bool)> {
    if token.len() == 1 {
        let byte = token.as_bytes()[0];
        if byte.is_ascii_lowercase() {
            return Some((LETTERS[usize::from(byte - b'a')].0, false));
        }
        if byte.is_ascii_uppercase() {
            return Some((LETTERS[usize::from(byte - b'A')].0, true));
        }
        if byte.is_ascii_digit() {
            return Some((DIGITS[usize::from(byte - b'0')].0, false));
        }
    }
    Some(match token {
        "`" => ("Backquote", false),
        "~" => ("Backquote", true),
        "!" => ("Digit1", true),
        "@" => ("Digit2", true),
        "#" => ("Digit3", true),
        "$" => ("Digit4", true),
        "%" => ("Digit5", true),
        "^" => ("Digit6", true),
        "&" => ("Digit7", true),
        "*" => ("Digit8", true),
        "(" => ("Digit9", true),
        ")" => ("Digit0", true),
        "-" => ("Minus", false),
        "_" => ("Minus", true),
        "=" => ("Equal", false),
        "+" => ("Equal", true),
        "\\" => ("Backslash", false),
        "|" => ("Backslash", true),
        "[" => ("BracketLeft", false),
        "{" => ("BracketLeft", true),
        "]" => ("BracketRight", false),
        "}" => ("BracketRight", true),
        ";" => ("Semicolon", false),
        ":" => ("Semicolon", true),
        "'" => ("Quote", false),
        "\"" => ("Quote", true),
        "," => ("Comma", false),
        "<" => ("Comma", true),
        "." => ("Period", false),
        ">" => ("Period", true),
        "/" => ("Slash", false),
        "?" => ("Slash", true),
        " " => ("Space", false),
        _ => return None,
    })
}

fn definition_for_code(code: &str) -> Option<KeyDefinition> {
    if let Some(&(code, key, key_code)) = LETTERS.iter().find(|entry| entry.0 == code) {
        let upper = match key {
            "a" => "A",
            "b" => "B",
            "c" => "C",
            "d" => "D",
            "e" => "E",
            "f" => "F",
            "g" => "G",
            "h" => "H",
            "i" => "I",
            "j" => "J",
            "k" => "K",
            "l" => "L",
            "m" => "M",
            "n" => "N",
            "o" => "O",
            "p" => "P",
            "q" => "Q",
            "r" => "R",
            "s" => "S",
            "t" => "T",
            "u" => "U",
            "v" => "V",
            "w" => "W",
            "x" => "X",
            "y" => "Y",
            "z" => "Z",
            _ => return None,
        };
        return Some(KeyDefinition::basic(code, key, key_code).shifted(upper));
    }
    if let Some(&(code, key, shift, key_code)) = DIGITS.iter().find(|entry| entry.0 == code) {
        return Some(KeyDefinition::basic(code, key, key_code).shifted(shift));
    }
    let definition = match code {
        "Escape" => KeyDefinition::basic("Escape", "Escape", 27),
        "F1" => KeyDefinition::basic("F1", "F1", 112),
        "F2" => KeyDefinition::basic("F2", "F2", 113),
        "F3" => KeyDefinition::basic("F3", "F3", 114),
        "F4" => KeyDefinition::basic("F4", "F4", 115),
        "F5" => KeyDefinition::basic("F5", "F5", 116),
        "F6" => KeyDefinition::basic("F6", "F6", 117),
        "F7" => KeyDefinition::basic("F7", "F7", 118),
        "F8" => KeyDefinition::basic("F8", "F8", 119),
        "F9" => KeyDefinition::basic("F9", "F9", 120),
        "F10" => KeyDefinition::basic("F10", "F10", 121),
        "F11" => KeyDefinition::basic("F11", "F11", 122),
        "F12" => KeyDefinition::basic("F12", "F12", 123),
        "Backquote" => KeyDefinition::basic("Backquote", "`", 192).shifted("~"),
        "Minus" => KeyDefinition::basic("Minus", "-", 189).shifted("_"),
        "Equal" => KeyDefinition::basic("Equal", "=", 187).shifted("+"),
        "Backslash" => KeyDefinition::basic("Backslash", "\\", 220).shifted("|"),
        "Backspace" => KeyDefinition::basic("Backspace", "Backspace", 8),
        "Tab" => KeyDefinition::basic("Tab", "Tab", 9),
        "BracketLeft" => KeyDefinition::basic("BracketLeft", "[", 219).shifted("{"),
        "BracketRight" => KeyDefinition::basic("BracketRight", "]", 221).shifted("}"),
        "CapsLock" => KeyDefinition::basic("CapsLock", "CapsLock", 20),
        "Semicolon" => KeyDefinition::basic("Semicolon", ";", 186).shifted(":"),
        "Quote" => KeyDefinition::basic("Quote", "'", 222).shifted("\""),
        "Enter" => KeyDefinition::basic("Enter", "Enter", 13).text("\r"),
        "ShiftLeft" => KeyDefinition::basic("ShiftLeft", "Shift", 160).located(1, 16),
        "ShiftRight" => KeyDefinition::basic("ShiftRight", "Shift", 161).located(2, 16),
        "Comma" => KeyDefinition::basic("Comma", ",", 188).shifted("<"),
        "Period" => KeyDefinition::basic("Period", ".", 190).shifted(">"),
        "Slash" => KeyDefinition::basic("Slash", "/", 191).shifted("?"),
        "ControlLeft" => KeyDefinition::basic("ControlLeft", "Control", 162).located(1, 17),
        "MetaLeft" => KeyDefinition::basic("MetaLeft", "Meta", 91).located(1, 91),
        "AltLeft" => KeyDefinition::basic("AltLeft", "Alt", 164).located(1, 18),
        "Space" => KeyDefinition::basic("Space", " ", 32),
        "AltRight" => KeyDefinition::basic("AltRight", "Alt", 165).located(2, 18),
        "AltGraph" => KeyDefinition::basic("AltGraph", "AltGraph", 225),
        "MetaRight" => KeyDefinition::basic("MetaRight", "Meta", 92).located(2, 92),
        "ContextMenu" => KeyDefinition::basic("ContextMenu", "ContextMenu", 93),
        "ControlRight" => KeyDefinition::basic("ControlRight", "Control", 163).located(2, 17),
        "PrintScreen" => KeyDefinition::basic("PrintScreen", "PrintScreen", 44),
        "ScrollLock" => KeyDefinition::basic("ScrollLock", "ScrollLock", 145),
        "Pause" => KeyDefinition::basic("Pause", "Pause", 19),
        "PageUp" => KeyDefinition::basic("PageUp", "PageUp", 33),
        "PageDown" => KeyDefinition::basic("PageDown", "PageDown", 34),
        "Insert" => KeyDefinition::basic("Insert", "Insert", 45),
        "Delete" => KeyDefinition::basic("Delete", "Delete", 46),
        "Home" => KeyDefinition::basic("Home", "Home", 36),
        "End" => KeyDefinition::basic("End", "End", 35),
        "ArrowLeft" => KeyDefinition::basic("ArrowLeft", "ArrowLeft", 37),
        "ArrowUp" => KeyDefinition::basic("ArrowUp", "ArrowUp", 38),
        "ArrowRight" => KeyDefinition::basic("ArrowRight", "ArrowRight", 39),
        "ArrowDown" => KeyDefinition::basic("ArrowDown", "ArrowDown", 40),
        "AudioVolumeMute" => KeyDefinition::basic("AudioVolumeMute", "AudioVolumeMute", 173),
        "AudioVolumeDown" => KeyDefinition::basic("AudioVolumeDown", "AudioVolumeDown", 174),
        "AudioVolumeUp" => KeyDefinition::basic("AudioVolumeUp", "AudioVolumeUp", 175),
        "MediaTrackNext" => KeyDefinition::basic("MediaTrackNext", "MediaTrackNext", 176),
        "MediaTrackPrevious" => {
            KeyDefinition::basic("MediaTrackPrevious", "MediaTrackPrevious", 177)
        }
        "MediaPlayPause" => KeyDefinition::basic("MediaPlayPause", "MediaPlayPause", 179),
        "NumLock" => KeyDefinition::basic("NumLock", "NumLock", 144),
        "NumpadDivide" => KeyDefinition::basic("NumpadDivide", "/", 111).located(3, 111),
        "NumpadMultiply" => KeyDefinition::basic("NumpadMultiply", "*", 106).located(3, 106),
        "NumpadSubtract" => KeyDefinition::basic("NumpadSubtract", "-", 109).located(3, 109),
        "Numpad7" => KeyDefinition::basic("Numpad7", "Home", 36)
            .located(3, 36)
            .shifted("7"),
        "Numpad8" => KeyDefinition::basic("Numpad8", "ArrowUp", 38)
            .located(3, 38)
            .shifted("8"),
        "Numpad9" => KeyDefinition::basic("Numpad9", "PageUp", 33)
            .located(3, 33)
            .shifted("9"),
        "Numpad4" => KeyDefinition::basic("Numpad4", "ArrowLeft", 37)
            .located(3, 37)
            .shifted("4"),
        "Numpad5" => KeyDefinition::basic("Numpad5", "Clear", 12)
            .located(3, 12)
            .shifted("5"),
        "Numpad6" => KeyDefinition::basic("Numpad6", "ArrowRight", 39)
            .located(3, 39)
            .shifted("6"),
        "NumpadAdd" => KeyDefinition::basic("NumpadAdd", "+", 107).located(3, 107),
        "Numpad1" => KeyDefinition::basic("Numpad1", "End", 35)
            .located(3, 35)
            .shifted("1"),
        "Numpad2" => KeyDefinition::basic("Numpad2", "ArrowDown", 40)
            .located(3, 40)
            .shifted("2"),
        "Numpad3" => KeyDefinition::basic("Numpad3", "PageDown", 34)
            .located(3, 34)
            .shifted("3"),
        "Numpad0" => KeyDefinition::basic("Numpad0", "Insert", 45)
            .located(3, 45)
            .shifted("0"),
        "NumpadDecimal" => KeyDefinition::basic("NumpadDecimal", "\0", 46)
            .located(3, 46)
            .shifted("."),
        "NumpadEnter" => KeyDefinition::basic("NumpadEnter", "Enter", 13)
            .located(3, 13)
            .text("\r"),
        _ => return None,
    };
    Some(definition)
}

/// Chromium editing commands derived from Playwright's pinned macOS table.
/// Commands beginning with `insert` are intentionally absent because
/// Chromium's adapter filters them before dispatch.
pub(crate) fn mac_editing_commands(code: &str, modifiers: CdpModifiers) -> &'static [&'static str] {
    match (modifiers.mask(), code) {
        (0, "Backspace") | (8, "Backspace") => &["deleteBackward"],
        (0, "Escape") | (8, "Escape") => &["cancelOperation"],
        (0, "ArrowUp") => &["moveUp"],
        (0, "ArrowDown") => &["moveDown"],
        (0, "ArrowLeft") => &["moveLeft"],
        (0, "ArrowRight") => &["moveRight"],
        (0, "F5") | (8, "F5") => &["complete"],
        (0, "Delete") | (8, "Delete") => &["deleteForward"],
        (0, "Home") => &["scrollToBeginningOfDocument"],
        (0, "End") => &["scrollToEndOfDocument"],
        (0, "PageUp") => &["scrollPageUp"],
        (0, "PageDown") => &["scrollPageDown"],
        (8, "ArrowUp") => &["moveUpAndModifySelection"],
        (8, "ArrowDown") => &["moveDownAndModifySelection"],
        (8, "ArrowLeft") => &["moveLeftAndModifySelection"],
        (8, "ArrowRight") => &["moveRightAndModifySelection"],
        (8, "Home") => &["moveToBeginningOfDocumentAndModifySelection"],
        (8, "End") => &["moveToEndOfDocumentAndModifySelection"],
        (8, "PageUp") => &["pageUpAndModifySelection"],
        (8, "PageDown") => &["pageDownAndModifySelection"],
        (8, "Numpad5") => &["delete"],
        (2, "Tab") => &["selectNextKeyView"],
        (2, "KeyA") => &["moveToBeginningOfParagraph"],
        (2, "KeyB") => &["moveBackward"],
        (2, "KeyD") => &["deleteForward"],
        (2, "KeyE") => &["moveToEndOfParagraph"],
        (2, "KeyF") => &["moveForward"],
        (2, "KeyH") => &["deleteBackward"],
        (2, "KeyK") => &["deleteToEndOfParagraph"],
        (2, "KeyL") => &["centerSelectionInVisibleArea"],
        (2, "KeyN") => &["moveDown"],
        (2, "KeyO") => &["moveBackward"],
        (2, "KeyP") => &["moveUp"],
        (2, "KeyT") => &["transpose"],
        (2, "KeyV") => &["pageDown"],
        (2, "KeyY") => &["yank"],
        (2, "Backspace") => &["deleteBackwardByDecomposingPreviousCharacter"],
        (2, "ArrowUp") => &["scrollPageUp"],
        (2, "ArrowDown") => &["scrollPageDown"],
        (2, "ArrowLeft") => &["moveToLeftEndOfLine"],
        (2, "ArrowRight") => &["moveToRightEndOfLine"],
        (10, "Tab") => &["selectPreviousKeyView"],
        (10, "KeyA") => &["moveToBeginningOfParagraphAndModifySelection"],
        (10, "KeyB") => &["moveBackwardAndModifySelection"],
        (10, "KeyE") => &["moveToEndOfParagraphAndModifySelection"],
        (10, "KeyF") => &["moveForwardAndModifySelection"],
        (10, "KeyN") => &["moveDownAndModifySelection"],
        (10, "KeyP") => &["moveUpAndModifySelection"],
        (10, "KeyV") => &["pageDownAndModifySelection"],
        (10, "Backspace") => &["deleteBackwardByDecomposingPreviousCharacter"],
        (10, "ArrowUp") => &["scrollPageUp"],
        (10, "ArrowDown") => &["scrollPageDown"],
        (10, "ArrowLeft") => &["moveToLeftEndOfLineAndModifySelection"],
        (10, "ArrowRight") => &["moveToRightEndOfLineAndModifySelection"],
        (1, "Backspace") => &["deleteWordBackward"],
        (1, "Escape") => &["complete"],
        (1, "ArrowUp") => &["moveBackward", "moveToBeginningOfParagraph"],
        (1, "ArrowDown") => &["moveForward", "moveToEndOfParagraph"],
        (1, "ArrowLeft") => &["moveWordLeft"],
        (1, "ArrowRight") => &["moveWordRight"],
        (1, "Delete") => &["deleteWordForward"],
        (1, "PageUp") => &["pageUp"],
        (1, "PageDown") => &["pageDown"],
        (9, "Backspace") => &["deleteWordBackward"],
        (9, "Escape") => &["complete"],
        (9, "ArrowUp") => &["moveParagraphBackwardAndModifySelection"],
        (9, "ArrowDown") => &["moveParagraphForwardAndModifySelection"],
        (9, "ArrowLeft") => &["moveWordLeftAndModifySelection"],
        (9, "ArrowRight") => &["moveWordRightAndModifySelection"],
        (9, "Delete") => &["deleteWordForward"],
        (9, "PageUp") => &["pageUp"],
        (9, "PageDown") => &["pageDown"],
        (3, "KeyB") => &["moveWordBackward"],
        (3, "KeyF") => &["moveWordForward"],
        (3, "Backspace") => &["deleteWordBackward"],
        (11, "KeyB") => &["moveWordBackwardAndModifySelection"],
        (11, "KeyF") => &["moveWordForwardAndModifySelection"],
        (11, "Backspace") => &["deleteWordBackward"],
        (4, "NumpadSubtract") | (12, "NumpadSubtract") => &["cancel"],
        (4, "Backspace") => &["deleteToBeginningOfLine"],
        (4, "ArrowUp") => &["moveToBeginningOfDocument"],
        (4, "ArrowDown") => &["moveToEndOfDocument"],
        (4, "ArrowLeft") => &["moveToLeftEndOfLine"],
        (4, "ArrowRight") => &["moveToRightEndOfLine"],
        (12, "Backspace") => &["deleteToBeginningOfLine"],
        (12, "ArrowUp") => &["moveToBeginningOfDocumentAndModifySelection"],
        (12, "ArrowDown") => &["moveToEndOfDocumentAndModifySelection"],
        (12, "ArrowLeft") => &["moveToLeftEndOfLineAndModifySelection"],
        (12, "ArrowRight") => &["moveToRightEndOfLineAndModifySelection"],
        (4, "KeyA") => &["selectAll"],
        (4, "KeyC") => &["copy"],
        (4, "KeyX") => &["cut"],
        (4, "KeyV") => &["paste"],
        (4, "KeyZ") => &["undo"],
        (12, "KeyZ") => &["redo"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyTokens, mac_editing_commands, resolve_key};
    use crate::input::CdpModifiers;

    #[test]
    fn plus_splitting_matches_playwright() {
        // Arrange
        let plus = KeyTokens::parse("+").expect("literal plus should parse");
        let shifted_plus = KeyTokens::parse("Shift++").expect("shifted plus should parse");

        // Act
        let plus_tokens = plus.as_slice();
        let shifted_tokens = shifted_plus.as_slice();

        // Assert
        assert_eq!(plus_tokens, &["+"]);
        assert_eq!(shifted_tokens, &["Shift", "+"]);
    }

    #[test]
    fn layout_resolves_shifted_alias_and_keypad_details() {
        // Arrange
        let shifted_a = resolve_key("KeyA", true, false).expect("KeyA should resolve");
        let smart = resolve_key("ControlOrMeta", false, true).expect("smart key should resolve");
        let keypad = resolve_key("Numpad7", true, false).expect("numpad should resolve");

        // Act / Assert
        assert_eq!(
            (shifted_a.key, shifted_a.code, shifted_a.text),
            ("A", "KeyA", "A")
        );
        assert_eq!((smart.key, smart.code), ("Meta", "MetaLeft"));
        assert_eq!((keypad.key, keypad.windows_virtual_key_code), ("7", 36));
        assert!(keypad.is_keypad());
    }

    #[test]
    fn layout_rejects_unknown_tokens() {
        // Arrange
        let token = "HyperSpace";

        // Act
        let error = resolve_key(token, false, false).expect_err("unknown key should fail");

        // Assert
        assert_eq!(error.0, token);
    }

    #[test]
    fn parser_rejects_an_empty_chord() {
        // Arrange
        let value = "";

        // Act
        let error = KeyTokens::parse(value).expect_err("empty chord should fail");

        // Assert
        assert_eq!(error, super::KeyChordParseError::EmptyToken { token: 0 });
    }

    #[test]
    fn parser_rejects_a_trailing_empty_token() {
        // Arrange
        let value = "Shift+";

        // Act
        let error = KeyTokens::parse(value).expect_err("empty suffix should fail");

        // Assert
        assert_eq!(error, super::KeyChordParseError::EmptyToken { token: 1 });
    }

    #[test]
    fn parser_reports_the_exact_oversized_chord_length() {
        // Arrange
        let value = (0..(pwright_input_model::INPUT_CHORD_MAX_KEYS + 2))
            .map(|_| "A")
            .collect::<Vec<_>>()
            .join("+");

        // Act
        let error = KeyTokens::parse(&value).expect_err("oversized chord should fail");

        // Assert
        assert_eq!(
            error,
            super::KeyChordParseError::TooManyTokens {
                tokens: 18,
                maximum: 16,
            }
        );
    }

    #[test]
    fn mac_commands_match_filtered_playwright_fixtures() {
        // Arrange
        let shifted = CdpModifiers::SHIFT;
        let control = CdpModifiers::CONTROL;
        let meta = CdpModifiers::META;

        // Act / Assert
        assert_eq!(
            mac_editing_commands("ArrowLeft", shifted),
            &["moveLeftAndModifySelection"]
        );
        assert_eq!(mac_editing_commands("KeyO", control), &["moveBackward"]);
        assert_eq!(mac_editing_commands("KeyA", meta), &["selectAll"]);
        assert!(mac_editing_commands("Enter", CdpModifiers::NONE).is_empty());
    }
}

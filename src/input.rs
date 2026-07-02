//! TODO

use indexmap::IndexMap;
use itertools::Itertools;
use ratatui::layout::{Position, Size};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display},
    hash::Hash,
    iter,
    str::FromStr,
};
use terminput::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    MouseButton, MouseEvent, MouseEventKind, ScrollDirection,
};
use thiserror::Error;
use tracing::trace;

/// Key code to string mappings
const KEY_CODES: Mapping<'static, KeyCode> = Mapping::new(&[
    // unstable: include ASCII chars
    // https://github.com/rust-lang/rust/issues/110998
    // vvvvv If making changes, make sure to update the docs vvvvv
    (KeyCode::Esc, &["escape", "esc"]),
    (KeyCode::Enter, &["enter"]),
    (KeyCode::Left, &["left"]),
    (KeyCode::Right, &["right"]),
    (KeyCode::Up, &["up"]),
    (KeyCode::Down, &["down"]),
    (KeyCode::Home, &["home"]),
    (KeyCode::End, &["end"]),
    (KeyCode::PageUp, &["pageup", "pgup"]),
    (KeyCode::PageDown, &["pagedown", "pgdn"]),
    (KeyCode::Tab, &["tab"]),
    (KeyCode::Backspace, &["backspace"]),
    (KeyCode::Delete, &["delete", "del"]),
    (KeyCode::Insert, &["insert", "ins"]),
    (KeyCode::CapsLock, &["capslock", "caps"]),
    (KeyCode::ScrollLock, &["scrolllock"]),
    (KeyCode::NumLock, &["numlock"]),
    (KeyCode::PrintScreen, &["printscreen"]),
    (KeyCode::Pause, &["pausebreak"]),
    (KeyCode::Menu, &["menu"]),
    (KeyCode::KeypadBegin, &["keypadbegin"]),
    (KeyCode::F(1), &["f1"]),
    (KeyCode::F(2), &["f2"]),
    (KeyCode::F(3), &["f3"]),
    (KeyCode::F(4), &["f4"]),
    (KeyCode::F(5), &["f5"]),
    (KeyCode::F(6), &["f6"]),
    (KeyCode::F(7), &["f7"]),
    (KeyCode::F(8), &["f8"]),
    (KeyCode::F(9), &["f9"]),
    (KeyCode::F(10), &["f10"]),
    (KeyCode::F(11), &["f11"]),
    (KeyCode::F(12), &["f12"]),
    (KeyCode::Char(' '), &["space"]),
    (KeyCode::Media(MediaKeyCode::Play), &["play"]),
    (KeyCode::Media(MediaKeyCode::Pause), &["pause"]),
    (KeyCode::Media(MediaKeyCode::PlayPause), &["playpause"]),
    (KeyCode::Media(MediaKeyCode::Reverse), &["reverse"]),
    (KeyCode::Media(MediaKeyCode::Stop), &["stop"]),
    (KeyCode::Media(MediaKeyCode::FastForward), &["fastforward"]),
    (KeyCode::Media(MediaKeyCode::Rewind), &["rewind"]),
    (KeyCode::Media(MediaKeyCode::TrackNext), &["tracknext"]),
    (
        KeyCode::Media(MediaKeyCode::TrackPrevious),
        &["trackprevious"],
    ),
    (KeyCode::Media(MediaKeyCode::Record), &["record"]),
    (KeyCode::Media(MediaKeyCode::LowerVolume), &["lowervolume"]),
    (KeyCode::Media(MediaKeyCode::RaiseVolume), &["raisevolume"]),
    (KeyCode::Media(MediaKeyCode::MuteVolume), &["mute"]),
    // ^^^^^ If making changes, make sure to update the docs ^^^^^
]);
/// Key modifier to string mappings
const KEY_MODIFIERS: Mapping<'static, KeyModifiers> = Mapping::new(&[
    // vvvvv If making changes, make sure to update the docs vvvvv
    (KeyModifiers::SHIFT, &["shift"]),
    (KeyModifiers::ALT, &["alt"]),
    (KeyModifiers::CTRL, &["ctrl"]),
    (KeyModifiers::SUPER, &["super"]),
    (KeyModifiers::HYPER, &["hyper"]),
    (KeyModifiers::META, &["meta"]),
    // ^^^^^ If making changes, make sure to update the docs ^^^^^
]);

/// One or more key combinations, which should correspond to a single action
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct InputBinding(Vec<KeyCombination>);

impl InputBinding {
    /// Does this binding have no actions? If true, it should be thrown away
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Does a key event contain this key combo?
    pub fn matches(&self, event: &KeyEvent) -> bool {
        self.0.iter().any(|combo| combo.matches(event))
    }
}

impl Display for InputBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, combo) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, " / ")?;
            }
            write!(f, "{combo}")?;
        }
        Ok(())
    }
}

impl From<Vec<(KeyModifiers, KeyCode)>> for InputBinding {
    fn from(combos: Vec<(KeyModifiers, KeyCode)>) -> Self {
        Self(
            combos
                .into_iter()
                .map(|(modifiers, code)| KeyCombination { modifiers, code })
                .collect(),
        )
    }
}

impl From<(KeyModifiers, KeyCode)> for InputBinding {
    fn from((modifiers, code): (KeyModifiers, KeyCode)) -> Self {
        Self(vec![KeyCombination { modifiers, code }])
    }
}

impl From<Vec<KeyCode>> for InputBinding {
    fn from(value: Vec<KeyCode>) -> Self {
        Self(value.into_iter().map(KeyCombination::from).collect())
    }
}

impl From<KeyCode> for InputBinding {
    fn from(code: KeyCode) -> Self {
        Self(vec![KeyCombination {
            modifiers: KeyModifiers::NONE,
            code,
        }])
    }
}

/// Key input sequence, which can trigger an action
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
pub struct KeyCombination {
    pub modifiers: KeyModifiers,
    pub code: KeyCode,
}

impl KeyCombination {
    /// Char between modifiers and key codes
    const SEPARATOR: char = ' ';

    pub fn matches(self, event: &KeyEvent) -> bool {
        // For char codes, terminal may report the code as caps
        fn to_lowercase(code: KeyCode) -> KeyCode {
            if let KeyCode::Char(c) = code {
                KeyCode::Char(c.to_ascii_lowercase())
            } else {
                code
            }
        }

        to_lowercase(event.code) == to_lowercase(self.code)
            && event.modifiers == self.modifiers
    }
}

/// User-friendly and compact display for a key combination. This is meant to
/// just be used in the UI, *not* for serialization!
impl Display for KeyCombination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Write modifiers first
        for (name, _) in self.modifiers.iter_names() {
            write!(f, "{}{}", name.to_lowercase(), Self::SEPARATOR)?;
        }

        // Write base code
        match self.code {
            KeyCode::Tab => write!(f, "tab"),
            KeyCode::Up => write!(f, "↑"),
            KeyCode::Down => write!(f, "↓"),
            KeyCode::Left => write!(f, "←"),
            KeyCode::Right => write!(f, "→"),
            KeyCode::Esc => write!(f, "esc"),
            KeyCode::Enter => write!(f, "enter"),
            KeyCode::Delete => write!(f, "del"),
            KeyCode::PageUp => write!(f, "pgup"),
            KeyCode::PageDown => write!(f, "pgdown"),
            KeyCode::Home => write!(f, "home"),
            KeyCode::End => write!(f, "end"),
            KeyCode::F(num) => write!(f, "F{num}"),
            KeyCode::Char(' ') => write!(f, "<space>"),
            KeyCode::Char(c) => write!(f, "{c}"),
            // Punting on everything else until we need it
            _ => write!(f, "???"),
        }
    }
}

impl From<KeyCode> for KeyCombination {
    fn from(key_code: KeyCode) -> Self {
        Self {
            code: key_code,
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl FromStr for KeyCombination {
    type Err = InputParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Last char should be the primary one, everything before should be
        // modifiers. Ignore extra whitespace on the ends *or* the middle.
        // Filtering out empty elements is easier than building a regex to split
        let mut tokens =
            s.trim().split(Self::SEPARATOR).filter(|s| !s.is_empty());
        let code = tokens.next_back().ok_or(InputParseError::Empty)?;
        let mut modifiers = KeyModifiers::NONE;
        // `backtab` is what crossterm calls `shift tab`. We supported it in the
        // past because this used to map directly to crossterm. Keeping this
        // mapping for backward compatibility. We need snowflake logic because
        // it's a code that maps to a code+modifier
        let code: KeyCode = if code == "backtab" {
            modifiers |= KeyModifiers::SHIFT;
            KeyCode::Tab
        } else {
            parse_key_code(code)?
        };

        // Parse modifiers, left-to-right
        for modifier in tokens {
            let modifier = parse_key_modifier(modifier)?;
            // Prevent duplicate
            if modifiers.contains(modifier) {
                return Err(InputParseError::DuplicateModifier { modifier });
            }
            modifiers |= modifier;
        }

        Ok(Self { modifiers, code })
    }
}

impl TryFrom<String> for KeyCombination {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// For serialization
impl From<KeyCombination> for String {
    fn from(key_combo: KeyCombination) -> Self {
        key_combo
            .modifiers
            .iter()
            .map(stringify_key_modifier)
            .chain(iter::once(stringify_key_code(key_combo.code)))
            .join(" ")
    }
}

/// Mapping of actions to input bindings
#[derive(Clone, Debug)]
struct InputMap<Action>(
    /// Intuitively this should be binding:action since we get key events from
    /// the user and need to look up the corresponding actions. But we
    /// can't look up a binding from the map based on an input event
    /// because event<=>binding matching is more nuanced that simple
    /// equality (e.g. bonus modifiers keys can be ignored). We have to
    /// iterate over map when checking inputs, but keying by
    /// action at least allows us to look up action=>binding for help text.
    IndexMap<Action, InputBinding>,
);

impl<Action: Eq + Hash + PartialEq> InputMap<Action> {
    /// TODO
    fn new(
        default: IndexMap<Action, InputBinding>,
        user_bindings: IndexMap<Action, InputBinding>,
    ) -> Self {
        let mut merged = default;
        // User bindings should overwrite any default ones
        merged.extend(user_bindings);
        // If the user overwrote an action with an empty binding, remove it from
        // the map. This has to be done *after* the extend, so the default
        // binding is also dropped
        merged.retain(|_, binding| !binding.is_empty());
        Self(merged)
    }

    /// Get the inner action:binding map
    fn into_inner(self) -> IndexMap<Action, InputBinding> {
        self.0
    }
}

/// Error parsing input combination
#[derive(Debug, Error)]
pub enum InputParseError {
    /// Combination contains the same modifier twice
    #[error("Duplicate modifier {modifier:?}")]
    DuplicateModifier { modifier: KeyModifiers },

    /// Input is empty
    #[error("Empty key combination")]
    Empty,

    /// Key code doesn't match any known keys
    #[error(
        "Invalid key code {input:?}; key combinations should be space-separated"
    )]
    InvalidKeyCode { input: String },

    /// Key modifier doesn't match any known modifiers
    #[error(
        "Invalid key modifier {input:?}; must be one of {:?}",
        KEY_MODIFIERS.all_strings().collect_vec(),
    )]
    InvalidKeyModifier { input: String },
}

/// Parse a plain key code
fn parse_key_code(s: &str) -> Result<KeyCode, InputParseError> {
    // Check for plain char code
    if let Ok(c) = s.parse::<char>() {
        Ok(KeyCode::Char(c))
    } else {
        // Don't include the full list of options in the error message, too long
        KEY_CODES
            .get(s)
            .ok_or_else(|| InputParseError::InvalidKeyCode {
                input: s.to_owned(),
            })
    }
}

/// Convert key code to string. Inverse of parsing
fn stringify_key_code(code: KeyCode) -> Cow<'static, str> {
    if let Some(label) = KEY_CODES.get_label(code) {
        // If it's mapped, use the mapped label
        label.into()
    } else if let KeyCode::Char(c) = code {
        // Otherwise we hope it's an ASCII char
        c.to_string().into()
    } else {
        "???".into()
    }
}

/// Parse a key modifier
fn parse_key_modifier(s: &str) -> Result<KeyModifiers, InputParseError> {
    KEY_MODIFIERS
        .get(s)
        .ok_or_else(|| InputParseError::InvalidKeyModifier {
            input: s.to_owned(),
        })
}

/// Convert key modifier to string. Inverse of parsing
fn stringify_key_modifier(modifier: KeyModifiers) -> Cow<'static, str> {
    // unwrap() is safe because all possible modifiers are mapped
    KEY_MODIFIERS.get_label(modifier).unwrap().into()
}

/// Map of input sequences to actions
#[derive(Debug)]
pub struct InputBindings<Action> {
    // TODO merge this with InputMap?
    bindings: InputMap<Action>,
}

impl<Action> InputBindings<Action>
where
    Action: Copy + Clone + Debug + Eq + Hash + PartialEq,
{
    pub fn new(bindings: InputMap<Action>) -> Self {
        Self { bindings }
    }

    /// Get the binding associated with a particular action. Useful for mapping
    /// input in reverse, when showing available bindings to the user.
    pub fn binding(&self, action: Action) -> Option<&InputBinding> {
        self.bindings.0.get(&action)
    }

    /// Get the binding associated with a particular action as a string. If the
    /// action is unbound, use a placeholder string instead
    pub fn binding_display(&self, action: Action) -> String {
        self.binding(action)
            .map(|binding| format!("[{binding}]"))
            .unwrap_or_else(|| "<unbound>".to_owned())
    }

    /// Append a hotkey hint to a label. If the given action is bound, adding
    /// a hint to the end of the given label. If unbound, return the label
    /// alone.
    pub fn add_hint(&self, label: impl Display, action: Action) -> String {
        if let Some(binding) = self.binding(action) {
            format!("{label} [{binding}]")
        } else {
            label.to_string()
        }
    }

    /// Convert a key event into its bound action, if any
    pub fn action(&self, event: &KeyEvent) -> Option<Action> {
        // Scan all bindings for a match
        let action = self
            .bindings
            .0
            .iter()
            .find(|(_, binding)| binding.matches(event))
            .inspect(|(action, binding)| {
                trace!(
                    ?event,
                    ?action,
                    ?binding,
                    "Matched key event to binding"
                );
            })
            .map(|(action, _)| *action);

        if let Some(action) = action {
            trace!(?action, "Input action");
        }

        action
    }

    /// Given a raw input event, generate a corresponding [InputEvent]. For key
    /// events, this includes mapping to the bound action (if any). Some
    /// events should *not* be handled; these will return `None`. This could be
    /// because they're just useless and noisy, or because they actually
    /// cause bugs (e.g. double key presses).
    pub fn convert_event(&self, event: Event) -> Option<InputEvent> {
        match event {
            // Windows sends a release event that causes double triggers
            // https://github.com/LucasPickering/slumber/issues/226
            Event::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            }) => None,

            // Handle everything else
            Event::Key(key_event) => {
                // Check for mapped actions
                let action = self.action(&key_event);
                Some(InputEvent::Key {
                    code: key_event.code,
                    modifiers: key_event.modifiers,
                    action,
                })
            }

            // Detecting mouse UP feels the most natural
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                row,
                column,
                ..
            }) => Some(InputEvent::Click {
                position: (column, row).into(),
            }),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Scroll(direction),
                row,
                column,
                ..
            }) => Some(InputEvent::Scroll {
                direction,
                position: (column, row).into(),
            }),
            Event::Paste(_) => Some(InputEvent::Paste),
            Event::Resize { rows, cols } => Some(InputEvent::Resize {
                size: Size {
                    width: cols as u16,
                    height: rows as u16,
                },
            }),

            // Toss everything else
            _ => None,
        }
    }
}

/// An event triggered by input from the user. This is a simplified version of
/// [terminput::Event] that eliminates all the possible events that we don't
/// care about handling.
#[derive(Debug, PartialEq)]
pub enum InputEvent<Action> {
    /// Key pressed down or repeated
    Key {
        /// Key pressed
        code: KeyCode,
        /// Additional modifiers keys that are active
        modifiers: KeyModifiers,
        /// Mapped input action, if any. Most consumers just care about the
        /// action. The input code/modifiers are only useful to things like
        /// text boxes that need to capture all input.
        action: Option<Action>,
    },
    /// Left click
    Click { position: Position },
    /// Scroll up/down/left/right
    Scroll {
        direction: ScrollDirection,
        position: Position,
    },
    /// Pasta!!
    Paste,
    /// Terminal was resized
    Resize { size: Size },
}

/// A static mapping between values (of type `T`) and labels (strings). Used to
/// both stringify from and parse to `T`.
struct Mapping<'a, T: Copy>(&'a [(T, &'a [&'a str])]);

impl<'a, T: Copy> Mapping<'a, T> {
    /// Construct a new mapping
    const fn new(mapping: &'a [(T, &'a [&'a str])]) -> Self {
        Self(mapping)
    }

    /// Get a value by one of its labels
    fn get(&self, s: &str) -> Option<T> {
        for (value, strs) in self.0 {
            for other_string in *strs {
                if *other_string == s {
                    return Some(*value);
                }
            }
        }
        None
    }

    /// Get the label mapped to a value. If it has multiple labels, use the
    /// first. Return `None` if the value isn't in the map or has no labels
    fn get_label(&self, value: T) -> Option<&str>
    where
        T: Debug + PartialEq,
    {
        let (_, strings) = self.0.iter().find(|(v, _)| v == &value)?;
        strings.first().copied()
    }

    /// Get all available mapped strings
    fn all_strings(&self) -> impl Iterator<Item = &str> {
        self.0
            .iter()
            .flat_map(|(_, strings)| strings.iter().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;
    use rstest::rstest;
    use terminput::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

    /// Input action for unit tests
    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
    enum Action {
        Submit,
    }

    /// Helper to create a key event
    fn key_event(
        kind: KeyEventKind,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> Event {
        Event::Key(KeyEvent {
            kind,
            code,
            modifiers,
            state: KeyEventState::empty(),
        })
    }

    /// Helper to create a mouse event
    fn mouse_event(kind: MouseEventKind) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })
    }

    /// Test keyboard input events to `convert_event`
    #[rstest]
    #[case::key_down_mapped(
        key_event(KeyEventKind::Press, KeyCode::Enter, KeyModifiers::NONE),
        Some(InputEvent::Key {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            action: Some(Action::Submit),
        })
    )]
    #[case::key_down_unmapped(
        key_event(KeyEventKind::Press, KeyCode::Char('i'), KeyModifiers::NONE),
        Some(InputEvent::Key {
            code: KeyCode::Char('i'),
            modifiers: KeyModifiers::NONE,
            action: None,
        })
    )]
    #[case::key_down_bonus_modifiers(
        key_event(KeyEventKind::Press, KeyCode::Enter, KeyModifiers::SHIFT),
        Some(InputEvent::Key {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::SHIFT,
            action: None,
        })
    )]
    #[case::key_repeat_mapped(
        key_event(KeyEventKind::Repeat, KeyCode::Enter, KeyModifiers::NONE),
        Some(InputEvent::Key {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            action: Some(Action::Submit),
        })
    )]
    #[case::key_repeat_unmapped(
        key_event(
            KeyEventKind::Repeat,
            KeyCode::Char('i'),
            KeyModifiers::NONE
        ),
        Some(InputEvent::Key {
            code: KeyCode::Char('i'),
            modifiers: KeyModifiers::NONE,
            action: None,
        })
    )]
    #[case::mouse_up_left(
        mouse_event(MouseEventKind::Up(MouseButton::Left)),
        Some(InputEvent::Click { position: (0, 0).into() })
    )]
    #[case::mouse_scroll_up(
        mouse_event(MouseEventKind::Scroll(ScrollDirection::Up)),
        Some(InputEvent::Scroll {
            direction: ScrollDirection::Up,
            position: (0, 0).into(),
        })
    )]
    #[case::mouse_scroll_down(
        mouse_event(MouseEventKind::Scroll(ScrollDirection::Down)),
        Some(InputEvent::Scroll {
            direction: ScrollDirection::Down,
                position: (0, 0).into(),
        })
    )]
    #[case::mouse_scroll_left(
        mouse_event(MouseEventKind::Scroll(ScrollDirection::Left)),
        Some(InputEvent::Scroll {
            direction: ScrollDirection::Left,
                position: (0, 0).into(),
        })
    )]
    #[case::mouse_scroll_right(
        mouse_event(MouseEventKind::Scroll(ScrollDirection::Right)),
        Some(InputEvent::Scroll {
            direction: ScrollDirection::Right,
                position: (0, 0).into(),
        })
    )]
    #[case::paste(Event::Paste("hello!".into()), Some(InputEvent::Paste))]
    // All these events should *not* be handled
    #[case::key_release(
        key_event(KeyEventKind::Release, KeyCode::Enter, KeyModifiers::NONE),
        None
    )]
    #[case::kill_focus_gained(Event::FocusGained, None)]
    #[case::kill_focus_lost(Event::FocusLost, None)]
    #[case::key_release(
        key_event(KeyEventKind::Release, KeyCode::Enter, KeyModifiers::NONE),
        None
    )]
    #[case::mouse_down(
        mouse_event(MouseEventKind::Down(MouseButton::Left)),
        None
    )]
    #[case::mouse_drag(
        mouse_event(MouseEventKind::Drag(MouseButton::Left)),
        None
    )]
    #[case::mouse_move(mouse_event(MouseEventKind::Moved), None)]
    fn test_convert_event(
        #[case] event: Event,
        #[case] expected: Option<InputEvent<Action>>,
    ) {
        let engine = InputBindings::default();
        let actual = engine.convert_event(event.clone());
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case::whitespace_stripped(" w ", KeyCode::Char('w'))]
    #[case::f_key("f2", KeyCode::F(2))]
    #[case::tab("tab", KeyCode::Tab)]
    #[case::page_up("pgup", KeyCode::PageUp)]
    #[case::page_down("pgdn", KeyCode::PageDown)]
    #[case::caps_lock("capslock", KeyCode::CapsLock)]
    #[case::f_key_with_modifier("shift f2", KeyCombination {
        code: KeyCode::F(2),
        modifiers: KeyModifiers::SHIFT,
    })]
    // Bonus spaces!
    #[case::extra_whitespace("shift  f2", KeyCombination {
        code: KeyCode::F(2),
        modifiers: KeyModifiers::SHIFT,
    })]
    #[case::extra_extra_whitespace("shift   f2", KeyCombination {
        code: KeyCode::F(2),
        modifiers: KeyModifiers::SHIFT,
    })]
    #[case::all_modifiers("super hyper meta alt ctrl shift f2", KeyCombination {
        code: KeyCode::F(2),
        modifiers: KeyModifiers::all(),
    })]
    // Backward compatibility: crossterm translates shift+tab as a separate
    // keycode call backtab. We previously used crossterm directly in this crate
    // so we supported this
    #[case::backtab("backtab", KeyCombination {
        code: KeyCode::Tab,
        modifiers: KeyModifiers::SHIFT,
    })]
    #[case::backtab_modifiers("ctrl backtab", KeyCombination {
        code: KeyCode::Tab,
        modifiers: KeyModifiers::CTRL | KeyModifiers::SHIFT,
    })]
    fn test_parse_key_combination(
        #[case] input: &str,
        #[case] expected: impl Into<KeyCombination>,
    ) {
        assert_eq!(input.parse::<KeyCombination>().unwrap(), expected.into());
    }

    #[rstest]
    #[case::empty("", "Empty key combination")]
    #[case::whitespace_only("  ", "Empty key combination")]
    #[case::invalid_delimiter("shift+w", "Invalid key code")]
    #[case::modifier_last("w shift", "Invalid key code")]
    #[case::invalid_modifier("shart w", "Invalid key modifier \"shart\"")]
    #[case::modifier_only("shift", "Invalid key code \"shift\"")]
    #[case::duplicate_modifier("alt alt w", "Duplicate modifier")]
    fn test_parse_key_combination_error(
        #[case] input: &str,
        #[case] expected_error: &str,
    ) {
        assert_err!(input.parse::<KeyCombination>(), expected_error);
    }

    #[rstest]
    #[case::char_only("g", KeyCode::Char('g'), KeyModifiers::NONE, true)]
    #[case::extra_modifier("g", KeyCode::Char('G'), KeyModifiers::SHIFT, false)]
    // Terminal may report the key code as caps if shift is pressed
    #[case::caps_input(
        "shift g",
        KeyCode::Char('G'),
        KeyModifiers::SHIFT,
        true
    )]
    #[case::caps_binding(
        "shift G",
        KeyCode::Char('g'),
        KeyModifiers::SHIFT,
        true
    )]
    #[case::multiple_modifiers(
        "ctrl shift end",
        KeyCode::End,
        KeyModifiers::CTRL | KeyModifiers::SHIFT,
        true,
    )]
    #[case::missing_modifier(
        "ctrl shift end",
        KeyCode::End,
        KeyModifiers::SHIFT,
        false
    )]
    fn test_key_combination_matches(
        #[case] combination: &str,
        #[case] code: KeyCode,
        #[case] modifiers: KeyModifiers,
        #[case] match_expected: bool,
    ) {
        let combination: KeyCombination = combination.parse().unwrap();
        let event = KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        assert_eq!(combination.matches(&event), match_expected);
    }

    /// Test stringifying/parsing key codes
    #[test]
    fn test_key_code() {
        // Build an iter of all codes
        let codes = [
            KeyCode::Backspace,
            KeyCode::Enter,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
            KeyCode::Tab,
            KeyCode::Delete,
            KeyCode::Insert,
            // Intentionally omitting Null (what is it??)
            KeyCode::Esc,
            KeyCode::CapsLock,
            KeyCode::ScrollLock,
            KeyCode::NumLock,
            KeyCode::PrintScreen,
            KeyCode::Pause,
            KeyCode::Menu,
            KeyCode::KeypadBegin,
        ]
        .into_iter()
        // F keys
        .chain((1..=12).map(KeyCode::F))
        // Chars (ASCII only)
        .chain((32..=126).map(|c| KeyCode::Char(char::from_u32(c).unwrap())))
        // Media keys
        .chain(
            [
                MediaKeyCode::Play,
                MediaKeyCode::Pause,
                MediaKeyCode::PlayPause,
                MediaKeyCode::Reverse,
                MediaKeyCode::Stop,
                MediaKeyCode::FastForward,
                MediaKeyCode::Rewind,
                MediaKeyCode::TrackNext,
                MediaKeyCode::TrackPrevious,
                MediaKeyCode::Record,
                MediaKeyCode::LowerVolume,
                MediaKeyCode::RaiseVolume,
                MediaKeyCode::MuteVolume,
            ]
            .into_iter()
            .map(KeyCode::Media),
        );
        // Intentionally ignore modifier key codes, they're treated separately

        // Round trip should get us in the same spot
        for code in codes {
            let s = stringify_key_code(code);
            let parsed = parse_key_code(&s).unwrap();
            assert_eq!(code, parsed, "code parse mismatch");
        }
    }

    /// Test stringifying/parsing each key modifier
    #[test]
    fn test_key_modifier() {
        // Round trip should get us in the same spot
        for modifier in KeyModifiers::all() {
            let s = stringify_key_modifier(modifier);
            let parsed = parse_key_modifier(&s).unwrap();
            assert_eq!(modifier, parsed, "modifier parse mismatch");
        }
    }

    /// Test that errors are forward correctly through deserialization, and
    /// that string/lists are both supported
    #[test]
    fn test_deserialize_input_binding() {
        assert_eq!(
            deserialize_yaml::<InputBinding>(vec!["f2", "f3"].into()).unwrap(),
            InputBinding(vec![KeyCode::F(2).into(), KeyCode::F(3).into()])
        );

        assert_err!(
            deserialize_yaml::<InputBinding>(vec!["no"].into())
                .map_err(LocatedError::into_error),
            "Invalid key code \"no\""
        );
        assert_err!(
            deserialize_yaml::<InputBinding>(vec!["shart f2"].into())
                .map_err(LocatedError::into_error),
            "Invalid key modifier \"shart\"; must be one of \
             [\"shift\", \"alt\", \"ctrl\", \"super\", \"hyper\", \"meta\"]"
        );
        assert_err!(
            deserialize_yaml::<InputBinding>(vec!["f2", "cortl f3"].into())
                .map_err(LocatedError::into_error),
            "Invalid key modifier \"cortl\"; must be one of \
            [\"shift\", \"alt\", \"ctrl\", \"super\", \"hyper\", \"meta\"]"
        );
        assert_err!(
            deserialize_yaml::<InputBinding>("f3".into())
                .map_err(LocatedError::into_error),
            "Expected sequence, received \"f3\""
        );
        assert_err!(
            deserialize_yaml::<InputBinding>(3.into())
                .map_err(LocatedError::into_error),
            "Expected sequence, received `3`"
        );
    }

    /// Test that user-provided bindings take priority
    #[rstest]
    #[case::user_binding(
        Action::Submit,
        KeyCode::Char('w'),
        KeyCode::Char('w'),
        Some(Action::Submit)
    )]
    #[case::default_not_available(
        Action::Submit,
        KeyCode::Tab,
        KeyCode::Enter,
        None
    )]
    #[case::unbound(Action::Submit, InputBinding(vec![]), KeyCode::Enter, None)]
    fn test_user_bindings(
        #[case] action: Action,
        #[case] binding: impl Into<InputBinding>,
        #[case] pressed: KeyCode,
        #[case] expected: Option<Action>,
    ) {
        let engine = InputMap::new(indexmap! {action => binding.into()});
        let event = KeyEvent {
            code: pressed,
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::empty(),
        };
        let actual = engine
            .iter()
            .find_map(|(action, binding)| {
                // Find the action mapped to the mocked event
                if binding.matches(&event) {
                    Some(action)
                } else {
                    None
                }
            })
            .copied();
        assert_eq!(actual, expected);
    }
}

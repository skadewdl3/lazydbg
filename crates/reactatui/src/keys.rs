use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A single parsed `"ctrl+shift+p"`-style spec, ready to be checked against
/// a live `KeyEvent`. Parsing happens once per `on_when` registration (i.e.
/// once per frame, same cost as any other hook), not once per keystroke.
#[derive(Debug, Clone, Copy)]
pub struct ParsedKeySpec {
    modifiers: KeyModifiers,
    code: KeyCode,
    /// True when this spec is "just shift + one printable-ish key" (a bare
    /// letter or Tab). Terminals disagree on how they report that
    /// combination, so these get lenient matching. Combined with any other
    /// modifier (e.g. ctrl+shift+p), we fall back to exact matching instead.
    shifted: bool,
}

/// A chorded sequence of key specs, e.g. `"ctrl+k-ctrl+s"` (press ctrl+k,
/// then ctrl+s). A plain `"ctrl+s"` parses to a single-step chord, so this
/// subsumes the non-chorded case.
#[derive(Debug, Clone)]
pub struct ParsedChord {
    pub steps: Vec<ParsedKeySpec>,
}

pub fn parse_chord_spec(spec: &str) -> ParsedChord {
    let steps: Vec<ParsedKeySpec> = spec
        .split('-')
        .map(|step| parse_key_spec(step.trim()))
        .collect();
    assert!(
        !steps.is_empty(),
        "keybindings!: empty chord spec \"{spec}\""
    );
    ParsedChord { steps }
}

pub fn parse_key_spec(spec: &str) -> ParsedKeySpec {
    let parts: Vec<&str> = spec.split('+').map(str::trim).collect();
    let (mod_parts, key_part) = parts.split_at(parts.len().saturating_sub(1));
    let key_part = key_part.first().copied().unwrap_or_default();

    let mut modifiers = KeyModifiers::NONE;
    for m in mod_parts {
        modifiers |= match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => KeyModifiers::CONTROL,
            "shift" => KeyModifiers::SHIFT,
            "alt" | "opt" | "option" => KeyModifiers::ALT,
            "super" | "cmd" | "command" | "meta" | "win" => KeyModifiers::SUPER,
            other => panic!("keybindings!: unknown modifier `{other}` in \"{spec}\""),
        };
    }

    let code = parse_keycode(key_part, spec);
    let shifted =
        modifiers == KeyModifiers::SHIFT && matches!(code, KeyCode::Char(_) | KeyCode::Tab);

    ParsedKeySpec {
        modifiers,
        code,
        shifted,
    }
}

fn parse_keycode(key: &str, spec: &str) -> KeyCode {
    let lower = key.to_ascii_lowercase();
    match lower.as_str() {
        "esc" | "escape" => KeyCode::Esc,
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "backspace" | "bs" => KeyCode::Backspace,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "space" => KeyCode::Char(' '),
        "null" => KeyCode::Null,
        "minus" | "hyphen" => KeyCode::Char('-'),
        "plus" => KeyCode::Char('+'),
        _ if lower.starts_with('f') && lower[1..].parse::<u8>().is_ok() => {
            KeyCode::F(lower[1..].parse().unwrap())
        }
        _ if key.chars().count() == 1 => {
            KeyCode::Char(key.chars().next().unwrap().to_ascii_lowercase())
        }
        _ => panic!("keybindings!: unrecognized key `{key}` in \"{spec}\""),
    }
}

impl ParsedKeySpec {
    pub fn matches(&self, event: &KeyEvent) -> bool {
        if self.shifted {
            match self.code {
                // Shift+Tab: some terminals send BackTab, others Tab+SHIFT.
                KeyCode::Tab => {
                    event.code == KeyCode::BackTab
                        || (event.code == KeyCode::Tab
                            && event.modifiers.contains(KeyModifiers::SHIFT))
                }
                // Shift+letter: some terminals send the uppercase char with
                // no modifier flag, others the lowercase char + SHIFT.
                KeyCode::Char(c) => {
                    event.code == KeyCode::Char(c.to_ascii_uppercase())
                        || (event.code == KeyCode::Char(c)
                            && event.modifiers.contains(KeyModifiers::SHIFT))
                }
                _ => unreachable!(),
            }
        } else if self.code == KeyCode::BackTab {
            // A bare "backtab" spec: ignore modifiers entirely, since
            // terminals vary on whether SHIFT rides along with it.
            event.code == KeyCode::BackTab
        } else {
            event.code == self.code && event.modifiers == self.modifiers
        }
    }
}

/// Declarative keybinding registration:
///
/// ```ignore
/// keybindings!(keys, {
///     "tab" => move || Pane::next(),
///     "shift+tab" | "backtab" => move || Pane::prev(),
///     "esc" => move || app.close(),
///     "q" => move || app.quit(),
///     "shift+q" => move || app.force_quit(),
///     "ctrl+c" => move || app.quit(),
/// });
/// ```
///
/// Each arm's handler is a zero-arg `FnMut()`, matching `KeyHandle::on`'s
/// convention — if you need the raw `KeyEvent`, use `keys.on_any`/`on_when`
/// directly instead.
#[macro_export]
macro_rules! keybindings {
    ($keys:expr, { $($arms:tt)* }) => {
        $crate::keybindings!(@arm $keys, $($arms)*);
    };

    (@arm $keys:expr $(,)?) => {};

    // Guarded catch-all:
    // key(k) if <condition using &KeyEvent> => handler
    (@arm $keys:expr,
        key($k:ident) if $guard:expr => $handler:expr
        $(, $($rest:tt)*)?
    ) => {
        $keys.on_when(
            move |$k: &::ratatui::crossterm::event::KeyEvent| {
                $guard
            },
            {
                let mut __h = $handler;

                move |__evt: ::ratatui::crossterm::event::KeyEvent|
                    -> ::reactatui::hooks::Propagation
                {
                    __h(__evt)
                }
            },
        );

        $crate::keybindings!(@arm $keys, $($($rest)*)?);
    };

    // Literal key specs
    (@arm $keys:expr,
        $($pat:literal)|+ => $handler:expr
        $(, $($rest:tt)*)?
    ) => {
        {
            let mut __h = $handler;
            let __chords: Vec<Vec<$crate::keys::ParsedKeySpec>> = vec![
                $($crate::keys::parse_chord_spec($pat).steps),+
            ];

            let all_single = __chords.iter().all(|c| c.len() == 1);

            if all_single {
                $keys.on_when(
                    move |event: &::ratatui::crossterm::event::KeyEvent| {
                        $(
                            if $crate::keys::parse_chord_spec($pat).steps[0].matches(event) {
                                return true;
                            }
                        )+
                        false
                    },
                    move |_event: ::ratatui::crossterm::event::KeyEvent| -> ::reactatui::hooks::Propagation {
                        __h();
                        ::reactatui::hooks::Propagation::Stop
                    }
                );
            } else {
                $keys.on_chord(
                    __chords,
                    move || -> ::reactatui::hooks::Propagation {
                        __h();
                        ::reactatui::hooks::Propagation::Stop
                    }
                );
            }
        }
        $crate::keybindings!(@arm $keys, $($($rest)*)?);
    };
}

// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;
use serde::de;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    pub const fn plain(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub const fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    pub fn from_key_event(key: &KeyEvent) -> Self {
        // Mask off SHIFT for char keys since crossterm includes it for uppercase
        let modifiers = match key.code {
            KeyCode::Char(_) => key.modifiers & !KeyModifiers::SHIFT,
            _ => key.modifiers,
        };
        Self {
            code: key.code,
            modifiers,
        }
    }
}

impl FromStr for KeyCombo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if let Some(rest) = s.strip_prefix("ctrl+") {
            let code = parse_key_code(rest)?;
            Ok(Self::ctrl(code))
        } else {
            let code = parse_key_code(&s)?;
            Ok(Self::plain(code))
        }
    }
}

fn parse_key_code(s: &str) -> Result<KeyCode, String> {
    match s {
        "esc" | "escape" => Ok(KeyCode::Esc),
        "enter" | "return" => Ok(KeyCode::Enter),
        "tab" => Ok(KeyCode::Tab),
        "backspace" => Ok(KeyCode::Backspace),
        "delete" | "del" => Ok(KeyCode::Delete),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" => Ok(KeyCode::PageUp),
        "pagedown" => Ok(KeyCode::PageDown),
        s if s.len() == 1 => Ok(KeyCode::Char(s.chars().next().unwrap())),
        _ => Err(format!("unknown key: {s}")),
    }
}

impl<'de> Deserialize<'de> for KeyCombo {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

// Custom deserializer for HashMap<KeyCombo, ...> since JSON keys must be strings
pub fn deserialize_keycombo_map<'de, D, V>(
    deserializer: D,
) -> Result<HashMap<KeyCombo, V>, D::Error>
where
    D: de::Deserializer<'de>,
    V: Deserialize<'de>,
{
    let map: HashMap<String, V> = HashMap::deserialize(deserializer)?;
    map.into_iter()
        .map(|(k, v)| {
            let combo: KeyCombo = k.parse().map_err(de::Error::custom)?;
            Ok((combo, v))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Command {
    Quit,
    Reload,
    SelectNext,
    SelectPrev,
    ToggleDetail,
    ToggleFocus,
    ScrollDown,
    ScrollUp,
    ScrollLeft,
    ScrollRight,
    PageDown,
    PageUp,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Commands {
    One(Command),
    Many(Vec<Command>),
}

impl Commands {
    pub fn as_slice(&self) -> &[Command] {
        match self {
            Commands::One(cmd) => std::slice::from_ref(cmd),
            Commands::Many(cmds) => cmds,
        }
    }
}

impl From<Command> for Commands {
    fn from(cmd: Command) -> Self {
        Commands::One(cmd)
    }
}

impl From<Vec<Command>> for Commands {
    fn from(cmds: Vec<Command>) -> Self {
        Commands::Many(cmds)
    }
}

pub struct Keymap {
    bindings: HashMap<KeyCombo, Commands>,
}

impl Keymap {
    pub fn lookup(&self, key: &KeyEvent) -> Option<&[Command]> {
        let combo = KeyCombo::from_key_event(key);
        self.bindings.get(&combo).map(Commands::as_slice)
    }

    pub fn merge(&mut self, overrides: HashMap<KeyCombo, Commands>) {
        self.bindings.extend(overrides);
    }

    pub fn qwerty() -> Self {
        Self::from_static(QWERTY_BINDINGS)
    }

    pub fn colemak_dh() -> Self {
        Self::from_static(COLEMAK_DH_BINDINGS)
    }

    fn from_static(entries: &[(KeyCombo, &[Command])]) -> Self {
        let bindings = entries
            .iter()
            .map(|(combo, cmds)| (*combo, Commands::Many(cmds.to_vec())))
            .collect();
        Self { bindings }
    }
}

impl fmt::Debug for Keymap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keymap")
            .field("bindings_count", &self.bindings.len())
            .finish()
    }
}

// Generated at compile time by build.rs
include!(concat!(env!("OUT_DIR"), "/keymaps.rs"));

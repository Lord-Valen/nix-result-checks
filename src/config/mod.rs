// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

pub mod keymap;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub use keymap::{Command, Keymap};
use keymap::{Commands, KeyCombo, deserialize_keycombo_map};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PresetName {
    #[default]
    Qwerty,
    ColemakDh,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub preset: Option<PresetName>,
    #[serde(default, deserialize_with = "deserialize_keycombo_map")]
    pub bindings: HashMap<KeyCombo, Commands>,
}

impl Config {
    /// Load config from the first file found in precedence order:
    /// 1. explicit --config path
    /// 2. .config/nrc.json (project-local dotfile)
    /// 3. ./nrc.json (project root)
    /// 4. `dirs::config_dir()/nrc/keymap.json` (XDG/platform)
    pub fn load(explicit: Option<&Path>) -> anyhow::Result<Option<Self>> {
        let candidates: Vec<PathBuf> = if let Some(path) = explicit {
            vec![path.to_path_buf()]
        } else {
            let mut paths = vec![PathBuf::from(".config/nrc.json"), PathBuf::from("nrc.json")];
            if let Some(config_dir) = dirs::config_dir() {
                paths.push(config_dir.join("nrc").join("keymap.json"));
            }
            paths
        };

        for path in &candidates {
            if path.is_file() {
                let content = fs::read_to_string(path)?;
                let config: Config = serde_json::from_str(&content)?;
                return Ok(Some(config));
            }
        }

        Ok(None)
    }

    /// Build the final Keymap from config and CLI overrides.
    /// Precedence: CLI --keymap > config preset > default (qwerty).
    /// Config bindings are merged on top of the resolved preset.
    pub fn resolve(config: Option<Self>, cli_keymap: Option<PresetName>) -> Keymap {
        let preset = cli_keymap
            .or_else(|| config.as_ref().and_then(|c| c.preset))
            .unwrap_or_default();

        let mut keymap = match preset {
            PresetName::Qwerty => Keymap::qwerty(),
            PresetName::ColemakDh => Keymap::colemak_dh(),
        };

        if let Some(config) = config.filter(|c| !c.bindings.is_empty()) {
            keymap.merge(config.bindings);
        }

        keymap
    }
}

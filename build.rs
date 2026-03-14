// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT-0

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("keymaps.rs");

    let keymaps_dir = Path::new("keymaps");

    println!("cargo::rerun-if-changed=keymaps/");

    let qwerty = load_keymap(&keymaps_dir.join("qwerty.json"));
    let colemak_dh = load_keymap(&keymaps_dir.join("colemak-dh.json"));

    let mut output = String::new();
    output.push_str(&generate_static("QWERTY_BINDINGS", &qwerty));
    output.push_str(&generate_static("COLEMAK_DH_BINDINGS", &colemak_dh));

    fs::write(&dest, output).unwrap();
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RawCommands {
    One(String),
    Many(Vec<String>),
}

impl RawCommands {
    fn as_vec(&self) -> Vec<&str> {
        match self {
            RawCommands::One(s) => vec![s.as_str()],
            RawCommands::Many(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

fn load_keymap(path: &Path) -> Vec<(ParsedCombo, Vec<String>)> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let raw: HashMap<String, RawCommands> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));

    let mut entries: Vec<(ParsedCombo, Vec<String>)> = Vec::new();
    for (key_str, cmds) in &raw {
        let combo = parse_key_combo(key_str, path);
        let commands: Vec<String> = cmds
            .as_vec()
            .iter()
            .map(|c| {
                validate_command(c, path);
                c.to_string()
            })
            .collect();
        entries.push((combo, commands));
    }
    // Sort for deterministic output
    entries.sort_by(|a, b| a.0.rust_expr().cmp(&b.0.rust_expr()));
    entries
}

struct ParsedCombo {
    code_expr: String,
    modifier_expr: String,
}

impl ParsedCombo {
    fn rust_expr(&self) -> String {
        format!(
            "KeyCombo {{ code: {}, modifiers: {} }}",
            self.code_expr, self.modifier_expr
        )
    }
}

fn parse_key_combo(s: &str, path: &Path) -> ParsedCombo {
    let s_lower = s.to_lowercase();
    let (modifier_expr, key_part) = if let Some(rest) = s_lower.strip_prefix("ctrl+") {
        ("KeyModifiers::CONTROL".to_string(), rest.to_string())
    } else {
        ("KeyModifiers::NONE".to_string(), s_lower)
    };

    let code_expr = match key_part.as_str() {
        "esc" | "escape" => "KeyCode::Esc".to_string(),
        "enter" | "return" => "KeyCode::Enter".to_string(),
        "tab" => "KeyCode::Tab".to_string(),
        "backspace" => "KeyCode::Backspace".to_string(),
        "delete" | "del" => "KeyCode::Delete".to_string(),
        "up" => "KeyCode::Up".to_string(),
        "down" => "KeyCode::Down".to_string(),
        "left" => "KeyCode::Left".to_string(),
        "right" => "KeyCode::Right".to_string(),
        "home" => "KeyCode::Home".to_string(),
        "end" => "KeyCode::End".to_string(),
        "pageup" => "KeyCode::PageUp".to_string(),
        "pagedown" => "KeyCode::PageDown".to_string(),
        s if s.len() == 1 => {
            format!("KeyCode::Char('{}')", s.chars().next().unwrap())
        }
        _ => panic!("unknown key '{}' in {}", s, path.display()),
    };

    ParsedCombo {
        code_expr,
        modifier_expr,
    }
}

const VALID_COMMANDS: &[&str] = &[
    "Quit",
    "Reload",
    "SelectNext",
    "SelectPrev",
    "ToggleDetail",
    "ToggleFocus",
    "ScrollDown",
    "ScrollUp",
    "ScrollLeft",
    "ScrollRight",
    "PageDown",
    "PageUp",
];

fn validate_command(cmd: &str, path: &Path) {
    if !VALID_COMMANDS.contains(&cmd) {
        panic!("unknown command '{}' in {}", cmd, path.display());
    }
}

fn generate_static(name: &str, entries: &[(ParsedCombo, Vec<String>)]) -> String {
    let mut out = String::new();
    out.push_str(&format!("static {name}: &[(KeyCombo, &[Command])] = &[\n"));
    for (combo, commands) in entries {
        let cmds_str: Vec<String> = commands.iter().map(|c| format!("Command::{c}")).collect();
        out.push_str(&format!(
            "    ({}, &[{}]),\n",
            combo.rust_expr(),
            cmds_str.join(", ")
        ));
    }
    out.push_str("];\n\n");
    out
}

// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Fail,
    Skip,
}

impl Status {
    pub fn symbol(&self) -> &'static str {
        match self {
            Status::Pass => "✓",
            Status::Fail => "✗",
            Status::Skip => "·",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Status::Pass => Color::Green,
            Status::Fail => Color::Red,
            Status::Skip => Color::DarkGray,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckEntry {
    pub name: String,
    pub status: Status,
    pub kind: EntryKind,
    #[serde(rename = "exitCode")]
    pub exit_code: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    Result,
    Snapshot,
}

pub struct App {
    pub order: Vec<String>,
    pub entries: HashMap<String, CheckEntry>,
    generation: u64,
    entry_generations: HashMap<String, u64>,
}

impl App {
    pub fn new() -> Self {
        Self {
            order: Vec::new(),
            entries: HashMap::new(),
            generation: 0,
            entry_generations: HashMap::new(),
        }
    }

    pub fn bump_generation(&mut self) {
        self.generation += 1;
    }

    pub fn upsert(&mut self, entry: CheckEntry) {
        if !self.entries.contains_key(&entry.name) {
            self.order.push(entry.name.clone());
        }
        self.entry_generations
            .insert(entry.name.clone(), self.generation);
        self.entries.insert(entry.name.clone(), entry);
    }

    pub fn prune(&mut self) {
        let current = self.generation;
        self.entries
            .retain(|name, _| self.entry_generations.get(name).copied().unwrap_or(0) >= current);
        self.entry_generations.retain(|_, g| *g >= current);
        self.order.retain(|name| self.entries.contains_key(name));
    }
}

// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::collections::{HashMap, HashSet};

use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
pub struct CheckEntry {
    pub name: String,
    pub status: Status,
    pub kind: EntryKind,
    #[serde(rename = "exitCode")]
    pub exit_code: String,
    pub stdout: String,
    pub stderr: String,
    #[serde(default)]
    pub suite: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    Result,
    Snapshot,
    Eval,
}

pub fn entry_key(suite: Option<&str>, name: &str) -> String {
    match suite {
        Some(s) => format!("{s}:{name}"),
        None => name.to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderItem {
    Suite { name: String, checks: Vec<String> },
    Check(String),
}

impl OrderItem {
    fn suite_name(&self) -> Option<&str> {
        match self {
            OrderItem::Suite { name, .. } => Some(name),
            OrderItem::Check(_) => None,
        }
    }
}

pub enum VisibleItem {
    Suite(String),
    Check(String),
}

pub struct App {
    pub order: Vec<OrderItem>,
    pub entries: HashMap<String, CheckEntry>,
    pub folded_suites: HashSet<String>,
    generation: u64,
    entry_generations: HashMap<String, u64>,
}

impl App {
    pub fn new() -> Self {
        Self {
            order: Vec::new(),
            entries: HashMap::new(),
            folded_suites: HashSet::new(),
            generation: 0,
            entry_generations: HashMap::new(),
        }
    }

    pub fn bump_generation(&mut self) {
        self.generation += 1;
    }

    pub fn upsert(&mut self, entry: CheckEntry) {
        let key = entry_key(entry.suite.as_deref(), &entry.name);
        if !self.entries.contains_key(&key) {
            match &entry.suite {
                None => {
                    self.order.push(OrderItem::Check(key.clone()));
                }
                Some(suite_name) => {
                    let suite_name = suite_name.clone();
                    if let Some(item) = self
                        .order
                        .iter_mut()
                        .find(|i| i.suite_name() == Some(suite_name.as_str()))
                    {
                        if let OrderItem::Suite { checks, .. } = item {
                            checks.push(key.clone());
                        }
                    } else {
                        self.order.push(OrderItem::Suite {
                            name: suite_name,
                            checks: vec![key.clone()],
                        });
                    }
                }
            }
        }
        self.entry_generations.insert(key.clone(), self.generation);
        self.entries.insert(key, entry);
    }

    pub fn prune(&mut self) {
        let current = self.generation;
        self.entry_generations.retain(|_, g| *g >= current);
        let live: HashSet<String> = self.entry_generations.keys().cloned().collect();
        self.entries.retain(|key, _| live.contains(key.as_str()));
        for item in &mut self.order {
            if let OrderItem::Suite { checks, .. } = item {
                checks.retain(|k| live.contains(k.as_str()));
            }
        }
        self.order.retain(|item| match item {
            OrderItem::Check(key) => live.contains(key.as_str()),
            OrderItem::Suite { checks, .. } => !checks.is_empty(),
        });
        let live_suites: HashSet<&str> = self
            .order
            .iter()
            .filter_map(|item| match item {
                OrderItem::Suite { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        self.folded_suites
            .retain(|name| live_suites.contains(name.as_str()));
    }

    pub fn all_keys(&self) -> impl Iterator<Item = &String> {
        self.order.iter().flat_map(|item| {
            let slice: &[String] = match item {
                OrderItem::Check(key) => std::slice::from_ref(key),
                OrderItem::Suite { checks, .. } => checks.as_slice(),
            };
            slice.iter()
        })
    }

    pub fn visible_items(&self) -> Vec<VisibleItem> {
        let mut items = Vec::new();
        for order_item in &self.order {
            match order_item {
                OrderItem::Check(key) => items.push(VisibleItem::Check(key.clone())),
                OrderItem::Suite { name, checks } => {
                    items.push(VisibleItem::Suite(name.clone()));
                    if !self.folded_suites.contains(name) {
                        for key in checks {
                            items.push(VisibleItem::Check(key.clone()));
                        }
                    }
                }
            }
        }
        items
    }

    pub fn toggle_suite(&mut self, name: &str) {
        if self.folded_suites.contains(name) {
            self.folded_suites.remove(name);
        } else {
            self.folded_suites.insert(name.to_owned());
        }
    }
}

#[cfg(test)]
mod tests;

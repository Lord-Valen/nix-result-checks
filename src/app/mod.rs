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
    /// Nested entries surfaced by the check itself (e.g. a snapshot's
    /// wrapped result, named `"actual"`). Flattened into the app's own
    /// entries on upsert, grouped under this check the same way a suite
    /// groups its members — the key is this check's own key.
    #[serde(default)]
    pub children: Vec<CheckEntry>,
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
    /// A check row. `depth` is the indent level: 0 for a flat top-level
    /// check, 1 for a suite member or a top-level check's direct child,
    /// and one more per further nesting level of `children`.
    Check {
        key: String,
        depth: usize,
    },
}

pub struct App {
    pub order: Vec<OrderItem>,
    pub entries: HashMap<String, CheckEntry>,
    /// Direct children of a check, by the check's own key. Populated by
    /// flattening `CheckEntry::children` on upsert; unrelated to suite
    /// membership, which stays exactly one flat level (`order`/`suite`).
    pub child_keys: HashMap<String, Vec<String>>,
    pub folded_suites: HashSet<String>,
    /// Check keys whose children are hidden. Separate from
    /// `folded_suites` — a check with children starts folded by default
    /// (children are inspection detail, not the primary list), whereas a
    /// suite starts unfolded.
    pub folded_checks: HashSet<String>,
    generation: u64,
    entry_generations: HashMap<String, u64>,
}

impl App {
    pub fn new() -> Self {
        Self {
            order: Vec::new(),
            entries: HashMap::new(),
            child_keys: HashMap::new(),
            folded_suites: HashSet::new(),
            folded_checks: HashSet::new(),
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
        self.insert_with_children(key, entry);
    }

    /// Stores `entry` under `key` and recursively does the same for its
    /// `children`, linking each into `child_keys[key]`. Shared by `upsert`
    /// (which additionally places `key` in `order`) and by children
    /// themselves, which never get their own `order` placement.
    fn insert_with_children(&mut self, key: String, mut entry: CheckEntry) {
        let children = std::mem::take(&mut entry.children);
        let is_new = !self.entries.contains_key(&key);
        self.entry_generations.insert(key.clone(), self.generation);
        self.entries.insert(key.clone(), entry);

        let child_keys: Vec<String> = children
            .into_iter()
            .map(|child| {
                let child_key = entry_key(Some(&key), &child.name);
                self.insert_with_children(child_key.clone(), child);
                child_key
            })
            .collect();
        if child_keys.is_empty() {
            self.child_keys.remove(&key);
        } else {
            if is_new {
                self.folded_checks.insert(key.clone());
            }
            self.child_keys.insert(key, child_keys);
        }
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
        self.child_keys.retain(|key, _| live.contains(key.as_str()));
        for children in self.child_keys.values_mut() {
            children.retain(|k| live.contains(k.as_str()));
        }
        self.folded_checks
            .retain(|key| self.child_keys.contains_key(key.as_str()));
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
        let mut keys = Vec::new();
        for item in &self.order {
            match item {
                OrderItem::Check(key) => {
                    keys.push(key);
                    self.push_descendant_keys(key, &mut keys);
                }
                OrderItem::Suite { checks, .. } => {
                    for key in checks {
                        keys.push(key);
                        self.push_descendant_keys(key, &mut keys);
                    }
                }
            }
        }
        keys.into_iter()
    }

    fn push_descendant_keys<'a>(&'a self, key: &str, keys: &mut Vec<&'a String>) {
        if let Some(children) = self.child_keys.get(key) {
            for child in children {
                keys.push(child);
                self.push_descendant_keys(child, keys);
            }
        }
    }

    pub fn visible_items(&self) -> Vec<VisibleItem> {
        let mut items = Vec::new();
        for order_item in &self.order {
            match order_item {
                OrderItem::Check(key) => {
                    items.push(VisibleItem::Check {
                        key: key.clone(),
                        depth: 0,
                    });
                    self.push_visible_children(key, 1, &mut items);
                }
                OrderItem::Suite { name, checks } => {
                    items.push(VisibleItem::Suite(name.clone()));
                    if !self.folded_suites.contains(name) {
                        for key in checks {
                            items.push(VisibleItem::Check {
                                key: key.clone(),
                                depth: 1,
                            });
                            self.push_visible_children(key, 2, &mut items);
                        }
                    }
                }
            }
        }
        items
    }

    fn push_visible_children(&self, key: &str, depth: usize, items: &mut Vec<VisibleItem>) {
        if self.folded_checks.contains(key) {
            return;
        }
        let Some(children) = self.child_keys.get(key) else {
            return;
        };
        for child_key in children {
            items.push(VisibleItem::Check {
                key: child_key.clone(),
                depth,
            });
            self.push_visible_children(child_key, depth + 1, items);
        }
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        self.count_keys(self.all_keys())
    }

    pub fn suite_counts(&self, name: &str) -> (usize, usize, usize) {
        let keys = self.order.iter().find_map(|item| match item {
            OrderItem::Suite { name: n, checks } if n == name => Some(checks.as_slice()),
            _ => None,
        });
        self.count_keys(keys.unwrap_or(&[]).iter())
    }

    fn count_keys<'a>(&self, keys: impl Iterator<Item = &'a String>) -> (usize, usize, usize) {
        let (mut pass, mut fail, mut skip) = (0, 0, 0);
        for key in keys {
            match self.entries.get(key).map(|e| &e.status) {
                Some(Status::Pass) => pass += 1,
                Some(Status::Fail) => fail += 1,
                Some(Status::Skip) => skip += 1,
                None => {}
            }
        }
        (pass, fail, skip)
    }

    pub fn toggle_suite(&mut self, name: &str) {
        if self.folded_suites.contains(name) {
            self.folded_suites.remove(name);
        } else {
            self.folded_suites.insert(name.to_owned());
        }
    }

    pub fn toggle_children(&mut self, key: &str) {
        if self.folded_checks.contains(key) {
            self.folded_checks.remove(key);
        } else {
            self.folded_checks.insert(key.to_owned());
        }
    }
}

#[cfg(test)]
mod tests;

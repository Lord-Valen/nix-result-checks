// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use ratatui::widgets::ListState;

use crate::app::{App, VisibleItem};
use crate::config::Command;

/// State of the check list panel: which entry is selected and, via
/// [`ListState`]'s offset, which rows are currently scrolled into view.
/// The offset is persisted across frames so the viewport only scrolls
/// when the selection would otherwise move off screen.
#[derive(Debug, Default)]
pub struct ListView {
    pub state: ListState,
}

/// Outcome of a `ListView::execute` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwimOutcome {
    /// Folded/unfolded a suite or check in place; selection didn't move.
    Toggled,
    /// Selection moved to a different row.
    Moved,
    /// Nothing to fold, toggle, or navigate to.
    Unhandled,
}

impl DwimOutcome {
    fn from_moved(moved: bool) -> Self {
        if moved { Self::Moved } else { Self::Unhandled }
    }

    fn from_toggled(toggled: bool) -> Self {
        if toggled {
            Self::Toggled
        } else {
            Self::Unhandled
        }
    }
}

impl ListView {
    /// Handles the subset of `Command`s that mean list navigation/folding:
    /// `SelectNext`/`SelectPrev`/`NextSuite`/`PrevSuite`/`ToggleSuite`/
    /// `RightDwim`/`LeftDwim`. Anything else is `Unhandled` — the caller
    /// owns those commands instead.
    pub fn execute(&mut self, cmd: Command, app: &mut App) -> DwimOutcome {
        match cmd {
            Command::SelectNext => DwimOutcome::from_moved(self.select_next(app)),
            Command::SelectPrev => DwimOutcome::from_moved(self.select_prev(app)),
            Command::NextSuite => DwimOutcome::from_moved(self.select_next_suite(app)),
            Command::PrevSuite => DwimOutcome::from_moved(self.select_prev_suite(app)),
            Command::ToggleSuite => DwimOutcome::from_toggled(self.toggle_suite(app)),
            Command::RightDwim => self.right_dwim(app),
            Command::LeftDwim => self.left_dwim(app),
            _ => DwimOutcome::Unhandled,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    fn set_selected(&mut self, selected: Option<usize>) {
        *self.state.selected_mut() = selected;
    }

    /// Moves the selection to the next entry. Returns `false` if the list
    /// is empty and no selection change was made.
    pub fn select_next(&mut self, app: &App) -> bool {
        let visible = app.visible_items();
        if visible.is_empty() {
            return false;
        }
        self.set_selected(Some(match self.selected() {
            None => 0,
            Some(i) => (i + 1).min(visible.len() - 1),
        }));
        true
    }

    /// Moves the selection to the previous entry. Returns `false` if the
    /// list is empty and no selection change was made.
    pub fn select_prev(&mut self, app: &App) -> bool {
        let visible = app.visible_items();
        if visible.is_empty() {
            return false;
        }
        self.set_selected(Some(match self.selected() {
            None => 0,
            Some(i) => i.saturating_sub(1),
        }));
        true
    }

    /// Jumps to the next suite header. Returns `false` if there is none.
    pub fn select_next_suite(&mut self, app: &App) -> bool {
        let visible = app.visible_items();
        let start = self.selected().map_or(0, |i| i + 1);
        let Some(idx) = visible[start..]
            .iter()
            .position(|v| matches!(v, VisibleItem::Suite(_)))
        else {
            return false;
        };
        self.set_selected(Some(start + idx));
        true
    }

    /// Jumps to the previous suite header. Returns `false` if there is none.
    pub fn select_prev_suite(&mut self, app: &App) -> bool {
        let visible = app.visible_items();
        let end = self.selected().unwrap_or(0);
        let Some(idx) = visible[..end]
            .iter()
            .rposition(|v| matches!(v, VisibleItem::Suite(_)))
        else {
            return false;
        };
        self.set_selected(Some(idx));
        true
    }

    /// Folds/unfolds the selected suite. Returns `false` if the selected
    /// row isn't a suite header.
    pub fn toggle_suite(&self, app: &mut App) -> bool {
        let Some(idx) = self.selected() else {
            return false;
        };
        let visible = app.visible_items();
        let Some(VisibleItem::Suite(name)) = visible.get(idx) else {
            return false;
        };
        let name = name.clone();
        app.toggle_suite(&name);
        true
    }

    /// Unfolds the selected suite/check if folded; otherwise moves to its
    /// first child (the next row, since `visible_items` is depth-first).
    pub fn right_dwim(&mut self, app: &mut App) -> DwimOutcome {
        let Some(idx) = self.selected() else {
            return DwimOutcome::Unhandled;
        };
        let visible = app.visible_items();
        let Some(item) = visible.get(idx) else {
            return DwimOutcome::Unhandled;
        };
        match item {
            VisibleItem::Suite(name) if app.folded_suites.contains(name) => {
                app.toggle_suite(&name.clone());
                DwimOutcome::Toggled
            }
            VisibleItem::Check { key, .. }
                if app.child_keys.contains_key(key) && app.folded_checks.contains(key) =>
            {
                app.toggle_children(&key.clone());
                DwimOutcome::Toggled
            }
            _ if idx + 1 < visible.len() && depth_of(item) < depth_of(&visible[idx + 1]) => {
                self.set_selected(Some(idx + 1));
                DwimOutcome::Moved
            }
            _ => DwimOutcome::Unhandled,
        }
    }

    /// Folds the selected suite/check if unfolded; otherwise moves to its
    /// parent (the nearest preceding row at a shallower depth).
    pub fn left_dwim(&mut self, app: &mut App) -> DwimOutcome {
        let Some(idx) = self.selected() else {
            return DwimOutcome::Unhandled;
        };
        let visible = app.visible_items();
        let Some(item) = visible.get(idx) else {
            return DwimOutcome::Unhandled;
        };
        match item {
            VisibleItem::Suite(name) if !app.folded_suites.contains(name) => {
                app.toggle_suite(&name.clone());
                DwimOutcome::Toggled
            }
            VisibleItem::Check { key, .. }
                if app.child_keys.contains_key(key) && !app.folded_checks.contains(key) =>
            {
                app.toggle_children(&key.clone());
                DwimOutcome::Toggled
            }
            _ => {
                let cur_depth = depth_of(item);
                match (0..idx).rev().find(|&i| depth_of(&visible[i]) < cur_depth) {
                    Some(parent_idx) => {
                        self.set_selected(Some(parent_idx));
                        DwimOutcome::Moved
                    }
                    None => DwimOutcome::Unhandled,
                }
            }
        }
    }
}

/// Ordering depth used by `right_dwim`/`left_dwim` to find a parent or
/// first child in the depth-first `visible_items` list. `None` (a suite
/// header) sorts shallower than any check row, including flat top-level
/// checks, via `Option`'s derived `Ord`.
fn depth_of(item: &VisibleItem) -> Option<usize> {
    match item {
        VisibleItem::Suite(_) => None,
        VisibleItem::Check { depth, .. } => Some(*depth),
    }
}

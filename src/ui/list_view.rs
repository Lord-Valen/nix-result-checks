// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use ratatui::widgets::ListState;

use crate::app::{App, VisibleItem};

/// State of the check list panel: which entry is selected and, via
/// [`ListState`]'s offset, which rows are currently scrolled into view.
/// The offset is persisted across frames so the viewport only scrolls
/// when the selection would otherwise move off screen.
#[derive(Debug, Default)]
pub struct ListView {
    pub state: ListState,
}

impl ListView {
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
}

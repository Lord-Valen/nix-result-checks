// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use crate::app::{App, VisibleItem};
use crate::render::PanelBounds;
use crate::ui::clamp_u16;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DetailFocus {
    #[default]
    Stdout,
    Stderr,
}

impl DetailFocus {
    fn toggle(&self) -> Self {
        match self {
            DetailFocus::Stdout => DetailFocus::Stderr,
            DetailFocus::Stderr => DetailFocus::Stdout,
        }
    }
}

fn content_maxes(content: &str, bounds: PanelBounds) -> (u16, u16) {
    let lines = if content.is_empty() {
        1
    } else {
        content.lines().count()
    };
    let max_w = content
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    (
        clamp_u16(lines.saturating_sub(bounds.height)),
        clamp_u16(max_w.saturating_sub(bounds.width)),
    )
}

/// State of the detail panel: which check it's showing and the scroll
/// position of its stdout/stderr sub-panels.
#[derive(Debug, Default)]
pub struct DetailView {
    /// The entry key of the check shown in the detail panel. Updated when
    /// navigation lands on a check; unchanged when navigating to a suite
    /// header so the panel keeps showing the last-viewed check.
    pub key: Option<String>,
    pub open: bool,
    pub focus: DetailFocus,
    pub stdout_scroll: u16,
    pub stdout_h_scroll: u16,
    pub stderr_scroll: u16,
    pub stderr_h_scroll: u16,
    pub stdout_bounds: PanelBounds,
    pub stderr_bounds: PanelBounds,
}

impl DetailView {
    /// Updates `key` to the check at `idx`, resolving a suite header to its
    /// first check. Leaves `key` unchanged if `idx` doesn't resolve to one.
    pub fn sync_selection(&mut self, app: &App, idx: usize) {
        match app.visible_items().get(idx) {
            Some(VisibleItem::Check { key, .. }) => {
                self.key = Some(key.clone());
            }
            Some(VisibleItem::Suite(name)) => {
                let first = app.order.iter().find_map(|item| match item {
                    crate::app::OrderItem::Suite { name: n, checks } if n == name => {
                        checks.first().cloned()
                    }
                    _ => None,
                });
                if let Some(key) = first {
                    self.key = Some(key);
                }
            }
            None => {}
        }
    }

    /// Toggles the panel open/closed. Returns `false` if there's no check
    /// to show yet.
    pub fn toggle(&mut self) -> bool {
        if self.key.is_none() {
            return false;
        }
        self.open = !self.open;
        self.reset_scrolls();
        true
    }

    /// Toggles stdout/stderr focus. Returns `false` if the panel is closed.
    pub fn toggle_focus(&mut self) -> bool {
        if !self.open {
            return false;
        }
        self.focus = self.focus.toggle();
        true
    }

    pub fn set_panel_bounds(&mut self, stdout: PanelBounds, stderr: PanelBounds, app: &App) {
        self.stdout_bounds = stdout;
        self.stderr_bounds = stderr;
        self.clamp_scrolls(app);
    }

    /// Scrolls the focused sub-panel vertically. Returns `false` if the
    /// panel is closed.
    pub fn scroll_v(&mut self, app: &App, op: fn(u16, u16) -> u16, amount: u16) -> bool {
        if !self.open {
            return false;
        }
        let s = self.focused_scroll_mut();
        *s = op(*s, amount);
        self.clamp_scrolls(app);
        true
    }

    /// Scrolls the focused sub-panel horizontally. Returns `false` if the
    /// panel is closed.
    pub fn scroll_h(&mut self, app: &App, op: fn(u16, u16) -> u16, amount: u16) -> bool {
        if !self.open {
            return false;
        }
        let s = self.focused_h_scroll_mut();
        *s = op(*s, amount);
        self.clamp_scrolls(app);
        true
    }

    pub fn nudge_scroll_down(&mut self, app: &App) {
        let s = self.focused_scroll_mut();
        *s = s.saturating_add(1);
        self.clamp_scrolls(app);
    }

    pub fn nudge_scroll_up(&mut self) {
        let s = self.focused_scroll_mut();
        *s = s.saturating_sub(1);
    }

    fn clamp_scrolls(&mut self, app: &App) {
        let Some(key) = &self.key else { return };
        let Some(entry) = app.entries.get(key) else {
            return;
        };

        let (sv, sh) = content_maxes(&entry.stdout, self.stdout_bounds);
        let (ev, eh) = content_maxes(&entry.stderr, self.stderr_bounds);

        self.stdout_scroll = self.stdout_scroll.min(sv);
        self.stdout_h_scroll = self.stdout_h_scroll.min(sh);
        self.stderr_scroll = self.stderr_scroll.min(ev);
        self.stderr_h_scroll = self.stderr_h_scroll.min(eh);
    }

    fn focused_scroll_mut(&mut self) -> &mut u16 {
        match self.focus {
            DetailFocus::Stdout => &mut self.stdout_scroll,
            DetailFocus::Stderr => &mut self.stderr_scroll,
        }
    }

    fn focused_h_scroll_mut(&mut self) -> &mut u16 {
        match self.focus {
            DetailFocus::Stdout => &mut self.stdout_h_scroll,
            DetailFocus::Stderr => &mut self.stderr_h_scroll,
        }
    }

    pub fn reset_scrolls(&mut self) {
        self.stdout_scroll = 0;
        self.stdout_h_scroll = 0;
        self.stderr_scroll = 0;
        self.stderr_h_scroll = 0;
        self.focus = DetailFocus::Stdout;
    }
}

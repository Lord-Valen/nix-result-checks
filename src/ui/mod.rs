// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

mod detail_view;
mod list_view;

use std::sync::mpsc;

use ratatui::crossterm::event::MouseEventKind;

pub use detail_view::DetailFocus;
use detail_view::DetailView;
use list_view::ListView;

use crate::app::{App, VisibleItem};
use crate::config::{Command, Keymap};
use crate::event::Event;
use crate::render::PanelBounds;

/// Narrowing conversion from `usize` to `u16`, clamping at `u16::MAX`.
pub fn clamp_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// Top-level window state: which overlay is showing, background task
/// status, and the two panels' view state.
pub struct Ui {
    pub list: ListView,
    pub detail: DetailView,
    pub toast: Option<String>,
    pub show_help: bool,
    pub rebuilding: bool,
    pub watch_count: Option<usize>,
    tx: mpsc::Sender<Event>,
}

impl Ui {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        Self {
            list: ListView::default(),
            detail: DetailView::default(),
            toast: None,
            show_help: false,
            rebuilding: true,
            watch_count: None,
            tx,
        }
    }

    pub fn set_panel_bounds(&mut self, stdout: PanelBounds, stderr: PanelBounds, app: &App) {
        self.detail.set_panel_bounds(stdout, stderr, app);
    }

    pub fn list_panel_width(app: &App) -> u16 {
        let check_width = app
            .all_keys()
            .filter_map(|k| app.entries.get(k))
            .map(|e| e.name.chars().count() + 6)
            .max()
            .unwrap_or(0);
        let suite_width = app
            .order
            .iter()
            .filter_map(|item| match item {
                crate::app::OrderItem::Suite { name, .. } => Some(name.chars().count() + 6),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        clamp_u16(check_width.max(suite_width).max(20))
    }

    pub fn handle(&mut self, event: &Event, app: &mut App, keymap: &Keymap) {
        match event {
            Event::Key(_) if self.toast.is_some() => {
                self.toast = None;
            }
            Event::Key(_) if self.show_help => {
                self.show_help = false;
            }
            Event::Key(key) => {
                if let Some(cmds) = keymap.lookup(key) {
                    self.dispatch_commands(cmds, app);
                }
            }
            Event::Mouse(mouse) => self.handle_mouse(*mouse, app),
            _ => {}
        }
    }

    fn handle_mouse(&mut self, mouse: ratatui::crossterm::event::MouseEvent, app: &mut App) {
        let over_detail = self.detail.open && mouse.column >= Self::list_panel_width(app);
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if over_detail {
                    self.detail.nudge_scroll_down(app);
                } else {
                    self.select_next(app);
                }
            }
            MouseEventKind::ScrollUp => {
                if over_detail {
                    self.detail.nudge_scroll_up();
                } else {
                    self.select_prev(app);
                }
            }
            _ => {}
        }
    }

    fn dispatch_commands(&mut self, commands: &[Command], app: &mut App) {
        for &cmd in commands {
            if self.execute(cmd, app) {
                return;
            }
        }
    }

    /// Execute a command. Returns `true` if the command was handled.
    pub fn execute(&mut self, cmd: Command, app: &mut App) -> bool {
        match cmd {
            Command::Quit => {
                let _ = self.tx.send(Event::Quit);
                true
            }
            Command::Reload => {
                let _ = self.tx.send(Event::Reload);
                true
            }
            Command::SelectNext => {
                self.select_next(app);
                true
            }
            Command::SelectPrev => {
                self.select_prev(app);
                true
            }
            Command::NextSuite => {
                self.select_next_suite(app);
                true
            }
            Command::PrevSuite => {
                self.select_prev_suite(app);
                true
            }
            Command::Dwim => {
                let visible = app.visible_items();
                match self.list.selected().and_then(|i| visible.get(i)) {
                    Some(VisibleItem::Suite(_)) => self.execute(Command::ToggleSuite, app),
                    Some(VisibleItem::Check { .. }) => self.execute(Command::ToggleDetail, app),
                    None => false,
                }
            }
            Command::ToggleSuite => {
                self.toggle_suite(app);
                true
            }
            Command::ToggleDetail => self.detail.toggle(),
            Command::ToggleFocus => self.detail.toggle_focus(),
            Command::ScrollLeft => self.detail.scroll_h(app, u16::saturating_sub, 1),
            Command::ScrollRight => self.detail.scroll_h(app, u16::saturating_add, 1),
            Command::ScrollDown => self.detail.scroll_v(app, u16::saturating_add, 1),
            Command::ScrollUp => self.detail.scroll_v(app, u16::saturating_sub, 1),
            Command::PageDown => self.detail.scroll_v(app, u16::saturating_add, 10),
            Command::PageUp => self.detail.scroll_v(app, u16::saturating_sub, 10),
            Command::ShowHelp => {
                self.show_help = !self.show_help;
                true
            }
        }
    }

    /// Syncs the detail panel to the current selection and resets its
    /// scroll position, as happens after any navigation.
    fn after_selection_change(&mut self, app: &App) {
        if let Some(idx) = self.list.selected() {
            self.detail.sync_selection(app, idx);
        }
        self.detail.reset_scrolls();
    }

    fn select_next(&mut self, app: &App) {
        if self.list.select_next(app) {
            self.after_selection_change(app);
        }
    }

    fn select_prev(&mut self, app: &App) {
        if self.list.select_prev(app) {
            self.after_selection_change(app);
        }
    }

    fn select_next_suite(&mut self, app: &App) {
        if self.list.select_next_suite(app) {
            self.after_selection_change(app);
        }
    }

    fn select_prev_suite(&mut self, app: &App) {
        if self.list.select_prev_suite(app) {
            self.after_selection_change(app);
        }
    }

    fn toggle_suite(&mut self, app: &mut App) {
        let Some(idx) = self.list.selected() else {
            return;
        };
        let visible = app.visible_items();
        if let Some(VisibleItem::Suite(name)) = visible.get(idx) {
            let name = name.clone();
            app.toggle_suite(&name);
        }
    }
}

#[cfg(test)]
mod tests;

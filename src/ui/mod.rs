// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::sync::mpsc;

use ratatui::crossterm::event::MouseEventKind;

use crate::app::{App, VisibleItem};
use crate::config::{Command, Keymap};
use crate::event::Event;
use crate::render::PanelBounds;

/// Narrowing conversion from `usize` to `u16`, clamping at `u16::MAX`.
pub fn clamp_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailFocus {
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

pub struct Ui {
    pub selected: Option<usize>,
    /// The entry key of the check shown in the detail panel. Updated when
    /// navigation lands on a check; unchanged when navigating to a suite header
    /// so the panel keeps showing the last-viewed check.
    pub detail_key: Option<String>,
    pub detail_open: bool,
    pub detail_focus: DetailFocus,
    pub stdout_scroll: u16,
    pub stdout_h_scroll: u16,
    pub stderr_scroll: u16,
    pub stderr_h_scroll: u16,
    pub toast: Option<String>,
    pub show_help: bool,
    pub rebuilding: bool,
    pub watch_count: Option<usize>,
    pub stdout_bounds: PanelBounds,
    pub stderr_bounds: PanelBounds,
    tx: mpsc::Sender<Event>,
}

impl Ui {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        Self {
            selected: None,
            detail_key: None,
            detail_open: false,
            detail_focus: DetailFocus::Stdout,
            stdout_scroll: 0,
            stdout_h_scroll: 0,
            stderr_scroll: 0,
            stderr_h_scroll: 0,
            toast: None,
            show_help: false,
            rebuilding: true,
            watch_count: None,
            stdout_bounds: PanelBounds::default(),
            stderr_bounds: PanelBounds::default(),
            tx,
        }
    }

    pub fn set_panel_bounds(&mut self, stdout: PanelBounds, stderr: PanelBounds, app: &App) {
        self.stdout_bounds = stdout;
        self.stderr_bounds = stderr;
        self.clamp_scrolls(app);
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
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if self.detail_open && mouse.column >= Self::list_panel_width(app) {
                    let s = self.focused_scroll_mut();
                    *s = s.saturating_add(1);
                    self.clamp_scrolls(app);
                } else {
                    self.select_next(app);
                }
            }
            MouseEventKind::ScrollUp => {
                if self.detail_open && mouse.column >= Self::list_panel_width(app) {
                    let s = self.focused_scroll_mut();
                    *s = s.saturating_sub(1);
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
                match self.selected.and_then(|i| visible.get(i)) {
                    Some(VisibleItem::Suite(_)) => self.execute(Command::ToggleSuite, app),
                    Some(VisibleItem::Check(_)) => self.execute(Command::ToggleDetail, app),
                    None => false,
                }
            }
            Command::ToggleSuite => {
                self.toggle_suite(app);
                true
            }
            Command::ToggleDetail => {
                if self.detail_key.is_some() {
                    self.detail_open = !self.detail_open;
                    self.reset_scrolls();
                    true
                } else {
                    false
                }
            }
            Command::ToggleFocus => {
                if self.detail_open {
                    self.detail_focus = self.detail_focus.toggle();
                    true
                } else {
                    false
                }
            }
            Command::ScrollLeft => self.scroll_h(app, u16::saturating_sub, 1),
            Command::ScrollRight => self.scroll_h(app, u16::saturating_add, 1),
            Command::ScrollDown => self.scroll_v(app, u16::saturating_add, 1),
            Command::ScrollUp => self.scroll_v(app, u16::saturating_sub, 1),
            Command::PageDown => self.scroll_v(app, u16::saturating_add, 10),
            Command::PageUp => self.scroll_v(app, u16::saturating_sub, 10),
            Command::ShowHelp => {
                self.show_help = !self.show_help;
                true
            }
        }
    }

    fn scroll_v(&mut self, app: &App, op: fn(u16, u16) -> u16, amount: u16) -> bool {
        if self.detail_open {
            let s = self.focused_scroll_mut();
            *s = op(*s, amount);
            self.clamp_scrolls(app);
            true
        } else {
            false
        }
    }

    fn scroll_h(&mut self, app: &App, op: fn(u16, u16) -> u16, amount: u16) -> bool {
        if self.detail_open {
            let s = self.focused_h_scroll_mut();
            *s = op(*s, amount);
            self.clamp_scrolls(app);
            true
        } else {
            false
        }
    }

    fn clamp_scrolls(&mut self, app: &App) {
        let Some(key) = &self.detail_key else { return };
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
        match self.detail_focus {
            DetailFocus::Stdout => &mut self.stdout_scroll,
            DetailFocus::Stderr => &mut self.stderr_scroll,
        }
    }

    fn focused_h_scroll_mut(&mut self) -> &mut u16 {
        match self.detail_focus {
            DetailFocus::Stdout => &mut self.stdout_h_scroll,
            DetailFocus::Stderr => &mut self.stderr_h_scroll,
        }
    }

    fn reset_scrolls(&mut self) {
        self.stdout_scroll = 0;
        self.stdout_h_scroll = 0;
        self.stderr_scroll = 0;
        self.stderr_h_scroll = 0;
        self.detail_focus = DetailFocus::Stdout;
    }

    fn update_detail_key(&mut self, app: &App) {
        let Some(idx) = self.selected else { return };
        match app.visible_items().get(idx) {
            Some(VisibleItem::Check(key)) => {
                self.detail_key = Some(key.clone());
            }
            Some(VisibleItem::Suite(name)) => {
                let first = app.order.iter().find_map(|item| match item {
                    crate::app::OrderItem::Suite { name: n, checks } if n == name => {
                        checks.first().cloned()
                    }
                    _ => None,
                });
                if let Some(key) = first {
                    self.detail_key = Some(key);
                }
            }
            None => {}
        }
    }

    fn select_next(&mut self, app: &App) {
        let visible = app.visible_items();
        if visible.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(i) => (i + 1).min(visible.len() - 1),
        });
        self.update_detail_key(app);
        self.reset_scrolls();
    }

    fn select_prev(&mut self, app: &App) {
        let visible = app.visible_items();
        if visible.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(i) => i.saturating_sub(1),
        });
        self.update_detail_key(app);
        self.reset_scrolls();
    }

    fn select_next_suite(&mut self, app: &App) {
        let visible = app.visible_items();
        let start = self.selected.map_or(0, |i| i + 1);
        if let Some(idx) = visible[start..]
            .iter()
            .position(|v| matches!(v, VisibleItem::Suite(_)))
        {
            self.selected = Some(start + idx);
            self.update_detail_key(app);
            self.reset_scrolls();
        }
    }

    fn select_prev_suite(&mut self, app: &App) {
        let visible = app.visible_items();
        let end = self.selected.unwrap_or(0);
        if let Some(idx) = visible[..end]
            .iter()
            .rposition(|v| matches!(v, VisibleItem::Suite(_)))
        {
            self.selected = Some(idx);
            self.update_detail_key(app);
            self.reset_scrolls();
        }
    }

    fn toggle_suite(&mut self, app: &mut App) {
        let Some(idx) = self.selected else { return };
        let visible = app.visible_items();
        if let Some(VisibleItem::Suite(name)) = visible.get(idx) {
            let name = name.clone();
            app.toggle_suite(&name);
        }
    }
}

#[cfg(test)]
mod tests;

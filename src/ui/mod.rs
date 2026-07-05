// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

mod detail_view;
mod list_view;

use std::sync::mpsc;

use ratatui::crossterm::event::MouseEventKind;

pub use detail_view::DetailFocus;
use detail_view::DetailView;
use list_view::{DwimOutcome, ListView};

use crate::app::{App, VisibleItem};
use crate::config::{Command, Keymap};
use crate::event::Event;
use crate::render::PanelBounds;

/// Narrowing conversion from `usize` to `u16`, clamping at `u16::MAX`.
pub fn clamp_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// Which side of the UI keyboard input targets. While `Detail`, Left/Right
/// dwim don't fold or navigate the list — they're reserved for scrolling
/// the detail panel, so a check's own (possibly long) output stays
/// reachable even when the check has children of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    List,
    Detail,
}

/// Top-level window state: which overlay is showing, background task
/// status, and the two panels' view state.
pub struct Ui {
    pub list: ListView,
    pub detail: DetailView,
    pub pane: Pane,
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
            pane: Pane::default(),
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
    ///
    /// `Ui` just tracks which pane has focus and forwards: the six
    /// commands that mean two different things depending on focus
    /// (list navigation vs. detail scrolling) go to whichever of
    /// `self.list`/`self.detail` owns that meaning; everything else is
    /// pane-independent and forwards straight to `self.detail` (or is
    /// truly `Ui`'s own — quitting, reloading, overlay toggles, the
    /// pane switch itself).
    pub fn execute(&mut self, cmd: Command, app: &mut App) -> bool {
        match cmd {
            Command::SelectNext
            | Command::SelectPrev
            | Command::NextSuite
            | Command::PrevSuite
            | Command::RightDwim
            | Command::LeftDwim => match self.pane {
                Pane::List => {
                    let outcome = self.list.execute(cmd, app);
                    self.apply_dwim_outcome(outcome, app)
                }
                Pane::Detail => self.execute_in_detail(cmd, app),
            },
            Command::Quit => {
                let _ = self.tx.send(Event::Quit);
                true
            }
            Command::Reload => {
                let _ = self.tx.send(Event::Reload);
                true
            }
            Command::ToggleDwim => {
                let visible = app.visible_items();
                match self.list.selected().and_then(|i| visible.get(i)) {
                    Some(VisibleItem::Suite(_)) => self.execute(Command::ToggleSuite, app),
                    Some(VisibleItem::Check { .. }) => self.execute(Command::ToggleDetail, app),
                    None => false,
                }
            }
            Command::ToggleSuite => self.list.toggle_suite(app),
            // Deliberately doesn't touch `pane`: opening the panel with
            // 'd' is a quick peek, not a request to move keyboard focus —
            // otherwise it'd hijack Up/Down away from list navigation, and
            // walking down the list with j/k while glancing at each
            // check's detail (a common flow) would break after the first
            // press. Only Tab/TogglePane and the Left/Right dead-end
            // fallback (OpenDetail) actually switch pane.
            Command::OpenDetail => {
                let handled = self.detail.execute(cmd, app);
                if handled {
                    self.pane = Pane::Detail;
                }
                handled
            }
            Command::TogglePane => match self.pane {
                Pane::List => {
                    if self.detail.execute(Command::OpenDetail, app) {
                        self.pane = Pane::Detail;
                        true
                    } else {
                        false
                    }
                }
                Pane::Detail => {
                    self.pane = Pane::List;
                    true
                }
            },
            Command::ShowHelp => {
                self.show_help = !self.show_help;
                true
            }
            // ToggleDetail, ToggleFocus, Scroll*, Page*: pure detail-panel
            // business with no pane bookkeeping — forward directly.
            _ => self.detail.execute(cmd, app),
        }
    }

    /// Syncs the detail panel and pane when a fold/navigate attempt moved
    /// the selection; a fold/unfold with no move needs neither.
    fn apply_dwim_outcome(&mut self, outcome: DwimOutcome, app: &App) -> bool {
        match outcome {
            DwimOutcome::Moved => {
                self.after_selection_change(app);
                true
            }
            DwimOutcome::Toggled => true,
            DwimOutcome::Unhandled => false,
        }
    }

    /// Detail-pane meaning of the six focus-dependent commands: scroll
    /// its content instead (`DetailView::execute` handles that
    /// translation). Left, once already scrolled fully left, is the one
    /// exception handled here: it exits back to the list pane and
    /// performs that pane's normal fold-or-move-to-parent action in the
    /// same press, so backing out never costs an extra keypress — a
    /// cross-view transition neither view can decide on its own.
    fn execute_in_detail(&mut self, cmd: Command, app: &mut App) -> bool {
        if cmd == Command::LeftDwim && self.detail.focused_h_scroll() == 0 {
            self.pane = Pane::List;
            let outcome = self.list.execute(Command::LeftDwim, app);
            return self.apply_dwim_outcome(outcome, app);
        }
        self.detail.execute(cmd, app)
    }

    /// Syncs the detail panel to the current selection and resets its
    /// scroll position, as happens after any navigation. Also returns
    /// focus to the list — navigating away from what you were looking at
    /// implies you're browsing again, not still inspecting it.
    fn after_selection_change(&mut self, app: &App) {
        if let Some(idx) = self.list.selected() {
            self.detail.sync_selection(app, idx);
        }
        self.detail.reset_scrolls();
        self.pane = Pane::List;
    }

    /// Moves list selection down, used directly by the mouse wheel
    /// handler, which decides list vs. detail by pointer position rather
    /// than `pane`.
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
}

#[cfg(test)]
mod tests;

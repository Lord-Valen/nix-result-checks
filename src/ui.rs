// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::sync::mpsc;

use ratatui::crossterm::event::MouseEventKind;

use crate::app::App;
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
    pub detail_open: bool,
    pub detail_focus: DetailFocus,
    pub stdout_scroll: u16,
    pub stdout_h_scroll: u16,
    pub stderr_scroll: u16,
    pub stderr_h_scroll: u16,
    pub toast: Option<String>,
    pub rebuilding: bool,
    pub watch_count: Option<usize>,
    stdout_bounds: PanelBounds,
    stderr_bounds: PanelBounds,
    tx: mpsc::Sender<Event>,
}

impl Ui {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        Self {
            selected: None,
            detail_open: false,
            detail_focus: DetailFocus::Stdout,
            stdout_scroll: 0,
            stdout_h_scroll: 0,
            stderr_scroll: 0,
            stderr_h_scroll: 0,
            toast: None,
            rebuilding: false,
            watch_count: None,
            stdout_bounds: PanelBounds::default(),
            stderr_bounds: PanelBounds::default(),
            tx,
        }
    }

    /// Called after each render to update panel dimensions and clamp scrolls.
    pub fn set_panel_bounds(&mut self, stdout: PanelBounds, stderr: PanelBounds, app: &App) {
        self.stdout_bounds = stdout;
        self.stderr_bounds = stderr;
        self.clamp_scrolls(app);
    }

    pub fn list_panel_width(app: &App) -> u16 {
        clamp_u16(
            app.order
                .iter()
                .filter_map(|n| app.entries.get(n))
                .map(|e| e.name.chars().count() + 6)
                .max()
                .unwrap_or(20)
                .max(20),
        )
    }

    pub fn handle(&mut self, event: &Event, app: &App, keymap: &Keymap) {
        match event {
            Event::Key(_) if self.toast.is_some() => {
                self.toast = None;
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

    fn handle_mouse(&mut self, mouse: ratatui::crossterm::event::MouseEvent, app: &App) {
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

    fn dispatch_commands(&mut self, commands: &[Command], app: &App) {
        for &cmd in commands {
            if self.execute(cmd, app) {
                return;
            }
        }
    }

    /// Execute a command. Returns `true` if the command was handled.
    fn execute(&mut self, cmd: Command, app: &App) -> bool {
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
            Command::ToggleDetail => {
                if self.selected.is_some() {
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
        let Some(idx) = self.selected else { return };
        let Some(name) = app.order.get(idx) else {
            return;
        };
        let Some(entry) = app.entries.get(name) else {
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

    fn select_next(&mut self, app: &App) {
        if app.order.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(i) => (i + 1).min(app.order.len() - 1),
        });
        self.reset_scrolls();
    }

    fn select_prev(&mut self, app: &App) {
        if app.order.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(i) => i.saturating_sub(1),
        });
        self.reset_scrolls();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::app::{App, CheckEntry, EntryKind, Status};
    use crate::config::{Command, Keymap};

    fn make_ui() -> (Ui, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        (Ui::new(tx), rx)
    }

    fn make_app(n: usize) -> App {
        let mut app = App::new();
        for i in 0..n {
            app.upsert(CheckEntry {
                name: format!("check-{i}"),
                status: Status::Pass,
                kind: EntryKind::Result,
                exit_code: "0".to_string(),
                stdout: String::new(),
                stderr: String::new(),
            });
        }
        app
    }

    fn key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    // -- Selection --

    #[test]
    fn select_next_from_none() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.execute(Command::SelectNext, &app);
        assert_eq!(ui.selected, Some(0));
    }

    #[test]
    fn select_next_advances() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        ui.execute(Command::SelectNext, &app);
        assert_eq!(ui.selected, Some(1));
    }

    #[test]
    fn select_next_clamps_at_end() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(2);
        ui.execute(Command::SelectNext, &app);
        assert_eq!(ui.selected, Some(2));
    }

    #[test]
    fn select_prev_from_none() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.execute(Command::SelectPrev, &app);
        assert_eq!(ui.selected, Some(0));
    }

    #[test]
    fn select_prev_clamps_at_start() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        ui.execute(Command::SelectPrev, &app);
        assert_eq!(ui.selected, Some(0));
    }

    #[test]
    fn select_next_noop_on_empty() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(0);
        ui.execute(Command::SelectNext, &app);
        assert_eq!(ui.selected, None);
    }

    // -- Toggle detail --

    #[test]
    fn toggle_detail_requires_selection() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        assert!(!ui.execute(Command::ToggleDetail, &app));
        assert!(!ui.detail_open);
    }

    #[test]
    fn toggle_detail_opens_with_selection() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        assert!(ui.execute(Command::ToggleDetail, &app));
        assert!(ui.detail_open);
    }

    #[test]
    fn toggle_detail_closes() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.execute(Command::ToggleDetail, &app);
        assert!(!ui.detail_open);
    }

    // -- Toggle focus --

    #[test]
    fn toggle_focus_requires_detail_open() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        assert!(!ui.execute(Command::ToggleFocus, &app));
        assert_eq!(ui.detail_focus, DetailFocus::Stdout);
    }

    #[test]
    fn toggle_focus_switches() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.execute(Command::ToggleFocus, &app);
        assert_eq!(ui.detail_focus, DetailFocus::Stderr);
    }

    // -- Scroll --

    #[test]
    fn scroll_noop_without_detail() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        assert!(!ui.execute(Command::ScrollDown, &app));
        assert_eq!(ui.stdout_scroll, 0);
    }

    #[test]
    fn scroll_down_increments() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.stdout_bounds = PanelBounds {
            height: 5,
            width: 80,
        };
        ui.execute(Command::ScrollDown, &app);
        // Clamped to 0 because content is empty (max scroll = 0)
        assert_eq!(ui.stdout_scroll, 0);
    }

    #[test]
    fn page_down_increments_by_10() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        // Give the entry enough content to scroll
        app.entries.get_mut("check-0").unwrap().stdout = "a\n".repeat(20);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.stdout_bounds = PanelBounds {
            height: 5,
            width: 80,
        };
        ui.execute(Command::PageDown, &app);
        assert_eq!(ui.stdout_scroll, 10);
    }

    // -- Quit and Reload send events --

    #[test]
    fn quit_sends_event() {
        let (mut ui, rx) = make_ui();
        let app = make_app(0);
        ui.execute(Command::Quit, &app);
        assert!(matches!(rx.try_recv(), Ok(Event::Quit)));
    }

    #[test]
    fn reload_sends_event() {
        let (mut ui, rx) = make_ui();
        let app = make_app(0);
        ui.execute(Command::Reload, &app);
        assert!(matches!(rx.try_recv(), Ok(Event::Reload)));
    }

    // -- Toast --

    #[test]
    fn any_key_dismisses_toast() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(0);
        let keymap = Keymap::qwerty();
        ui.toast = Some("error".to_string());
        ui.handle(&key_event(KeyCode::Char('x')), &app, &keymap);
        assert!(ui.toast.is_none());
    }

    #[test]
    fn toast_blocks_command() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        let keymap = Keymap::qwerty();
        ui.selected = Some(0);
        ui.toast = Some("error".to_string());
        ui.handle(&key_event(KeyCode::Char('j')), &app, &keymap);
        // j would normally SelectNext, but toast blocks it
        assert_eq!(ui.selected, Some(0));
    }

    // -- dispatch_commands: first applicable wins --

    #[test]
    fn dispatch_commands_stops_at_first_handled() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        // ScrollDown requires detail_open (false), SelectNext always works
        let cmds = [Command::ScrollDown, Command::SelectNext];
        ui.dispatch_commands(&cmds, &app);
        // ScrollDown was skipped, SelectNext applied
        assert_eq!(ui.selected, Some(1));
    }

    #[test]
    fn dispatch_commands_does_not_continue_after_handled() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        let cmds = [Command::SelectNext, Command::SelectNext];
        ui.dispatch_commands(&cmds, &app);
        // Only first SelectNext should fire
        assert_eq!(ui.selected, Some(1));
    }

    // -- Missing selection branch --

    #[test]
    fn select_prev_decrements() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(2);
        ui.execute(Command::SelectPrev, &app);
        assert_eq!(ui.selected, Some(1));
    }

    #[test]
    fn select_prev_noop_on_empty() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(0);
        ui.execute(Command::SelectPrev, &app);
        assert_eq!(ui.selected, None);
    }

    // -- Missing scroll commands --

    #[test]
    fn scroll_up_decrements() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        app.entries.get_mut("check-0").unwrap().stdout = "a\n".repeat(20);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.stdout_bounds = PanelBounds {
            height: 5,
            width: 80,
        };
        ui.stdout_scroll = 5;
        ui.execute(Command::ScrollUp, &app);
        assert_eq!(ui.stdout_scroll, 4);
    }

    #[test]
    fn scroll_up_noop_without_detail() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(1);
        assert!(!ui.execute(Command::ScrollUp, &app));
    }

    #[test]
    fn scroll_left_decrements() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        app.entries.get_mut("check-0").unwrap().stdout = "a".repeat(100);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.stdout_bounds = PanelBounds {
            height: 5,
            width: 20,
        };
        ui.stdout_h_scroll = 5;
        ui.execute(Command::ScrollLeft, &app);
        assert_eq!(ui.stdout_h_scroll, 4);
    }

    #[test]
    fn scroll_left_noop_without_detail() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(1);
        assert!(!ui.execute(Command::ScrollLeft, &app));
    }

    #[test]
    fn scroll_right_increments() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        app.entries.get_mut("check-0").unwrap().stdout = "a".repeat(100);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.stdout_bounds = PanelBounds {
            height: 5,
            width: 20,
        };
        ui.execute(Command::ScrollRight, &app);
        assert_eq!(ui.stdout_h_scroll, 1);
    }

    #[test]
    fn scroll_right_noop_without_detail() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(1);
        assert!(!ui.execute(Command::ScrollRight, &app));
    }

    #[test]
    fn page_up_decrements_by_10() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        app.entries.get_mut("check-0").unwrap().stdout = "a\n".repeat(30);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.stdout_bounds = PanelBounds {
            height: 5,
            width: 80,
        };
        ui.stdout_scroll = 15;
        ui.execute(Command::PageUp, &app);
        assert_eq!(ui.stdout_scroll, 5);
    }

    #[test]
    fn page_up_noop_without_detail() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(1);
        assert!(!ui.execute(Command::PageUp, &app));
    }

    #[test]
    fn page_down_noop_without_detail() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(1);
        assert!(!ui.execute(Command::PageDown, &app));
    }

    // -- Focus affects which scroll is modified --

    #[test]
    fn scroll_down_affects_stderr_when_focused() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        app.entries.get_mut("check-0").unwrap().stderr = "a\n".repeat(20);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.detail_focus = DetailFocus::Stderr;
        ui.stderr_bounds = PanelBounds {
            height: 5,
            width: 80,
        };
        ui.execute(Command::ScrollDown, &app);
        assert_eq!(ui.stderr_scroll, 1);
        assert_eq!(ui.stdout_scroll, 0);
    }

    #[test]
    fn scroll_right_affects_stderr_when_focused() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        app.entries.get_mut("check-0").unwrap().stderr = "a".repeat(100);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.detail_focus = DetailFocus::Stderr;
        ui.stderr_bounds = PanelBounds {
            height: 5,
            width: 20,
        };
        ui.execute(Command::ScrollRight, &app);
        assert_eq!(ui.stderr_h_scroll, 1);
        assert_eq!(ui.stdout_h_scroll, 0);
    }

    // -- Toggle focus both directions --

    #[test]
    fn toggle_focus_stderr_to_stdout() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.detail_focus = DetailFocus::Stderr;
        ui.execute(Command::ToggleFocus, &app);
        assert_eq!(ui.detail_focus, DetailFocus::Stdout);
    }

    // -- Unbound key --

    #[test]
    fn unbound_key_is_noop() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        let keymap = Keymap::qwerty();
        ui.selected = Some(0);
        // 'z' is not bound in qwerty
        ui.handle(&key_event(KeyCode::Char('z')), &app, &keymap);
        assert_eq!(ui.selected, Some(0));
    }

    // -- Mouse handling --

    fn mouse_event(kind: MouseEventKind, column: u16) -> Event {
        Event::Mouse(ratatui::crossterm::event::MouseEvent {
            kind,
            column,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn mouse_scroll_down_selects_next_on_list() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        let keymap = Keymap::qwerty();
        ui.selected = Some(0);
        ui.handle(&mouse_event(MouseEventKind::ScrollDown, 0), &app, &keymap);
        assert_eq!(ui.selected, Some(1));
    }

    #[test]
    fn mouse_scroll_up_selects_prev_on_list() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        let keymap = Keymap::qwerty();
        ui.selected = Some(1);
        ui.handle(&mouse_event(MouseEventKind::ScrollUp, 0), &app, &keymap);
        assert_eq!(ui.selected, Some(0));
    }

    #[test]
    fn mouse_scroll_down_scrolls_detail_panel() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        app.entries.get_mut("check-0").unwrap().stdout = "a\n".repeat(20);
        let keymap = Keymap::qwerty();
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.stdout_bounds = PanelBounds {
            height: 5,
            width: 80,
        };
        // Column beyond list panel width → hits detail area
        let col = Ui::list_panel_width(&app);
        ui.handle(&mouse_event(MouseEventKind::ScrollDown, col), &app, &keymap);
        assert_eq!(ui.stdout_scroll, 1);
    }

    #[test]
    fn mouse_scroll_up_scrolls_detail_panel() {
        let (mut ui, _rx) = make_ui();
        let mut app = make_app(1);
        app.entries.get_mut("check-0").unwrap().stdout = "a\n".repeat(20);
        let keymap = Keymap::qwerty();
        ui.selected = Some(0);
        ui.detail_open = true;
        ui.stdout_bounds = PanelBounds {
            height: 5,
            width: 80,
        };
        ui.stdout_scroll = 3;
        let col = Ui::list_panel_width(&app);
        ui.handle(&mouse_event(MouseEventKind::ScrollUp, col), &app, &keymap);
        assert_eq!(ui.stdout_scroll, 2);
    }

    #[test]
    fn mouse_click_is_noop() {
        let (mut ui, _rx) = make_ui();
        let app = make_app(3);
        let keymap = Keymap::qwerty();
        ui.selected = Some(0);
        ui.handle(
            &mouse_event(
                MouseEventKind::Down(ratatui::crossterm::event::MouseButton::Left),
                0,
            ),
            &app,
            &keymap,
        );
        assert_eq!(ui.selected, Some(0));
    }
}

// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::sync::mpsc;

use ratatui::crossterm::event::{KeyModifiers, MouseEventKind};

use super::*;
use crate::app::{App, CheckEntry, EntryKind, Status};
use crate::config::{Command, Keymap};
use crate::render::PanelBounds;

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
            suite: None,
            children: Vec::new(),
        });
    }
    app
}

fn make_suite_app() -> App {
    let mut app = App::new();
    for name in ["alpha", "beta"] {
        app.upsert(CheckEntry {
            name: name.to_string(),
            status: Status::Pass,
            kind: EntryKind::Result,
            exit_code: "0".to_string(),
            stdout: String::new(),
            stderr: String::new(),
            suite: Some("db".to_string()),
            children: Vec::new(),
        });
    }
    app
}

fn mouse_event(kind: MouseEventKind, column: u16) -> Event {
    Event::Mouse(ratatui::crossterm::event::MouseEvent {
        kind,
        column,
        row: 0,
        modifiers: KeyModifiers::NONE,
    })
}

#[test]
fn select_next_clamps_at_end() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(3);
    *ui.list.state.selected_mut() = Some(2);
    ui.execute(Command::SelectNext, &mut app);
    assert_eq!(ui.list.state.selected(), Some(2));
}

#[test]
fn select_prev_clamps_at_start() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(3);
    *ui.list.state.selected_mut() = Some(0);
    ui.execute(Command::SelectPrev, &mut app);
    assert_eq!(ui.list.state.selected(), Some(0));
}

// -- Dwim --

#[test]
fn dwim_on_suite_folds() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    ui.execute(Command::Dwim, &mut app);
    assert!(app.folded_suites.contains("db"));
}

#[test]
fn dwim_on_suite_folds_even_when_detail_open() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    ui.detail.key = Some("db:alpha".to_string());
    ui.detail.open = true;
    ui.execute(Command::Dwim, &mut app);
    assert!(app.folded_suites.contains("db"));
    assert!(ui.detail.open); // detail stays open — suite Dwim doesn't touch it
}

#[test]
fn dwim_on_check_opens_detail() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(1); // Check("db:alpha")
    ui.detail.key = Some("db:alpha".to_string());
    ui.execute(Command::Dwim, &mut app);
    assert!(ui.detail.open);
}

#[test]
fn dwim_on_check_closes_detail() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(1); // Check("db:alpha")
    ui.detail.key = Some("db:alpha".to_string());
    ui.detail.open = true;
    ui.execute(Command::Dwim, &mut app);
    assert!(!ui.detail.open);
}

// -- Toggle detail --

#[test]
fn toggle_detail_stays_open_when_suite_selected() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    // opened detail on a check, then navigated to suite header —
    // detail_key persists, so ToggleDetail still works
    ui.detail.key = Some("db:alpha".to_string());
    ui.detail.open = true;
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    assert!(ui.execute(Command::ToggleDetail, &mut app));
    assert!(!ui.detail.open);
}

// -- Page scroll --

#[test]
fn page_down_increments_by_10() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(1);
    app.entries.get_mut("check-0").unwrap().stdout = "a\n".repeat(20);
    *ui.list.state.selected_mut() = Some(0);
    ui.detail.key = Some("check-0".to_string());
    ui.detail.open = true;
    ui.detail.stdout_bounds = PanelBounds {
        height: 5,
        width: 80,
    };
    ui.execute(Command::PageDown, &mut app);
    assert_eq!(ui.detail.stdout_scroll, 10);
}

#[test]
fn page_up_decrements_by_10() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(1);
    app.entries.get_mut("check-0").unwrap().stdout = "a\n".repeat(30);
    *ui.list.state.selected_mut() = Some(0);
    ui.detail.open = true;
    ui.detail.stdout_bounds = PanelBounds {
        height: 5,
        width: 80,
    };
    ui.detail.stdout_scroll = 15;
    ui.execute(Command::PageUp, &mut app);
    assert_eq!(ui.detail.stdout_scroll, 5);
}

// -- Dispatch --

#[test]
fn dispatch_commands_does_not_continue_after_handled() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(3);
    *ui.list.state.selected_mut() = Some(0);
    let cmds = [Command::SelectNext, Command::SelectNext];
    ui.dispatch_commands(&cmds, &mut app);
    assert_eq!(ui.list.state.selected(), Some(1));
}

// -- Mouse --

#[test]
fn mouse_scroll_down_scrolls_detail_panel() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(1);
    app.entries.get_mut("check-0").unwrap().stdout = "a\n".repeat(20);
    let keymap = Keymap::qwerty();
    *ui.list.state.selected_mut() = Some(0);
    ui.detail.open = true;
    ui.detail.stdout_bounds = PanelBounds {
        height: 5,
        width: 80,
    };
    let col = Ui::list_panel_width(&app);
    ui.handle(
        &mouse_event(MouseEventKind::ScrollDown, col),
        &mut app,
        &keymap,
    );
    assert_eq!(ui.detail.stdout_scroll, 1);
}

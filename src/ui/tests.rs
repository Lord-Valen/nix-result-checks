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

fn make_app_with_children() -> App {
    let mut app = App::new();
    app.upsert(CheckEntry {
        name: "snap".to_string(),
        status: Status::Pass,
        kind: EntryKind::Snapshot,
        exit_code: "0".to_string(),
        stdout: String::new(),
        stderr: String::new(),
        suite: None,
        children: vec![CheckEntry {
            name: "actual".to_string(),
            status: Status::Pass,
            kind: EntryKind::Result,
            exit_code: "0".to_string(),
            stdout: String::new(),
            stderr: String::new(),
            suite: None,
            children: Vec::new(),
        }],
    });
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

#[test]
fn select_next_scrolls_detail_instead_of_moving_list_in_detail_pane() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(3);
    *ui.list.state.selected_mut() = Some(0);
    ui.detail.key = Some("check-0".to_string());
    ui.detail.open = true;
    ui.pane = Pane::Detail;
    ui.execute(Command::SelectNext, &mut app);
    assert_eq!(ui.list.state.selected(), Some(0), "list must not move");
    assert_eq!(ui.detail.stdout_scroll, 1, "scrolled instead");
    assert_eq!(ui.pane, Pane::Detail, "stays in detail pane");
}

#[test]
fn select_next_suite_is_a_noop_in_detail_pane() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    ui.pane = Pane::Detail;
    ui.execute(Command::NextSuite, &mut app);
    assert_eq!(ui.list.state.selected(), Some(0));
}

// -- Dwim --

#[test]
fn dwim_on_suite_folds() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    ui.execute(Command::ToggleDwim, &mut app);
    assert!(app.folded_suites.contains("db"));
}

#[test]
fn dwim_on_suite_folds_even_when_detail_open() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    ui.detail.key = Some("db:alpha".to_string());
    ui.detail.open = true;
    ui.execute(Command::ToggleDwim, &mut app);
    assert!(app.folded_suites.contains("db"));
    assert!(ui.detail.open); // detail stays open — suite Dwim doesn't touch it
}

#[test]
fn dwim_on_check_opens_detail() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(1); // Check("db:alpha")
    ui.detail.key = Some("db:alpha".to_string());
    ui.execute(Command::ToggleDwim, &mut app);
    assert!(ui.detail.open);
}

#[test]
fn dwim_on_check_with_children_still_opens_detail() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app_with_children();
    *ui.list.state.selected_mut() = Some(0); // Check("snap"), has children
    ui.detail.key = Some("snap".to_string());
    ui.execute(Command::ToggleDwim, &mut app);
    assert!(app.folded_checks.contains("snap"), "fold state untouched");
    assert!(ui.detail.open, "Dwim always opens detail on a check");
}

#[test]
fn dwim_on_check_closes_detail() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(1); // Check("db:alpha")
    ui.detail.key = Some("db:alpha".to_string());
    ui.detail.open = true;
    ui.execute(Command::ToggleDwim, &mut app);
    assert!(!ui.detail.open);
}

// -- Left/Right dwim --

#[test]
fn right_dwim_unfolds_folded_suite() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    app.toggle_suite("db"); // start folded
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    assert!(ui.execute(Command::RightDwim, &mut app));
    assert!(!app.folded_suites.contains("db"));
}

#[test]
fn right_dwim_on_unfolded_suite_moves_to_first_child() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app(); // unfolded by default
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    assert!(ui.execute(Command::RightDwim, &mut app));
    assert_eq!(ui.list.state.selected(), Some(1)); // db:alpha
}

#[test]
fn right_dwim_unfolds_folded_children() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app_with_children(); // folded by default
    *ui.list.state.selected_mut() = Some(0); // Check("snap")
    assert!(ui.execute(Command::RightDwim, &mut app));
    assert!(!app.folded_checks.contains("snap"));
}

#[test]
fn right_dwim_on_leaf_check_is_a_noop() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(3);
    *ui.list.state.selected_mut() = Some(0);
    assert!(!ui.execute(Command::RightDwim, &mut app));
    assert_eq!(ui.list.state.selected(), Some(0));
}

#[test]
fn left_dwim_folds_unfolded_suite() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app(); // unfolded by default
    *ui.list.state.selected_mut() = Some(0); // Suite("db")
    assert!(ui.execute(Command::LeftDwim, &mut app));
    assert!(app.folded_suites.contains("db"));
}

#[test]
fn left_dwim_on_suite_child_moves_to_parent() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_suite_app();
    *ui.list.state.selected_mut() = Some(1); // Check("db:alpha")
    assert!(ui.execute(Command::LeftDwim, &mut app));
    assert_eq!(ui.list.state.selected(), Some(0)); // Suite("db")
}

#[test]
fn left_dwim_on_top_level_flat_check_is_a_noop() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(3);
    *ui.list.state.selected_mut() = Some(0);
    assert!(!ui.execute(Command::LeftDwim, &mut app));
    assert_eq!(ui.list.state.selected(), Some(0));
}

// -- Pane --

#[test]
fn open_detail_switches_to_detail_pane() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(1);
    *ui.list.state.selected_mut() = Some(0);
    ui.detail.key = Some("check-0".to_string());
    assert!(ui.execute(Command::OpenDetail, &mut app));
    assert_eq!(ui.pane, Pane::Detail);
}

#[test]
fn toggle_detail_opening_leaves_pane_at_list() {
    // 'd' is a quick peek, not a focus request — walking down the list
    // with j/k while glancing at each check's detail must keep working.
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(1);
    ui.detail.key = Some("check-0".to_string());
    ui.execute(Command::ToggleDetail, &mut app);
    assert!(ui.detail.open);
    assert_eq!(ui.pane, Pane::List);
}

#[test]
fn toggle_detail_closing_leaves_pane_unchanged() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(1);
    ui.detail.key = Some("check-0".to_string());
    ui.detail.open = true;
    ui.pane = Pane::Detail;
    ui.execute(Command::ToggleDetail, &mut app);
    assert!(!ui.detail.open);
    assert_eq!(ui.pane, Pane::Detail);
}

#[test]
fn togglepane_enters_and_exits_detail() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(1);
    ui.detail.key = Some("check-0".to_string());
    assert!(ui.execute(Command::TogglePane, &mut app));
    assert_eq!(ui.pane, Pane::Detail);
    assert!(ui.detail.open, "entering detail pane opens the panel");
    assert!(ui.execute(Command::TogglePane, &mut app));
    assert_eq!(ui.pane, Pane::List);
}

#[test]
fn togglepane_is_a_noop_with_nothing_to_show() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(1);
    assert!(!ui.execute(Command::TogglePane, &mut app));
    assert_eq!(ui.pane, Pane::List);
}

#[test]
fn right_dwim_is_suppressed_in_detail_pane() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app_with_children(); // folded check with children
    *ui.list.state.selected_mut() = Some(0);
    ui.pane = Pane::Detail;
    assert!(!ui.execute(Command::RightDwim, &mut app));
    assert!(app.folded_checks.contains("snap"), "must not unfold");
}

#[test]
fn left_dwim_scrolls_instead_of_folding_in_detail_pane() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app_with_children();
    *ui.list.state.selected_mut() = Some(0); // Check("snap"), unfolded
    app.toggle_children("snap");
    ui.pane = Pane::Detail;
    ui.detail.stdout_h_scroll = 5; // not fully scrolled left
    assert!(!ui.execute(Command::LeftDwim, &mut app));
    assert!(!app.folded_checks.contains("snap"), "must not fold");
    assert_eq!(ui.pane, Pane::Detail);
}

#[test]
fn left_dwim_exits_detail_pane_and_folds_when_scrolled_fully_left() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app_with_children();
    *ui.list.state.selected_mut() = Some(0); // Check("snap"), unfolded
    app.toggle_children("snap");
    ui.pane = Pane::Detail;
    ui.detail.stdout_h_scroll = 0;
    assert!(ui.execute(Command::LeftDwim, &mut app));
    assert_eq!(ui.pane, Pane::List);
    assert!(app.folded_checks.contains("snap"));
}

#[test]
fn selecting_next_returns_to_list_pane() {
    let (mut ui, _rx) = make_ui();
    let mut app = make_app(3);
    *ui.list.state.selected_mut() = Some(0);
    // Not in the detail pane — SelectNext moves the list as usual and,
    // per after_selection_change, keeps/returns pane at List.
    ui.execute(Command::SelectNext, &mut app);
    assert_eq!(ui.pane, Pane::List);
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

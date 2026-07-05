// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding},
};

use crate::app::{App, VisibleItem};
use crate::ui::Ui;

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::app::{App, CheckEntry, EntryKind, Status};
    use crate::config::Keymap;
    use crate::render::render;
    use crate::ui::Ui;

    fn make_entry(name: &str, status: Status) -> CheckEntry {
        CheckEntry {
            name: name.to_string(),
            status,
            kind: EntryKind::Result,
            exit_code: "0".to_string(),
            stdout: String::new(),
            stderr: String::new(),
            suite: None,
            children: Vec::new(),
        }
    }

    fn snapshot(entries: &[CheckEntry], width: u16, height: u16) -> String {
        let mut app = App::new();
        for entry in entries {
            app.upsert(entry.clone());
        }
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut ui = Ui::new(tx);
        *ui.list.state.selected_mut() = Some(0);
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let keymap = Keymap::qwerty();
        terminal
            .draw(|frame| {
                render(frame, &app, &mut ui, &keymap);
            })
            .unwrap();
        terminal.backend().to_string()
    }

    #[test]
    fn shows_status_symbols() {
        let entries = vec![
            make_entry("check-foo", Status::Pass),
            make_entry("check-bar", Status::Fail),
            make_entry("check-baz", Status::Skip),
        ];
        insta::assert_snapshot!(snapshot(&entries, 40, 10));
    }

    #[test]
    fn renders_empty() {
        insta::assert_snapshot!(snapshot(&[], 40, 10));
    }

    #[test]
    fn selection_moves_within_view_without_scrolling() {
        // 10 entries, inner height 5 (7 - 2 borders): first render forces a
        // scroll to make entry 9 visible, landing offset at 5 (items 5..10).
        // Moving the selection back up to entry 6 stays inside that same
        // window, so the offset must not change.
        let entries: Vec<CheckEntry> = (0..10)
            .map(|i| make_entry(&format!("check-{i}"), Status::Pass))
            .collect();
        let mut app = App::new();
        for entry in &entries {
            app.upsert(entry.clone());
        }
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut ui = Ui::new(tx);
        let backend = TestBackend::new(40, 7);
        let mut terminal = Terminal::new(backend).unwrap();
        let keymap = Keymap::qwerty();

        *ui.list.state.selected_mut() = Some(9);
        terminal
            .draw(|frame| {
                render(frame, &app, &mut ui, &keymap);
            })
            .unwrap();
        let offset_after_scroll = ui.list.state.offset();
        assert!(offset_after_scroll > 0, "expected a scroll to occur");

        *ui.list.state.selected_mut() = Some(6);
        terminal
            .draw(|frame| {
                render(frame, &app, &mut ui, &keymap);
            })
            .unwrap();
        assert_eq!(
            ui.list.state.offset(),
            offset_after_scroll,
            "selecting an item already in view must not move the scroll offset"
        );
    }

    #[test]
    fn truncates_at_right_padding() {
        // Content per row: "✓ <name>" (symbol + space + name).
        // With borders (2) + padding (2), inner width = 20 - 4 = 16.
        // "✓ check-overflow!" = 2 + 15 = 17 chars, exceeding inner by 1.
        // If padding is present, the last char is truncated. Without padding,
        // the full text would fit (inner would be 18).
        let entries = vec![make_entry("check-overflow!", Status::Pass)];
        insta::assert_snapshot!(snapshot(&entries, 20, 5));
    }
}

fn count_spans(pass: usize, fail: usize, skip: usize, selected: bool) -> Vec<Span<'static>> {
    let (ps, fs, ss) = if selected {
        (
            Style::new().bg(Color::Green),
            Style::new().bg(Color::Red),
            Style::new().bg(Color::DarkGray),
        )
    } else {
        (
            Style::new().fg(Color::Green),
            Style::new().fg(Color::Red),
            Style::new().fg(Color::DarkGray),
        )
    };
    vec![
        Span::styled(format!("✓{pass}"), ps),
        Span::raw(" "),
        Span::styled(format!("✗{fail}"), fs),
        Span::raw(" "),
        Span::styled(format!("·{skip}"), ss),
    ]
}

fn counts_line(pass: usize, fail: usize, skip: usize) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    spans.extend(count_spans(pass, fail, skip, false));
    spans.push(Span::raw(" "));
    Line::from(spans)
}

pub fn render_list(frame: &mut Frame, app: &App, ui: &mut Ui, area: Rect) {
    let status = if ui.rebuilding {
        " loading… ".to_string()
    } else if let Some(n) = ui.watch_count {
        format!(" watching {n} files ")
    } else {
        String::new()
    };
    let (pass, fail, skip) = app.counts();
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .title("nix-result-checks")
        .title_bottom(status.as_str())
        .title_bottom(counts_line(pass, fail, skip).alignment(Alignment::Right));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = app
        .visible_items()
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let selected = ui.list.state.selected() == Some(idx);
            match item {
                VisibleItem::Suite(name) => {
                    let folded = app.folded_suites.contains(&name);
                    let (pass, fail, skip) = app.suite_counts(&name);
                    let arrow = if folded { "▶" } else { "▼" };
                    let mut spans = vec![Span::raw(format!("{arrow} {name} ("))];
                    spans.extend(count_spans(pass, fail, skip, selected));
                    spans.push(Span::raw(")"));
                    let mut style = Style::new().add_modifier(Modifier::BOLD);
                    if selected {
                        style = style.add_modifier(Modifier::REVERSED);
                    }
                    ListItem::new(Line::from(spans)).style(style)
                }
                VisibleItem::Check { key, depth } => {
                    let entry = app.entries.get(&key).expect("visible item has entry");
                    let indent = "  ".repeat(depth);
                    let arrow = if !app.child_keys.contains_key(&key) {
                        ""
                    } else if app.folded_checks.contains(&key) {
                        "▶ "
                    } else {
                        "▼ "
                    };
                    let line = format!("{indent}{arrow}{} {}", entry.status.symbol(), entry.name);
                    let mut style = Style::new().fg(entry.status.color());
                    if selected {
                        style = style.add_modifier(Modifier::REVERSED);
                    }
                    ListItem::new(line).style(style)
                }
            }
        })
        .collect();

    frame.render_stateful_widget(List::new(items), inner, &mut ui.list.state);
}

// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState, Padding},
    Frame,
};

use crate::app::App;
use crate::ui::Ui;

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use crate::app::{App, CheckEntry, EntryKind, Status};
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
        }
    }

    fn snapshot(entries: &[CheckEntry], width: u16, height: u16) -> String {
        let mut app = App::new();
        for entry in entries {
            app.upsert(entry.clone());
        }
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut ui = Ui::new(tx);
        ui.selected = Some(0);
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render(frame, &app, &ui);
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

pub fn render_list(frame: &mut Frame, app: &App, ui: &Ui, area: Rect) {
    let status = if ui.rebuilding {
        " loading… ".to_string()
    } else if let Some(n) = ui.watch_count {
        format!(" watching {n} files ")
    } else {
        String::new()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .title("nix-result-checks")
        .title_bottom(status.as_str());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = app
        .order
        .iter()
        .map(|name| {
            let entry = app
                .entries
                .get(name)
                .expect("order and entries are in sync");
            let line = format!("{} {}", entry.status.symbol(), entry.name);
            ListItem::new(line).style(Style::new().fg(entry.status.color()))
        })
        .collect();

    let mut state = ListState::default();
    state.select(ui.selected);

    frame.render_stateful_widget(
        List::new(items).highlight_style(Style::new().reversed()),
        inner,
        &mut state,
    );
}

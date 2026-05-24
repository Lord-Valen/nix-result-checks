// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Stylize},
    text::Text,
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};

use crate::ui::clamp_u16;

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    use super::{render_toast, truncate_to_lines};

    fn snapshot(msg: &str, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, width, height);
                render_toast(frame, msg, area);
            })
            .unwrap();
        terminal.backend().to_string()
    }

    #[test]
    fn fits_single_line() {
        insta::assert_snapshot!(snapshot("file not found", 80, 24));
    }

    #[test]
    fn wraps_long_message() {
        let msg = "this is a much longer error message that should wrap within the toast popup because it exceeds the maximum width";
        insta::assert_snapshot!(snapshot(msg, 80, 24));
    }

    #[test]
    fn clamps_to_terminal_width() {
        insta::assert_snapshot!(snapshot(
            "this message is definitely wider than the terminal",
            20,
            10
        ));
    }

    #[test]
    fn renders_empty_message() {
        insta::assert_snapshot!(snapshot("", 80, 24));
    }

    #[test]
    fn truncates_long_message() {
        let msg = (0..20)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = truncate_to_lines(&msg, 10, 40);
        assert!(result.ends_with("… (truncated)"), "got: {result:?}");
        assert_eq!(result.lines().count(), 11); // 10 content + truncation line
    }

    #[test]
    fn short_message_not_truncated() {
        let msg = "line 1\nline 2\nline 3";
        let result = truncate_to_lines(msg, 10, 40);
        assert_eq!(result, msg);
    }
}

/// Count how many lines `text` would occupy when word-wrapped to `width` characters.
/// Mirrors ratatui's `Wrap { trim: false }` behaviour: break at the last space that fits,
/// or force-break if a single word exceeds the width.
fn truncate_to_lines(text: &str, max_lines: usize, width: usize) -> String {
    let mut total = 0;
    let mut out = String::new();
    for (i, line) in text.split('\n').enumerate() {
        let h = wrapped_line_height(line, width);
        if total + h > max_lines {
            if i > 0 {
                out.push('\n');
            }
            out.push_str("… (truncated)");
            return out;
        }
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line);
        total += h;
    }
    out
}

fn wrap_line_count(text: &str, width: usize) -> usize {
    if width == 0 {
        return 1;
    }
    let mut lines = 0;
    for line in text.split('\n') {
        lines += wrapped_line_height(line, width);
    }
    lines.max(1)
}

fn wrapped_line_height(line: &str, width: usize) -> usize {
    if line.is_empty() {
        return 1;
    }
    let mut lines = 0;
    let mut col = 0;
    for word in line.split_inclusive(' ') {
        let len = word.chars().count();
        if col > 0 && col + len > width {
            lines += 1;
            col = 0;
        }
        if len > width {
            lines += len / width;
            col = len % width;
        } else {
            col += len;
        }
    }
    if col > 0 {
        lines += 1;
    }
    lines
}

const MAX_TOAST_LINES: usize = 10;

pub(crate) fn render_toast(frame: &mut Frame, msg: &str, area: Rect) {
    const TITLE: &str = "Error";
    let max_width = area.width.min(60);
    // 2 borders + 2 padding; at least wide enough to show the title
    let min_width = TITLE.chars().count() + 2;
    let width = clamp_u16(msg.chars().count().saturating_add(4))
        .max(clamp_u16(min_width))
        .min(max_width);
    let inner_width = width.saturating_sub(4) as usize;
    let msg = truncate_to_lines(msg, MAX_TOAST_LINES, inner_width);
    // Simulate word wrapping to count lines accurately.
    let wrapped_lines = wrap_line_count(&msg, inner_width);
    // 2 for top/bottom borders
    let height = clamp_u16(wrapped_lines.saturating_add(2)).min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect {
        x,
        y,
        width,
        height,
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(Text::raw(msg.as_str()))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title("Error")
                    .fg(Color::Red),
            ),
        popup,
    );
}

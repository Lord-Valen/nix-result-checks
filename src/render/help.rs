// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use ratatui::{
    Frame,
    layout::Rect,
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};

use crate::config::Keymap;
use crate::ui::clamp_u16;

pub(crate) fn render_help(frame: &mut Frame, keymap: &Keymap, area: Rect) {
    let lines = keymap.help_lines();

    let key_width = lines
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);

    let content: Vec<Line> = lines
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::raw(format!("{key:<key_width$}")).bold(),
                Span::raw("  "),
                Span::raw(desc.clone()),
            ])
        })
        .collect();

    let desc_width = lines
        .iter()
        .map(|(_, d)| d.chars().count())
        .max()
        .unwrap_or(0);
    let inner_width = key_width + 2 + desc_width;
    // 2 borders + 2 padding on each side = 6 overhead
    let popup_width = clamp_u16(inner_width + 6).min(area.width);
    let popup_height = clamp_u16(lines.len() + 2).min(area.height);

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup = Rect {
        x,
        y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(Text::from(content))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .title("Help"),
            ),
        popup,
    );
}

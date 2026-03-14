// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::Text,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use super::PanelBounds;
use crate::app::{App, EntryKind};
use crate::ui::{DetailFocus, Ui};

pub fn render_detail(
    frame: &mut Frame,
    app: &App,
    ui: &Ui,
    area: Rect,
) -> (PanelBounds, PanelBounds) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let stdout_bounds = PanelBounds {
        height: usize::from(cols[0].height.saturating_sub(2)),
        width: usize::from(cols[0].width.saturating_sub(2)),
    };
    let stderr_bounds = PanelBounds {
        height: usize::from(cols[1].height.saturating_sub(2)),
        width: usize::from(cols[1].width.saturating_sub(2)),
    };

    let Some(idx) = ui.selected else {
        return (stdout_bounds, stderr_bounds);
    };
    let Some(name) = app.order.get(idx) else {
        return (stdout_bounds, stderr_bounds);
    };
    let Some(entry) = app.entries.get(name) else {
        return (stdout_bounds, stderr_bounds);
    };

    let kind_str = match entry.kind {
        EntryKind::Result => "result",
        EntryKind::Snapshot => "snapshot",
    };
    let exit_code_str: &str = if entry.exit_code.is_empty() {
        "(none)"
    } else {
        &entry.exit_code
    };

    frame.render_widget(
        Paragraph::new(Text::raw(format!(
            "Kind:      {kind_str}\nExit Code: {exit_code_str}"
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(entry.name.as_str()),
        ),
        rows[0],
    );

    render_scrollable_panel(
        frame,
        "stdout",
        &entry.stdout,
        ui.stdout_scroll,
        ui.stdout_h_scroll,
        ui.detail_focus == DetailFocus::Stdout,
        cols[0],
    );
    render_scrollable_panel(
        frame,
        "stderr",
        &entry.stderr,
        ui.stderr_scroll,
        ui.stderr_h_scroll,
        ui.detail_focus == DetailFocus::Stderr,
        cols[1],
    );

    (stdout_bounds, stderr_bounds)
}

fn render_scrollable_panel(
    frame: &mut Frame,
    title: &str,
    content: &str,
    v_scroll: u16,
    h_scroll: u16,
    focused: bool,
    area: Rect,
) {
    let display = if content.is_empty() {
        "(empty)"
    } else {
        content
    };
    let visible_height = area.height.saturating_sub(2) as usize;
    let visible_width = area.width.saturating_sub(2) as usize;
    let (line_count, max_line_width) = display.lines().fold((0, 0), |(count, max_w), l| {
        (count + 1, max_w.max(l.chars().count()))
    });

    let block = if focused {
        Block::default().borders(Borders::ALL).title(title).bold()
    } else {
        Block::default().borders(Borders::ALL).title(title)
    };

    frame.render_widget(
        Paragraph::new(Text::raw(display))
            .block(block)
            .scroll((v_scroll, h_scroll)),
        area,
    );

    if line_count > visible_height {
        // Trim top/bottom border rows so the track height matches the content height.
        let scrollbar_area = Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(2),
            ..area
        };
        let mut state =
            ScrollbarState::new(line_count - visible_height).position(v_scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            scrollbar_area,
            &mut state,
        );
    }

    if max_line_width > visible_width {
        // Trim left/right border columns so the track width matches the content width.
        let scrollbar_area = Rect {
            x: area.x + 1,
            width: area.width.saturating_sub(2),
            ..area
        };
        let mut state =
            ScrollbarState::new(max_line_width - visible_width).position(h_scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                .begin_symbol(None)
                .end_symbol(None),
            scrollbar_area,
            &mut state,
        );
    }
}

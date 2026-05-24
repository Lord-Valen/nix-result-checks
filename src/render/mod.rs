// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

mod detail;
mod list;
pub(crate) mod toast;

use std::io::{self, Stdout, stdout};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout},
};

use crate::app::App;
use crate::ui::Ui;

/// Visible inner dimensions of a scrollable panel (excluding borders).
#[derive(Debug, Clone, Copy, Default)]
pub struct PanelBounds {
    pub height: usize,
    pub width: usize,
}

pub struct Renderer {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Renderer {
    pub fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(Self { terminal })
    }

    pub fn clear(&mut self) {
        let _ = self.terminal.clear();
    }

    pub fn draw(&mut self, app: &App, ui: &mut Ui) -> io::Result<()> {
        let mut bounds = None;
        self.terminal.draw(|f| bounds = render(f, app, ui))?;
        if let Some((stdout_b, stderr_b)) = bounds {
            ui.set_panel_bounds(stdout_b, stderr_b, app);
        }
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

fn render(frame: &mut Frame, app: &App, ui: &Ui) -> Option<(PanelBounds, PanelBounds)> {
    let area = frame.area();
    let bounds = if ui.detail_open && ui.detail_key.is_some() {
        let list_width = Ui::list_panel_width(app);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_width), Constraint::Fill(1)])
            .split(area);
        list::render_list(frame, app, ui, chunks[0]);
        Some(detail::render_detail(frame, app, ui, chunks[1]))
    } else {
        list::render_list(frame, app, ui, area);
        None
    };
    if let Some(msg) = &ui.toast {
        toast::render_toast(frame, msg, area);
    }
    bounds
}

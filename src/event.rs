// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use crate::app::CheckEntry;

pub enum Event {
    Key(ratatui::crossterm::event::KeyEvent),
    Mouse(ratatui::crossterm::event::MouseEvent),
    Resize,
    Entry(CheckEntry),
    Done,
    Reload,
    Quit,
    Error(anyhow::Error),
}

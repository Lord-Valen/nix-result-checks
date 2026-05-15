// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::sync::mpsc;

use ratatui::crossterm::event::{read as read_event, Event as TermEvent};

use crate::event::Event;

#[allow(clippy::needless_pass_by_value)]
pub fn run(tx: mpsc::Sender<Event>) {
    loop {
        let event = match read_event() {
            Ok(TermEvent::Key(key)) => Event::Key(key),
            Ok(TermEvent::Mouse(mouse)) => Event::Mouse(mouse),
            Ok(TermEvent::Resize(_, _)) => Event::Resize,
            _ => continue,
        };
        if tx.send(event).is_err() {
            return;
        }
    }
}

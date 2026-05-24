// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::mpsc;

use crate::app::{App, CheckEntry, Status};
use crate::event::Event;
use crate::input::watcher::{self, WatchMode};

fn emit_entry(out: &mut impl Write, entry: &CheckEntry) -> io::Result<()> {
    let line = serde_json::to_string(entry).map_err(io::Error::other)?;
    writeln!(out, "{line}")
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    ingest_tx: mpsc::Sender<()>,
    watch_mode: WatchMode,
) -> anyhow::Result<()> {
    let mut app = App::new();
    let mut previous: HashMap<String, CheckEntry> = HashMap::new();
    let mut had_failure = false;
    let mut out = io::stdout();

    let one_shot = matches!(watch_mode, WatchMode::None);
    let _watcher = match watch_mode {
        WatchMode::Dir => Some(watcher::start_dir(tx)?.0),
        WatchMode::File(ref path) => Some(watcher::start(path, tx)?),
        WatchMode::None => {
            drop(tx);
            None
        }
    };

    for event in &rx {
        match event {
            Event::Entry(entry) => {
                app.upsert(entry);
            }
            Event::Done => {
                app.prune();
                let keys: Vec<String> = app.all_keys().cloned().collect();
                for name in &keys {
                    let entry = &app.entries[name];
                    let changed = previous.get(name).map_or(true, |prev| prev != entry);
                    if changed {
                        if let Err(e) = emit_entry(&mut out, entry) {
                            if e.kind() == io::ErrorKind::BrokenPipe {
                                return Ok(());
                            }
                            return Err(e.into());
                        }
                        if entry.status == Status::Fail {
                            had_failure = true;
                        }
                        previous.insert(name.clone(), entry.clone());
                    }
                }
                app.bump_generation();
                if one_shot {
                    break;
                }
            }
            Event::Error(e) => {
                eprintln!("error: {e:#}");
                std::process::exit(1);
            }
            Event::Reload => {
                app.bump_generation();
                let _ = ingest_tx.send(());
            }
            _ => {}
        }
    }

    if had_failure {
        std::process::exit(1);
    }
    Ok(())
}

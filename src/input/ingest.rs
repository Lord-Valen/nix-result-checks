// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::{Receiver, Sender};

use serde_json::Value;

use crate::app::CheckEntry;
use crate::event::Event;

pub enum Source {
    Stdin,
    File(PathBuf),
    Flake(String),
    Expr { expr: String, impure: bool },
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(source: Source, event_tx: Sender<Event>, ingest_rx: Receiver<()>) {
    stream(&source, &event_tx);
    while let Ok(()) = ingest_rx.recv() {
        stream(&source, &event_tx);
    }
}

fn stream(source: &Source, tx: &Sender<Event>) {
    let ended_on_doc = match source {
        Source::Stdin => stream_reader(io::stdin(), tx),
        Source::File(path) => open_and_stream(path, tx),
        Source::Flake(attr) => match nix_build(&[attr], false, tx) {
            Some(path) => open_and_stream(&path, tx),
            None => false,
        },
        Source::Expr { expr, impure } => match nix_build(&["--expr", expr], *impure, tx) {
            Some(path) => open_and_stream(&path, tx),
            None => false,
        },
    };
    if !ended_on_doc {
        let _ = tx.send(Event::Done);
    }
}

fn open_and_stream(path: &Path, tx: &Sender<Event>) -> bool {
    match File::open(path).map(BufReader::new) {
        Ok(reader) => stream_reader(reader, tx),
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
            false
        }
    }
}

fn nix_build(args: &[&str], impure: bool, tx: &Sender<Event>) -> Option<PathBuf> {
    let result = process::Command::new("nix")
        .arg("build")
        .args(impure.then_some("--impure"))
        .args(["--no-link", "--print-out-paths"])
        .args(args)
        .output();
    match result {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            let path = s.lines().next().unwrap_or("").trim();
            if path.is_empty() {
                let _ = tx.send(Event::Error(anyhow::anyhow!(
                    "nix build produced no output"
                )));
                None
            } else {
                Some(PathBuf::from(path))
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            let _ = tx.send(Event::Error(anyhow::anyhow!("nix build failed:\n{stderr}")));
            None
        }
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
            None
        }
    }
}

fn stream_reader<R: Read>(reader: R, tx: &Sender<Event>) -> bool {
    let iter = serde_json::Deserializer::from_reader(reader).into_iter::<Value>();
    let mut ended_on_doc = false;
    for result in iter {
        match result {
            Ok(Value::Array(arr)) => {
                for v in arr {
                    send_entry(v, tx);
                }
                let _ = tx.send(Event::Done);
                ended_on_doc = true;
            }
            Ok(other) => {
                send_entry(other, tx);
                ended_on_doc = false;
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.into()));
                ended_on_doc = false;
            }
        }
    }
    ended_on_doc
}

fn send_entry(val: Value, tx: &Sender<Event>) {
    match serde_json::from_value::<CheckEntry>(val) {
        Ok(entry) => {
            let _ = tx.send(Event::Entry(entry));
        }
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
        }
    }
}

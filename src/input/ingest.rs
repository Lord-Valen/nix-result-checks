// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use serde_json::Value;

use crate::app::CheckEntry;
use crate::event::Event;

pub enum Source {
    Stdin,
    File(PathBuf),
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
        Source::File(path) => match File::open(path).map(BufReader::new) {
            Ok(reader) => stream_reader(reader, tx),
            Err(e) => {
                let _ = tx.send(Event::Error(e.into()));
                false
            }
        },
    };
    if !ended_on_doc {
        let _ = tx.send(Event::Done);
    }
}

/// Reads values from the reader, sending entries as they arrive.
///
/// Sends `Event::Done` after each top-level array (document boundary), which
/// triggers pruning of the previous generation's stale entries. Returns true
/// if the stream ended on such a boundary (Done already sent).
///
/// NDJSON (top-level objects) streams continuously without pruning — there is
/// no natural document boundary to detect. However, an empty array `[]` can
/// serve as a sentinel: it fires Done (pruning the previous generation) and
/// advances the generation, without adding any entries. Generators that emit
/// NDJSON can append `[]` to signal end-of-generation.
fn stream_reader<R: Read>(reader: R, tx: &Sender<Event>) -> bool {
    let iter = serde_json::Deserializer::from_reader(reader).into_iter::<Value>();
    let mut ended_on_doc = false;
    for result in iter {
        match result {
            Ok(Value::Array(arr)) => {
                for v in arr {
                    send_entry(v, tx);
                }
                // Array boundary: prune stale entries from the previous generation.
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

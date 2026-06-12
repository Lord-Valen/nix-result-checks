// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use serde_json::Value;

use crate::app::CheckEntry;
use crate::event::Event;
use crate::runner;

pub enum Source {
    Stdin,
    File(PathBuf),
    Flake(String),
    Convention {
        flakeref: String,
        workers: usize,
    },
    Expr {
        expr: String,
        impure: bool,
    },
    NixFile {
        file: PathBuf,
        attr: Option<String>,
        workers: usize,
    },
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
        Source::Convention { flakeref, workers } => stream_convention(flakeref, *workers, tx),
        Source::Expr { expr, impure } => match nix_build(&["--expr", expr], *impure, tx) {
            Some(path) => open_and_stream(&path, tx),
            None => false,
        },
        Source::NixFile {
            file,
            attr,
            workers,
        } => match attr {
            Some(attr) => match nix_build_file(file, Some(attr), tx) {
                Some(path) => open_and_stream(&path, tx),
                None => false,
            },
            None => stream_file_convention(file, *workers, tx),
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

/// Stream both halves of the resultChecks convention: the report (built
/// derivation checks) and the eval checks (forced in parallel by
/// nix-eval-jobs). The two run concurrently; one Done is sent by the
/// caller after both complete, so report document boundaries must not
/// prune the eval entries arriving alongside.
fn stream_convention(flakeref: &str, workers: usize, tx: &Sender<Event>) -> bool {
    let system = match runner::current_system() {
        Ok(system) => system,
        Err(e) => {
            let _ = tx.send(Event::Error(e));
            return false;
        }
    };
    thread::scope(|scope| {
        scope.spawn(|| stream_report(flakeref, system, tx));
        scope.spawn(|| stream_eval_checks(flakeref, system, workers, tx));
    });
    false
}

fn stream_report(flakeref: &str, system: &str, tx: &Sender<Event>) {
    let Some(path) = nix_build(&[&runner::report_attr(flakeref, system)], false, tx) else {
        return;
    };
    stream_entries_from(&path, tx);
}

fn stream_entries_from(path: &Path, tx: &Sender<Event>) {
    match File::open(path).map(BufReader::new) {
        Ok(reader) => {
            stream_entries(reader, tx);
        }
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
        }
    }
}

/// File-mode convention: the file evaluates to { report; evalChecks; }.
/// Mirrors the flake convention without flakes or nix-command.
fn stream_file_convention(file: &Path, workers: usize, tx: &Sender<Event>) -> bool {
    thread::scope(|scope| {
        scope.spawn(|| {
            if let Some(path) = nix_build_file(file, Some("report"), tx) {
                stream_entries_from(&path, tx);
            }
        });
        scope.spawn(|| stream_file_eval_checks(file, workers, tx));
    });
    false
}

fn stream_file_eval_checks(file: &Path, workers: usize, tx: &Sender<Event>) {
    let args = runner::nej_file_args(&file.to_string_lossy(), workers);
    match process::Command::new("nix-eval-jobs")
        .args(&args)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()
    {
        Ok(child) => stream_nej(child, tx),
        Err(e) if e.kind() == io::ErrorKind::NotFound => file_eval_fallback(file, tx),
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
        }
    }
}

/// Sequential fallback for file mode: nix-instantiate works without
/// flakes or nix-command.
fn file_eval_fallback(file: &Path, tx: &Sender<Event>) {
    match process::Command::new("nix-instantiate")
        .args(["--eval", "--strict", "--json"])
        .arg(file)
        .args(["-A", "evalChecks"])
        .output()
    {
        Ok(o) if o.status.success() => send_tree_entries(&o.stdout, tx),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            let _ = tx.send(Event::Error(anyhow::anyhow!(
                "nix-instantiate failed:\n{stderr}"
            )));
        }
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
        }
    }
}

fn send_tree_entries(json: &[u8], tx: &Sender<Event>) {
    let entries = serde_json::from_slice::<Value>(json)
        .map_err(anyhow::Error::from)
        .and_then(runner::entries_from_tree);
    match entries {
        Ok(entries) => {
            for entry in entries {
                let _ = tx.send(Event::Entry(entry));
            }
        }
        Err(e) => {
            let _ = tx.send(Event::Error(e));
        }
    }
}

fn stream_eval_checks(flakeref: &str, system: &str, workers: usize, tx: &Sender<Event>) {
    let args = runner::nej_args(flakeref, system, workers);
    match process::Command::new("nix-eval-jobs")
        .args(&args)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()
    {
        Ok(child) => stream_nej(child, tx),
        Err(e) if e.kind() == io::ErrorKind::NotFound => eval_fallback(flakeref, system, tx),
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
        }
    }
}

fn stream_nej(mut child: process::Child, tx: &Sender<Event>) {
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) if line.trim().is_empty() => {}
                Ok(line) => match runner::entry_from_line(&line) {
                    Ok(entry) => {
                        let _ = tx.send(Event::Entry(entry));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e));
                    }
                },
                Err(e) => {
                    let _ = tx.send(Event::Error(e.into()));
                }
            }
        }
    }
    match child.wait_with_output() {
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            let _ = tx.send(Event::Error(anyhow::anyhow!(
                "nix-eval-jobs failed:\n{stderr}"
            )));
        }
        Ok(_) => {}
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
        }
    }
}

/// Sequential fallback when nix-eval-jobs is not on PATH: fetch the whole
/// evalChecks tree in one `nix eval --json` call.
fn eval_fallback(flakeref: &str, system: &str, tx: &Sender<Event>) {
    let attr = runner::eval_checks_attr(flakeref, system);
    match process::Command::new("nix")
        .args(["eval", "--option", "eval-cache", "false", "--json", &attr])
        .output()
    {
        Ok(o) if o.status.success() => send_tree_entries(&o.stdout, tx),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            let _ = tx.send(Event::Error(anyhow::anyhow!("nix eval failed:\n{stderr}")));
        }
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
        }
    }
}

fn run_builder(cmd: &mut process::Command, label: &str, tx: &Sender<Event>) -> Option<PathBuf> {
    match cmd.output() {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            let path = s.lines().next().unwrap_or("").trim().to_owned();
            if path.is_empty() {
                let _ = tx.send(Event::Error(anyhow::anyhow!("{label} produced no output")));
                None
            } else {
                Some(PathBuf::from(path))
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
            let _ = tx.send(Event::Error(anyhow::anyhow!("{label} failed:\n{stderr}")));
            None
        }
        Err(e) => {
            let _ = tx.send(Event::Error(e.into()));
            None
        }
    }
}

fn nix_build(args: &[&str], impure: bool, tx: &Sender<Event>) -> Option<PathBuf> {
    run_builder(
        process::Command::new("nix")
            .arg("build")
            .args(impure.then_some("--impure"))
            .args(["--no-link", "--print-out-paths"])
            .args(args),
        "nix build",
        tx,
    )
}

fn nix_build_file(file: &Path, attr: Option<&str>, tx: &Sender<Event>) -> Option<PathBuf> {
    run_builder(
        process::Command::new("nix-build")
            .arg(file)
            .args(attr.iter().flat_map(|a| ["-A", a]))
            .arg("--no-out-link"),
        "nix-build",
        tx,
    )
}

fn stream_reader<R: Read>(reader: R, tx: &Sender<Event>) -> bool {
    stream_json(reader, tx, true)
}

/// Like `stream_reader`, but document boundaries do not emit Done.
/// Used when entries from several sources share one generation.
fn stream_entries<R: Read>(reader: R, tx: &Sender<Event>) -> bool {
    stream_json(reader, tx, false)
}

fn stream_json<R: Read>(reader: R, tx: &Sender<Event>, done_on_doc: bool) -> bool {
    let iter = serde_json::Deserializer::from_reader(reader).into_iter::<Value>();
    let mut ended_on_doc = false;
    for result in iter {
        match result {
            Ok(Value::Array(arr)) => {
                for v in arr {
                    send_entry(v, tx);
                }
                if done_on_doc {
                    let _ = tx.send(Event::Done);
                }
                ended_on_doc = done_on_doc;
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

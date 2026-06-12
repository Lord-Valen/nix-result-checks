#![warn(clippy::pedantic)]

// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

mod app;
mod config;
mod event;
mod input;
mod render;
mod runner;
mod stream;
mod ui;

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use clap::Parser;

use config::{Config, Keymap, PresetName};
use event::Event;
use input::WatchMode;
use input::ingest::Source;
use render::Renderer;
use ui::Ui;

#[derive(Parser)]
#[command(about = "nix-result-checks TUI report viewer")]
struct Args {
    /// A flake reference (e.g. `.`) to run via the resultChecks
    /// convention: builds resultChecks.<system>.report and evaluates
    /// resultChecks.<system>.evalChecks through nix-eval-jobs. With a
    /// fragment (e.g. `.#attr`) builds that attribute as a report.
    #[arg(short = 'F', long)]
    flake: Option<String>,

    /// Eval workers for nix-eval-jobs (convention mode).
    #[arg(short = 'j', long, default_value_t = 8)]
    workers: usize,

    /// Evaluate a Nix expression and watch for changes.
    #[arg(short, long)]
    expr: Option<String>,

    /// A Nix file evaluating to { report; evalChecks; }, run like the
    /// flake convention but through nix-build and nix-instantiate (no
    /// flakes or nix-command required). With -A, builds that attribute
    /// as a report instead.
    #[arg(short, long, conflicts_with_all = ["flake", "expr"])]
    file: Option<PathBuf>,

    /// Attribute to build from --file.
    #[arg(short = 'A', long, requires = "file")]
    attr: Option<String>,

    /// Allow impure Nix evaluation (passed to nix build for --expr).
    #[arg(short, long, requires = "expr")]
    impure: bool,

    /// Re-stream the report whenever the file changes.
    #[arg(short, long)]
    watch: bool,

    /// Emit check results as newline-delimited JSON, one entry per line.
    /// Only changed entries are emitted on reload. Exits 1 if any check failed.
    #[arg(short, long)]
    stream: bool,

    /// Keymap preset to use.
    #[arg(short, long, value_enum)]
    keymap: Option<PresetName>,

    /// Path to a JSON config file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Path to a JSON report file, or - for stdin.
    #[arg(required_unless_present_any = ["flake", "expr", "file"])]
    report: Option<PathBuf>,
}

fn resolve_source(args: Args) -> anyhow::Result<(Source, WatchMode)> {
    let dir_watch = || {
        if args.watch {
            WatchMode::Dir
        } else {
            WatchMode::None
        }
    };
    if let Some(attr) = args.flake {
        if attr.contains('#') {
            Ok((Source::Flake(attr), dir_watch()))
        } else {
            Ok((
                Source::Convention {
                    flakeref: attr,
                    workers: args.workers,
                },
                dir_watch(),
            ))
        }
    } else if let Some(expr) = args.expr {
        Ok((
            Source::Expr {
                expr,
                impure: args.impure,
            },
            dir_watch(),
        ))
    } else if let Some(file) = args.file {
        Ok((
            Source::NixFile {
                file,
                attr: args.attr,
                workers: args.workers,
            },
            dir_watch(),
        ))
    } else if let Some(path) = args.report {
        if path.as_os_str() == "-" {
            if args.watch {
                anyhow::bail!("--watch cannot be used with stdin");
            }
            Ok((Source::Stdin, WatchMode::None))
        } else {
            let watch_mode = if args.watch {
                WatchMode::File(path.clone())
            } else {
                WatchMode::None
            };
            Ok((Source::File(path), watch_mode))
        }
    } else {
        unreachable!()
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (tx, rx) = mpsc::channel::<Event>();
    let (ingest_tx, ingest_rx) = mpsc::channel::<()>();

    let config = Config::load(args.config.as_deref())?;
    let keymap = Config::resolve(config, args.keymap);
    let stream = args.stream;

    let (source, watch_mode) = resolve_source(args)?;

    let event_tx = tx.clone();
    thread::spawn(move || input::ingest::run(source, event_tx, ingest_rx));

    if stream {
        stream::run(rx, tx, ingest_tx, watch_mode)
    } else {
        let input_tx = tx.clone();
        thread::spawn(move || input::terminal::run(input_tx));
        run(rx, tx, ingest_tx, keymap, watch_mode)
    }
}

#[allow(clippy::needless_pass_by_value)]
fn run(
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    ingest_tx: mpsc::Sender<()>,
    keymap: Keymap,
    watch_mode: WatchMode,
) -> anyhow::Result<()> {
    let mut renderer = Renderer::new()?;
    let mut app = app::App::new();
    let mut ui = Ui::new(tx.clone());

    let _watcher = match watch_mode {
        WatchMode::Dir => {
            let (w, count) = input::watcher::start_dir(tx)?;
            ui.watch_count = Some(count);
            Some(w)
        }
        WatchMode::File(ref path) => Some(input::watcher::start(path, tx)?),
        WatchMode::None => {
            drop(tx);
            None
        }
    };

    renderer.clear();
    renderer.draw(&app, &mut ui, &keymap)?;

    for event in &rx {
        match event {
            Event::Quit => break,
            Event::Entry(entry) => {
                let first = app.order.is_empty();
                app.upsert(entry);
                if first {
                    ui.selected = Some(0);
                }
            }
            Event::Done => {
                let selected_key = ui.selected.and_then(|i| {
                    app.visible_items()
                        .into_iter()
                        .nth(i)
                        .and_then(|v| match v {
                            crate::app::VisibleItem::Check(k) => Some(k),
                            crate::app::VisibleItem::Suite(name) => {
                                Some(format!("__suite__{name}"))
                            }
                        })
                });
                let old_idx = ui.selected;
                app.prune();
                app.bump_generation();
                ui.rebuilding = false;
                let visible = app.visible_items();
                ui.selected = if visible.is_empty() {
                    None
                } else if let Some(key) = selected_key {
                    let pos = if let Some(suite_name) = key.strip_prefix("__suite__") {
                        visible.iter().position(
                            |v| matches!(v, crate::app::VisibleItem::Suite(n) if n == suite_name),
                        )
                    } else {
                        visible.iter().position(
                            |v| matches!(v, crate::app::VisibleItem::Check(k) if k == &key),
                        )
                    };
                    Some(pos.unwrap_or_else(|| old_idx.unwrap_or(0).min(visible.len() - 1)))
                } else {
                    None
                };
            }
            Event::Error(e) => {
                ui.toast = Some(format!("{e:#}"));
            }
            Event::Reload => {
                app.bump_generation();
                ui.rebuilding = true;
                let _ = ingest_tx.send(());
            }
            ref event => ui.handle(event, &mut app, &keymap),
        }
        renderer.draw(&app, &mut ui, &keymap)?;
    }

    Ok(())
}

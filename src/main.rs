#![warn(clippy::pedantic)]

// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

mod app;
mod config;
mod event;
mod input;
mod render;
mod ui;

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use clap::Parser;

use app::App;
use config::{Config, Keymap, PresetName};
use event::Event;
use input::ingest::Source;
use render::Renderer;
use ui::Ui;

#[derive(Parser)]
#[command(about = "nix-result-checks TUI report viewer")]
struct Args {
    /// Evaluate a flake and watch for changes.
    #[arg(short = 'F', long)]
    flake: Option<String>,

    /// Evaluate a Nix expression and watch for changes.
    #[arg(short, long)]
    expr: Option<String>,

    /// Evaluate a Nix file (uses nix-build, no nix-command required).
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

enum WatchMode {
    None,
    Dir,
    File(PathBuf),
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (tx, rx) = mpsc::channel::<Event>();
    let (ingest_tx, ingest_rx) = mpsc::channel::<()>();

    let config = Config::load(args.config.as_deref())?;
    let keymap = Config::resolve(config, args.keymap);

    let source: Source;
    let watch_mode: WatchMode;
    if let Some(attr) = args.flake {
        source = Source::Flake(attr);
        watch_mode = if args.watch {
            WatchMode::Dir
        } else {
            WatchMode::None
        };
    } else if let Some(expr) = args.expr {
        source = Source::Expr {
            expr,
            impure: args.impure,
        };
        watch_mode = if args.watch {
            WatchMode::Dir
        } else {
            WatchMode::None
        };
    } else if let Some(file) = args.file {
        source = Source::NixFile {
            file,
            attr: args.attr,
        };
        watch_mode = if args.watch {
            WatchMode::Dir
        } else {
            WatchMode::None
        };
    } else if let Some(ref path) = args.report {
        if path.as_os_str() == "-" {
            if args.watch {
                anyhow::bail!("--watch cannot be used with stdin");
            }
            source = Source::Stdin;
            watch_mode = WatchMode::None;
        } else {
            source = Source::File(path.clone());
            watch_mode = if args.watch {
                WatchMode::File(path.clone())
            } else {
                WatchMode::None
            };
        }
    } else {
        unreachable!()
    }

    let event_tx = tx.clone();
    thread::spawn(move || input::ingest::run(source, event_tx, ingest_rx));

    let input_tx = tx.clone();
    thread::spawn(move || input::terminal::run(input_tx));

    run(rx, tx, ingest_tx, keymap, watch_mode)
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
    let mut app = App::new();
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
    renderer.draw(&app, &mut ui)?;

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
                let selected_name = ui.selected.and_then(|i| app.order.get(i)).cloned();
                let old_idx = ui.selected;
                app.prune();
                app.bump_generation();
                ui.rebuilding = false;
                ui.selected = if app.order.is_empty() {
                    None
                } else if let Some(name) = selected_name {
                    Some(
                        app.order
                            .iter()
                            .position(|n| n == &name)
                            .unwrap_or_else(|| old_idx.unwrap_or(0).min(app.order.len() - 1)),
                    )
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
            ref event => ui.handle(event, &app, &keymap),
        }
        renderer.draw(&app, &mut ui)?;
    }

    Ok(())
}

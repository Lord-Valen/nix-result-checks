// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

pub enum WatchMode {
    None,
    Dir,
    File(PathBuf),
}

use ignore::WalkBuilder;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::event::Event;

fn make_watcher(
    tx: mpsc::Sender<Event>,
    filter: impl Fn(&notify::Event) -> bool + Send + 'static,
) -> anyhow::Result<RecommendedWatcher> {
    let last = Arc::new(Mutex::new(
        Instant::now()
            .checked_sub(Duration::from_secs(10))
            .unwrap_or_else(Instant::now),
    ));
    Ok(notify::recommended_watcher(
        move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };
            if matches!(event.kind, notify::EventKind::Access(_)) {
                return;
            }
            if !filter(&event) {
                return;
            }
            let Ok(mut t) = last.lock() else { return };
            if t.elapsed() < Duration::from_millis(300) {
                return;
            }
            *t = Instant::now();
            let _ = tx.send(Event::Reload);
        },
    )?)
}

fn non_ignored_files(root: &Path) -> HashSet<PathBuf> {
    WalkBuilder::new(root)
        .require_git(false)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .map(|e| e.into_path())
        .collect()
}

/// Watch the current directory recursively, ignoring files excluded by
/// `.gitignore`, `.ignore`, or similar files.
/// Used with `--flake`/`--expr --watch` to trigger rebuilds.
/// Returns the watcher and the number of files being watched.
pub fn start_dir(tx: mpsc::Sender<Event>) -> anyhow::Result<(RecommendedWatcher, usize)> {
    let root = std::env::current_dir()?.canonicalize()?;
    let watched = non_ignored_files(&root);
    let count = watched.len();
    let mut w = if watched.is_empty() {
        make_watcher(tx, |_| true)?
    } else {
        make_watcher(tx, move |event: &notify::Event| {
            event.paths.iter().any(|p| watched.contains(p))
        })?
    };
    w.watch(Path::new("."), RecursiveMode::Recursive)?;
    Ok((w, count))
}

pub fn start(path: &Path, tx: mpsc::Sender<Event>) -> anyhow::Result<RecommendedWatcher> {
    let target: OsString = path
        .file_name()
        .expect("report path must have a filename")
        .to_owned();
    let watch_dir = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let mut w = make_watcher(tx, move |event| {
        event
            .paths
            .iter()
            .any(|p| p.file_name() == Some(target.as_os_str()))
    })?;
    w.watch(&watch_dir, RecursiveMode::NonRecursive)?;
    Ok(w)
}

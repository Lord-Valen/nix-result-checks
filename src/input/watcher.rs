// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::event::Event;

pub fn start(path: &Path, tx: mpsc::Sender<Event>) -> anyhow::Result<RecommendedWatcher> {
    let last_clone = Arc::new(Mutex::new(
        Instant::now()
            .checked_sub(Duration::from_secs(10))
            .unwrap_or_else(Instant::now),
    ));
    let target: OsString = path
        .file_name()
        .expect("report path must have a filename")
        .to_owned();
    let watch_dir = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);

    let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(event) = res else { return };
        if matches!(event.kind, notify::EventKind::Access(_)) {
            return;
        }
        if !event
            .paths
            .iter()
            .any(|p| p.file_name() == Some(target.as_os_str()))
        {
            return;
        }
        let mut t = last_clone.lock().unwrap();
        if t.elapsed() < Duration::from_millis(300) {
            return;
        }
        *t = Instant::now();
        let _ = tx.send(Event::Reload);
    })?;
    w.watch(&watch_dir, RecursiveMode::NonRecursive)?;
    Ok(w)
}

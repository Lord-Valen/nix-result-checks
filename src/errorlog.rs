// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static TRUNCATED: AtomicBool = AtomicBool::new(false);

/// `dirs::state_dir()/nrc/errors.log`, or `None` if no state directory
/// is available for this platform.
pub fn path() -> Option<PathBuf> {
    dirs::state_dir().map(|d| d.join("nrc").join("errors.log"))
}

/// A short suffix pointing at the log file, or empty if `path()` is
/// `None`. For appending to whatever's already shown to the user, so
/// they know where to find the untruncated version.
pub fn hint() -> String {
    path().map_or_else(String::new, |p| format!(" (full error: {})", p.display()))
}

/// Appends `message` to `path()`, timestamped. The TUI's toast
/// truncates long messages to fit the popup
/// (`render::toast::MAX_TOAST_LINES`), so this is the only place a
/// full, untruncated error is ever recoverable. The file is truncated
/// on the first error logged by this process, so it never grows past
/// one run's worth of errors, while every error within that run is
/// kept. Silently does nothing if no state directory is available.
pub fn append(message: &str) {
    let Some(path) = path() else {
        return;
    };
    let Some(dir) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    // .append(true) and .truncate(true) can't be combined (rejected by
    // OpenOptions itself, not just the OS), so the first write of this
    // process opens fresh (truncating) and later writes reopen in
    // plain append mode.
    let fresh = !TRUNCATED.swap(true, Ordering::Relaxed);
    let mut opts = OpenOptions::new();
    opts.create(true);
    if fresh {
        opts.write(true).truncate(true);
    } else {
        opts.append(true);
    }
    let Ok(mut file) = opts.open(&path) else {
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = writeln!(file, "[{now}] {message}");
}

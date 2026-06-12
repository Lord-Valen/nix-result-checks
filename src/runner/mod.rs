// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

//! Protocol for evaluating eval checks through nix-eval-jobs.
//!
//! The flake exposes pre-computed entries at
//! `resultChecks.<system>.evalChecks.<check>.<test>`. nix-eval-jobs only
//! emits derivation-shaped output, so `select.nix` wraps each entry in a
//! stub derivation that is never instantiated; `--apply` then lifts the
//! entry into the `extraValue` field of each NDJSON line. When
//! nix-eval-jobs is unavailable, the same tree is fetched in one piece
//! with `nix eval --json` instead (sequential, but dependency-free).

use serde::Deserialize;
use serde_json::Value;

use crate::app::{CheckEntry, EntryKind, Status};

pub const SELECT: &str = include_str!("select.nix");
pub const APPLY: &str = "drv: drv.result";

#[derive(Debug, Deserialize)]
struct PreEntry {
    kind: EntryKind,
    status: Status,
    #[serde(rename = "exitCode")]
    exit_code: String,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Deserialize)]
struct NejLine {
    #[serde(rename = "attrPath")]
    attr_path: Vec<String>,
    #[serde(rename = "extraValue")]
    extra_value: Option<PreEntry>,
    error: Option<String>,
}

fn identify(attr_path: &[String]) -> (Option<String>, String) {
    match attr_path {
        [suite, name, ..] => (Some(suite.clone()), name.clone()),
        [name] => (None, name.clone()),
        [] => (None, String::new()),
    }
}

fn entry(suite: Option<String>, name: String, pre: PreEntry) -> CheckEntry {
    CheckEntry {
        name,
        suite,
        kind: pre.kind,
        status: pre.status,
        exit_code: pre.exit_code,
        stdout: pre.stdout,
        stderr: pre.stderr,
    }
}

/// An evaluation error is a failing entry: the test exists and its
/// verdict is that it could not be evaluated.
fn error_entry(suite: Option<String>, name: String, error: String) -> CheckEntry {
    CheckEntry {
        name,
        suite,
        kind: EntryKind::Eval,
        status: Status::Fail,
        exit_code: "1".to_owned(),
        stdout: String::new(),
        stderr: error,
    }
}

/// Parse one nix-eval-jobs NDJSON line into a check entry.
pub fn entry_from_line(line: &str) -> anyhow::Result<CheckEntry> {
    let parsed: NejLine = serde_json::from_str(line)?;
    let (suite, name) = identify(&parsed.attr_path);
    match (parsed.extra_value, parsed.error) {
        (Some(pre), _) => Ok(entry(suite, name, pre)),
        (None, Some(error)) => Ok(error_entry(suite, name, error)),
        (None, None) => anyhow::bail!("nix-eval-jobs line has neither extraValue nor error"),
    }
}

/// Parse the whole evalChecks tree, as returned by `nix eval --json`,
/// into check entries. Fallback path when nix-eval-jobs is missing.
pub fn entries_from_tree(tree: Value) -> anyhow::Result<Vec<CheckEntry>> {
    let checks: std::collections::BTreeMap<String, std::collections::BTreeMap<String, PreEntry>> =
        serde_json::from_value(tree)?;
    Ok(checks
        .into_iter()
        .flat_map(|(suite, tests)| {
            tests
                .into_iter()
                .map(move |(name, pre)| entry(Some(suite.clone()), name, pre))
        })
        .collect())
}

/// The attribute holding pre-computed eval entries for `system`.
pub fn eval_checks_attr(flakeref: &str, system: &str) -> String {
    format!("{flakeref}#resultChecks.{system}.evalChecks")
}

/// The select script for file mode, where the root is the convention
/// attrset itself rather than a flake fragment: project out evalChecks
/// before wrapping.
pub fn file_select() -> String {
    format!("root: ({SELECT}) root.evalChecks")
}

/// The attribute holding the derivation-check report for `system`.
pub fn report_attr(flakeref: &str, system: &str) -> String {
    format!("{flakeref}#resultChecks.{system}.report")
}

// Eval results must reflect the current tree, not a previously cached
// verdict; a cached failure from an interrupted run would otherwise
// wedge every later one.
fn nej_base_args(workers: usize) -> Vec<String> {
    [
        "--option",
        "eval-cache",
        "false",
        "--no-instantiate",
        "--force-recurse",
        "--workers",
        &workers.to_string(),
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

/// Arguments for the parallel nix-eval-jobs invocation in flake mode.
pub fn nej_args(flakeref: &str, system: &str, workers: usize) -> Vec<String> {
    let mut args = nej_base_args(workers);
    args.extend(
        [
            "--select",
            SELECT,
            "--apply",
            APPLY,
            "--flake",
            &eval_checks_attr(flakeref, system),
        ]
        .iter()
        .map(ToString::to_string),
    );
    args
}

/// Arguments for the parallel nix-eval-jobs invocation in file mode.
pub fn nej_file_args(file: &str, workers: usize) -> Vec<String> {
    let mut args = nej_base_args(workers);
    args.extend(
        ["--select", &file_select(), "--apply", APPLY, file]
            .iter()
            .map(ToString::to_string),
    );
    args
}

/// The Nix system identifier of the running binary.
pub fn current_system() -> anyhow::Result<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "linux") => Ok("x86_64-linux"),
        ("aarch64", "linux") => Ok("aarch64-linux"),
        ("x86_64", "macos") => Ok("x86_64-darwin"),
        ("aarch64", "macos") => Ok("aarch64-darwin"),
        (arch, os) => anyhow::bail!("unsupported system: {arch}-{os}"),
    }
}

#[cfg(test)]
mod tests;

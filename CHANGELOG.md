<!--
SPDX-FileCopyrightText: 2026 Lord-Valen

SPDX-License-Identifier: CC0-1.0
-->

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Entries are added under **Unreleased** in the same change that warrants them;
CI enforces this for changes touching `src/` or `nix/`.

## [Unreleased]

### Added

- Checks can nest further report entries via `passthru.children` (a list
  of `{ name; check; }`), independent of the existing flat `suite`
  grouping. Snapshot checks use it to expose their wrapped check's
  actual output, named after that check's own name, as a foldable
  child in the TUI (folded by default).
- TUI: Left/Right now fold/unfold a suite or a check's children, or
  navigate to its first child/parent once already unfolded. Tab
  switches keyboard focus between the check list and the detail panel;
  while the detail panel has focus, Left/Right scroll it instead.
  Ctrl+Left/Ctrl+Right swap stdout/stderr focus (moved off Tab).

### Fixed

- TUI check list no longer resets its scroll position on every render.
  Previously the viewport recomputed from scratch each frame, so moving
  the selection could jump the whole list instead of just scrolling the
  minimum needed to keep the selection visible.

## [2.0.0] - 2026-06-12

### Changed

- **Breaking:** `mkEval` is now eval-only.
  It takes just the test attrset (no name argument) and returns plain data
  (`{ kind = "eval"; tests; }`) instead of a result check derivation.
  Eval checks never touch the store.
- **Breaking:** every attribute of an eval check is a test.
  Unlike `lib.debug.runTests`, names need not start with `test`.
- **Breaking:** `resultChecks.report` covers derivation checks only.
  Eval results are exposed through the new `evalChecks` option instead,
  so report builds no longer force eval tests sequentially.
  The complete merged report is available as `nrc --stream` output.
- **Breaking:** `flake.checks` integration is now a single aggregate
  `resultChecks` gate instead of one wrapper per check.
  Derivation checks still build in parallel as its dependencies;
  the per-test report is printed in its build log.
- `mkSkip` accepts eval checks, marking every test skipped without
  evaluating it.

### Added

- `mkEntries`: computes per-test result entries
  (`{ kind; status; exitCode; stdout; stderr; }`) for an eval check.
  Snapshot eval results by comparing against a pinned attrset in
  another eval test.
- Reserved flake output `resultChecks.<system> = { report; evalChecks; }`,
  the single attribute runners need.
  Flake-parts partition users must add `resultChecks` to `partitionedAttrs`.
- `resultChecks.evalChecks` module option.
- The value nrc consumes, `{ report; evalChecks; }`, is the contract.
  `mkReport` and `mkEvalChecks` produce its two halves from one check
  set (the same shape `resultChecks.checks` accepts); the flake-parts
  module is a pure shim over them.
- `nrc --file checks.nix` runs a file evaluating to that value like
  the flake convention: report build and parallel evaluation through
  nix-build, nix-eval-jobs, and nix-instantiate — no flakes or
  nix-command required. `-A attr` keeps the report-only behaviour.
- nrc convention mode: `nrc --flake <ref>` (no fragment) builds the report
  and evaluates eval checks in parallel through nix-eval-jobs,
  merging both streams.
  `--workers`/`-j` controls eval parallelism.
  Without nix-eval-jobs on PATH, falls back to sequential `nix eval --json`.
  `nrc --flake <ref>#attr` retains the report-only behaviour.
- nrc packages wrap nix-eval-jobs onto PATH.

### Removed

- The KDL report generator. JSON is the report format.
- The `resultChecks.reportGenerator` option.
  The report is the protocol consumed by nrc and the flake check gate;
  custom presentation belongs downstream of `nrc --stream` output.

### Migration

- `mkEval "name" { ... }` → `mkEval { ... }`.
  The checks attribute key names the suite.
- Helper values that rode along in a `runTests`-style attrset
  (ignored by its `test*` name filter) now fail loudly;
  move them into a `let` binding.
- `mkSnapshot ... <| mkEval ...` no longer works
  (eval checks have no derivation outputs).
  Snapshot at eval level instead:
  `{ expr = mkEntries (mkEval { ... }); expected = { ... }; }`.
- Anything reading eval results from the report derivation should read
  `resultChecks.<system>.evalChecks` or consume `nrc --stream` output.
- CI invoking per-check flake checks should depend on the single
  `checks.<system>.resultChecks` gate.

## [1.0.0] - 2026-05-28

### Added

- Initial release.
- Build support: `mkResult`, `mkResultWith`, `mkSnapshot`, `mkSnapshotWith`,
  `mkSkip`, `mkEval` — result checks as derivations with `stdout`, `stderr`,
  and `exitCode` outputs that always build; failures are captured, not fatal.
- flake-parts module: `resultChecks.checks` (flat checks and suites),
  `skipChecks`, report generation, and flake check wrappers.
- Report generators: JSON (NDJSON) and KDL.
- nrc TUI: streaming report viewer with suites, pass/fail/skip counts,
  watch mode, `--stream` NDJSON output, and configurable keybindings.
- Documentation as packages: mdBook site, man pages, options reference.

[Unreleased]: https://github.com/Lord-Valen/nix-result-checks/compare/2.0.0...HEAD
[2.0.0]: https://github.com/Lord-Valen/nix-result-checks/compare/1.0.0...2.0.0
[1.0.0]: https://github.com/Lord-Valen/nix-result-checks/releases/tag/1.0.0

<!--
SPDX-FileCopyrightText: 2026 Lord-Valen

SPDX-License-Identifier: CC0-1.0
-->

# nix-result-checks

A derivation-based testing framework for Nix.

## Design Principles

**Result monad.**
Every build check produces a derivation with four outputs:
`out` (artifacts), `stdout`, `stderr`, and `exitCode`.
The derivation itself always succeeds:
failures are captured in `exitCode` rather than propagated.
This means all checks build in parallel regardless of failures.

**Tests at their own level.**
Build behaviour is tested by derivations (`mkResult`, `mkSnapshot`);
pure Nix logic is tested at evaluation time (`mkEval`),
as plain data that never touches the store.
Runners evaluate eval tests in parallel via nix-eval-jobs.

**Uniform entries.**
Every check, built or evaluated,
reduces to the same entry shape
(`kind`, `status`, `exitCode`, `stdout`, `stderr`).
Generators and tooling consume any check without special-casing.

**Composable skip.**
Skipping is orthogonal to check type.
Any check can be skipped via `mkSkip`, the `passthru.skip` attribute,
or the flake-parts `skipChecks` option.
Skipped checks produce empty outputs and do not build their dependencies.

**Reports are a protocol.**
The JSON report is the wire format consumed by nrc and the flake check gate.
Custom presentation belongs downstream:
render from `nrc --stream` output or from the entry data itself.

## API

All functions are available under `pkgs.resultChecks` via the overlay.

### Build Support

| Function         | Signature                                                 | Description                                                                                                                                                                            |
| ---------------- | --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mkResult`       | `name: command: drv`                                      | Run a command, capturing stdout, stderr, and exit code. Sets `passthru.kind = "result"`.                                                                                               |
| `mkResultWith`   | `attrs: drv`                                              | Low-level `mkResult`. Pass `command` for capture-wrapped execution or `buildCommand` for full control. All `mkDerivation` attrs are supported.                                         |
| `mkSnapshot`     | `name: { exitCode?, stdout?, stderr? }: resultCheck: drv` | Assert a result check's outputs match expected values. Comparison is byte-exact via `cmp`. Sets `passthru.kind = "snapshot"`.                                                          |
| `mkSnapshotWith` | `attrs: drv`                                              | Low-level `mkSnapshot`. Accepts all `mkResultWith` attrs alongside snapshot-specific keys.                                                                                             |
| `mkEval`         | `tests: evalCheck`                                        | Declare eval tests (`{ expr; expected; }`, the `lib.debug.runTests` format). Returns plain data tagged `kind = "eval"` — no derivation. Every attribute is a test, regardless of name. |
| `mkEntries`      | `evalCheck: attrs`                                        | Compute per-test entries (`{ kind; status; exitCode; stdout; stderr; }`) for an eval check, lazily: a test only evaluates when its entry is forced.                                   |
| `mkReport`       | `checks: drv`                                             | Report package for the derivation half of a check set (flat checks and suites; eval checks ignored).                                                                                  |
| `mkEvalChecks`   | `checks: attrs`                                           | Lazy entries tree for the eval half of a check set, keyed by check then test — the shape behind `resultChecks.<system>.evalChecks`.                                                   |
| `mkSkip`         | `(drv \| evalCheck) -> same`                              | Skip any check. Derivations are overridden to produce empty outputs with no dependencies built; eval checks are marked so their tests are never evaluated.                             |

### Generators

| Package | Description                                                                             |
| ------- | --------------------------------------------------------------------------------------- |
| `json`  | Generates a JSON report from check results. Override `checks` to provide the check set. |

## Usage

### With flake-parts

Add `nix-result-checks` as an input and import the flake module:

```nix
# flake.nix
{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nix-result-checks.url = "github:Lord-Valen/nix-result-checks";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.nix-result-checks.flakeModules.default ];

      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem = { config, pkgs, ... }: {
        resultChecks.checks =
          let inherit (pkgs) resultChecks;
          in {
            # Flat check
            my-test = resultChecks.mkResult "my-test" ''
              echo "running tests..."
              exit 0
            '';
            # Suite: grouped checks under a named header
            db = {
              schema = resultChecks.mkResult "db-schema" "exit 0";
              migrations = resultChecks.mkResult "db-migrations" "exit 0";
            };
          };

        # Expose the report as a package
        packages.checks-report = config.resultChecks.report;
      };
    };
}
```

#### Options

| Option                           | Type                                                | Default        | Description                                                                                                                              |
| -------------------------------- | --------------------------------------------------- | -------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `resultChecks.enable`            | `bool`                                              | `true`         | Enable the module.                                                                                                                       |
| `resultChecks.checks`            | `attrsOf (package \| evalCheck \| attrsOf package)` | `{}`           | Flat checks, eval checks, or suites. Flat: `name = drv`. Eval: `name = mkEval { ... }`. Suite: `suite-name = { check-name = drv; ... }`. |
| `resultChecks.skipChecks`        | `listOf str`                                        | `[]`           | Check keys to skip. Flat checks: `"name"`. Suite checks and eval tests: `"suite:name"`.                                                  |
| `resultChecks.enableFlakeChecks` | `bool`                                              | `true`         | Add the aggregate `resultChecks` gate to `flake.checks`.                                                                                 |
| `resultChecks.report`            | `package`                                           | (derived)      | The generated report, covering derivation checks only. Read-only.                                                                        |
| `resultChecks.evalChecks`        | `lazyAttrsOf (lazyAttrsOf raw)`                     | (derived)      | Per-test entries of all eval checks, keyed by check then test name. Computed lazily so runners can force them in parallel. Read-only.    |

The module also defines the reserved flake output
`resultChecks.<system> = { report; evalChecks; }` —
the single attribute runners need.
If you use flake-parts partitions,
add `resultChecks` to `partitionedAttrs` or the output is silently dropped.

#### Flake Check Integration

A single aggregate `resultChecks` check is added to `flake.checks`.
It depends on every derivation check (built in parallel by the scheduler),
bakes in eval verdicts at evaluation time,
prints the full per-check report in its build log,
and fails if any check failed:

```console
$ nix flake check
```

### Without flake-parts

Everything nrc consumes is one value:

```
{ report :: drv; evalChecks :: { check -> test -> entry }; }
```

The flake-parts module is a shim that produces this value
at the `resultChecks.<system>` flake output;
without flake-parts, you produce it yourself:

```nix
# flake.nix, no flake-parts
outputs = { nixpkgs, nix-result-checks, ... }:
  let
    pkgs = import nixpkgs {
      system = "x86_64-linux";
      overlays = [ nix-result-checks.overlays.default ];
    };
    rc = pkgs.resultChecks;

    # The same shape resultChecks.checks accepts.
    checks = {
      my-test = rc.mkResult "my-test" "exit 0";
      my-lib = rc.mkEval {
        testAdd = {
          expr = 1 + 1;
          expected = 2;
        };
      };
    };
  in
  {
    resultChecks.x86_64-linux = {
      report = rc.mkReport checks;
      evalChecks = rc.mkEvalChecks checks;
    };
  };
```

`nrc --flake .` consumes this shape
exactly as it consumes the module-generated one.

### Without flakes

The same data type, in a file.
`default.nix` exposes the overlay for plain imports:

```nix
# checks.nix
let
  nix-result-checks = import ./path/to/nix-result-checks;
  pkgs = import <nixpkgs> {
    overlays = [ nix-result-checks.overlays.default ];
  };
  rc = pkgs.resultChecks;

  # The same shape resultChecks.checks accepts.
  checks = {
    my-test = rc.mkResult "my-test" "exit 0";
    my-lib = rc.mkEval {
      testAdd = {
        expr = 1 + 1;
        expected = 2;
      };
    };
  };
in
{
  report = rc.mkReport checks;
  evalChecks = rc.mkEvalChecks checks;
}
```

```console
$ nrc --file ./checks.nix
```

Same experience as flake mode:
the report builds while the eval entries evaluate in parallel,
through nix-build and nix-instantiate when nix-eval-jobs is absent,
so neither flakes nor nix-command is required.
`-A attr` builds a single attribute as a report instead.

## Artifact Reuse

A check that produces something expensive can share it:
the Nix store is the fixture cache.
Downstream checks reference the artifact path,
which doubles as the build ordering;
unchanged setup is never rebuilt across runs.

When the setup is not itself under test,
keep it a plain derivation:

```nix
{
  fixture = pkgs.runCommand "test-db" { } "expensive-setup > $out";
  schema = resultChecks.mkResult "schema" "validate ${fixture}";
  queries = resultChecks.mkResult "queries" "run-queries ${fixture}";
}
```

A failed fixture fails its dependents as ordinary build failures.

When the setup's own verdict belongs in the report,
make it a result check that writes the artifact to `$out`,
and guard downstream on its exit code to fail gracefully:

```nix
{
  fixture = resultChecks.mkResult "make-db" "expensive-setup > $out";

  queries = resultChecks.mkResult "queries" ''
    [ "$(cat ${checks.fixture.exitCode})" = "0" ] || {
      echo "setup failed" >&2
      exit 1
    }
    run-queries ${checks.fixture}
  '';
}
```

Since result checks always succeed as derivations
(failures are captured in `exitCode`),
dependent checks can inspect the exit code, log,
or artifacts of their dependencies and decide how to proceed.

## Eval Tests

`mkEval` declares pure Nix eval-time tests
using the same `{ expr; expected; }` format as `lib.debug.runTests`:

```nix
resultChecks.checks.my-lib = resultChecks.mkEval {
  testAddition = {
    expr = 1 + 1;
    expected = 2;
  };
  testConcat = {
    expr = "hello" + " " + "world";
    expected = "hello world";
  };
};
```

An eval check is plain data — no derivation, no store access.
Registered in `resultChecks.checks`,
it displays as a suite with one entry per test,
and runners evaluate the tests in parallel through nix-eval-jobs.

Unlike `lib.debug.runTests`, every attribute is a test:
a value that is not `{ expr, expected }`-shaped fails loudly
instead of being filtered by name.
Keep helpers in a `let` binding.

A failing test's entry carries a formatted report in `stdout`:

```
FAIL: testAddition
  expected: 3
  got:      2
```

On success, `stdout` and `stderr` are empty and `exitCode` is `"0"`.

To snapshot eval results,
pin the entries in another eval test.
`mkEntries` makes the verdicts themselves plain values:

```nix
resultChecks.checks.meta = resultChecks.mkEval {
  testFailureFormatting = {
    expr = resultChecks.mkEntries (resultChecks.mkEval {
      broken = {
        expr = 1 + 1;
        expected = 3;
      };
    });
    expected.broken = {
      kind = "eval";
      status = "fail";
      exitCode = "1";
      stdout = "FAIL: broken\n  expected: 3\n  got:      2\n";
      stderr = "1 test(s) failed\n";
    };
  };
};
```

## Skipping Checks

There are three ways to skip a check:

**`mkSkip`** wraps any check to skip it:

```nix
resultChecks.mkResult "my-test" ''
  echo "expensive test"
  exit 0
''
|> resultChecks.mkSkip
```

**`passthru.skip`** skips from within `mkResultWith`:

```nix
resultChecks.mkResultWith {
  name = "result-my-test";
  passthru.skip = true;
  command = ''
    echo "expensive test"
    exit 0
  '';
}
```

**`skipChecks`** skips by name via flake-parts configuration:

```nix
resultChecks.skipChecks = [ "my-test" ];
```

All three mechanisms produce the same result:
empty outputs, no dependencies built, and `status="skip"` in the report.

Eval checks skip the same way:
`mkSkip` marks the whole check,
and `skipChecks = [ "check:test" ]` skips a single test.
Skipped eval tests are never evaluated;
a skipped test may contain a `throw`.

## Running Checks with nrc

`nrc --flake .` follows the `resultChecks.<system>` convention:
it builds the report (derivation checks)
while nix-eval-jobs forces the eval test entries in parallel,
and merges both streams into one view.
`--workers`/`-j` controls eval parallelism.
Without nix-eval-jobs on PATH,
eval checks are fetched sequentially via `nix eval --json` instead.
The packaged `nrc` wraps nix-eval-jobs,
so the fallback only applies to ad-hoc builds.

`nrc --flake .#some-attr` builds that attribute as a report file
and skips the eval side.
`nrc --stream --flake .` emits the merged results as newline-delimited JSON
and exits non-zero on failure —
the complete report for CI without building one store artifact per run.

## Output Conventions

All outputs use `printf '%s'`:
files contain exactly the bytes written,
with no added trailing newlines.
User command output (stdout/stderr from `mkResult`)
is captured verbatim via shell redirection.
Exit codes are stored as plain digit strings (e.g. `0`, not `0\n`).

`mkSnapshot` comparison is byte-exact via `cmp`.
When writing expected values for commands that use `echo` (which adds `\n`),
use multiline `''` strings with the closing `''` on its own line,
which naturally includes the trailing newline:

```nix
mkSnapshot "my-test" {
  exitCode = "0";
  stdout = ''
    Test 1: PASS
    Test 2: PASS
  '';
}
<| checks.my-test
```

## Report Format

The default JSON report is newline-delimited JSON.
Each line is one check:

```json
{"kind":"result","status":"pass","name":"my-test","suite":null,"exitCode":"0","stdout":"test output\n","stderr":"","drvPath":"/nix/store/..."}
{"kind":"snapshot","status":"fail","name":"my-snapshot","suite":null,"exitCode":"1","stdout":"","stderr":"Stdout mismatch\n...","drvPath":"/nix/store/..."}
{"kind":"result","status":"skip","name":"skipped-test","suite":null,"exitCode":"","stdout":"","stderr":"","drvPath":"/nix/store/..."}
{"kind":"result","status":"pass","name":"schema","suite":"db","exitCode":"0","stdout":"","stderr":"","drvPath":"/nix/store/..."}
```

`kind` is one of `result`, `snapshot`, `eval`.
`status` is one of `pass`, `fail`, `skip`.
`suite` is the suite name for grouped checks,
or `null` for flat checks.

The report file covers derivation checks only;
eval check entries travel through `resultChecks.<system>.evalChecks`
and appear alongside report entries in nrc and `--stream` output
without a `drvPath`: there is no derivation.

The `drv` field is the store path of the check derivation.
It is useful for locating the derivation's outputs directly
(e.g. `${drv.stdout}`, `${drv.stderr}`).
Note that `nix log <drvPath>` shows the builder's own stderr —
not the command output,
which is always redirected into the derivation's output files.

## Development

Library code (`nix/`, excluding `nix/dev/`)
must evaluate with stable Nix plus only the `nix-command` and `flakes` features.
Consumers are never required to enable anything else.

Experimental syntax — currently the pipe operators `|>` and `<|` —
is permitted only inside `nix/dev/`.
CI enables `pipe-operators` solely because
building the partitioned docs package parses the dev tests.
Doc-comment examples may show pipe operators:
comments are not parsed.

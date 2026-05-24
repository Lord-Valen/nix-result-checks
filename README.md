<!--
SPDX-FileCopyrightText: 2026 Lord-Valen

SPDX-License-Identifier: CC0-1.0
-->

# nix-result-checks

A derivation-based testing framework for Nix.

## Design Principles

**Result monad.** Every check produces a derivation with four outputs: `out` (artifacts), `stdout`, `stderr`, and `exitCode`. The derivation itself always succeeds — failures are captured in `exitCode` rather than propagated. This means all checks build in parallel regardless of failures.

**Uniform interface.** All check types (`mkResult`, `mkSnapshot`, `mkEval`) produce derivations with the same shape. Generators and tooling can consume any check without special-casing.

**Composable skip.** Skipping is orthogonal to check type. Any check can be skipped via `mkSkip`, the `passthru.skip` attribute, or the flake-parts `skipChecks` option. Skipped checks produce empty outputs and do not build their dependencies.

**Pluggable reporting.** Check results are plain derivations. Generators transform them into reports in any format. The default generator produces JSON.

## API

All functions are available under `pkgs.resultChecks` via the overlay.

### Build Support

| Function | Signature | Description |
|---|---|---|
| `mkResult` | `name: command: drv` | Run a command, capturing stdout, stderr, and exit code. Sets `passthru.kind = "result"`. |
| `mkResultWith` | `attrs: drv` | Low-level `mkResult`. Pass `command` for capture-wrapped execution or `buildCommand` for full control. All `mkDerivation` attrs are supported. |
| `mkSnapshot` | `name: { exitCode?, stdout?, stderr? }: resultCheck: drv` | Assert a result check's outputs match expected values. Comparison is byte-exact via `cmp`. Sets `passthru.kind = "snapshot"`. |
| `mkSnapshotWith` | `attrs: drv` | Low-level `mkSnapshot`. Accepts all `mkResultWith` attrs alongside snapshot-specific keys. |
| `mkEval` | `name: tests: drv` | Run `lib.debug.runTests`-style eval tests (`{ expr; expected; }`). Formats failures as `FAIL: name / expected: X / got: Y`. Sets `passthru.kind = "eval"`. |
| `mkSkip` | `drv: drv` | Skip any check. Overrides the derivation to produce empty outputs, clearing `buildInputs` and `nativeBuildInputs` so dependencies are not built. |

### Generators

| Package | Description |
|---|---|
| `json` | Generates a JSON report from check results. Override `checks` to provide the check set. |
| `kdl` | Generates a KDL report from check results. Override `checks` to provide the check set. |

## Usage with flake-parts

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

### flake-parts Options

| Option | Type | Default | Description |
|---|---|---|---|
| `resultChecks.enable` | `bool` | `true` | Enable the module. |
| `resultChecks.checks` | `attrsOf (package \| attrsOf package)` | `{}` | Flat checks or suites. Flat: `name = drv`. Suite: `suite-name = { check-name = drv; ... }`. |
| `resultChecks.skipChecks` | `listOf str` | `[]` | Check keys to skip. Flat checks: `"name"`. Suite checks: `"suite:name"`. |
| `resultChecks.enableFlakeChecks` | `bool` | `true` | Bridge non-skipped checks into `flake.checks` so `nix flake check` reports failures. |
| `resultChecks.reportGenerator` | `checks -> package` | JSON generator | Function producing a report package from the checks attrset. |
| `resultChecks.report` | `package` | (derived) | The generated report. Read-only. |

### Flake Check Integration

Non-skipped checks are automatically bridged into `flake.checks`, so `nix flake check` reports failures:

```console
$ nix flake check
```

## Test Orchestration

Tests can depend on other tests via `nativeBuildInputs` in `mkResultWith`. This creates build-time dependencies, ensuring ordering:

```nix
{
  unit-tests = resultChecks.mkResult "unit-tests" ''
    echo "running unit tests..."
    exit 0
  '';

  integration = (resultChecks.mkResult "integration" ''
    exitCode=$(cat ${checks.unit-tests.exitCode})
    if [ "$exitCode" != "0" ]; then
      echo "unit tests failed, skipping integration"
      exit 1
    fi
    echo "running integration tests..."
    exit 0
  '').overrideAttrs {
    nativeBuildInputs = [ checks.unit-tests ];
  };
}
```

`integration` will not build until `unit-tests` completes. Since results always succeed as derivations (failures are captured in `exitCode`), dependent tests can inspect the exit code, log, or artifacts of their dependencies and decide how to proceed.

## Skipping Checks

There are three ways to skip a check:

**`mkSkip` function** — wrap any check to skip it:

```nix
resultChecks.mkResult "my-test" ''
  echo "expensive test"
  exit 0
''
|> resultChecks.mkSkip
```

**`passthru.skip` attribute** — skip via `mkResultWith`:

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

**`skipChecks` option** — skip by name via flake-parts configuration:

```nix
resultChecks.skipChecks = [ "my-test" ];
```

All three mechanisms produce the same result: empty outputs, no dependencies built, and `status="skip"` in the report.

## Eval Tests

`mkEval` runs pure Nix eval-time tests using the same `{ expr; expected; }` format as `lib.debug.runTests`:

```nix
resultChecks.mkEval "my-eval-tests" {
  testAddition = {
    expr = 1 + 1;
    expected = 2;
  };
  testConcat = {
    expr = "hello" + " " + "world";
    expected = "hello world";
  };
}
```

On failure, stdout contains a formatted report:

```
FAIL: testAddition
  expected: 3
  got:      2
```

On success, all outputs are empty (exitCode is `"0"`). The failure count is written to stderr as a convenience (e.g. `"2 test(s) failed"`).

## Output Conventions

All outputs use `printf '%s'` — files contain exactly the bytes written, with no added trailing newlines. User command output (stdout/stderr from `mkResult`) is captured verbatim via shell redirection. Exit codes are stored as plain digit strings (e.g. `0`, not `0\n`).

`mkSnapshot` comparison is byte-exact via `cmp`. When writing expected values for commands that use `echo` (which adds `\n`), use multiline `''` strings with the closing `''` on its own line — this naturally includes the trailing newline:

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

## Usage without flake-parts

The overlay and flake module are available without a flake via `default.nix`:

```nix
let
  nix-result-checks = import ./path/to/nix-result-checks;
in
{
  # Apply the overlay to your pkgs
  overlays = [ nix-result-checks.overlays.default ];

  # Or import the flake module for use with flake-parts outside a flake
  imports = [ nix-result-checks.flakeModules.default ];
}
```

## Report Format

The default JSON report is newline-delimited JSON. Each line is one check:

```json
{"kind":"result","status":"pass","name":"my-test","suite":null,"exitCode":"0","stdout":"test output\n","stderr":"","drvPath":"/nix/store/..."}
{"kind":"snapshot","status":"fail","name":"my-snapshot","suite":null,"exitCode":"1","stdout":"","stderr":"Stdout mismatch\n...","drvPath":"/nix/store/..."}
{"kind":"result","status":"skip","name":"skipped-test","suite":null,"exitCode":"","stdout":"","stderr":"","drvPath":"/nix/store/..."}
{"kind":"result","status":"pass","name":"schema","suite":"db","exitCode":"0","stdout":"","stderr":"","drvPath":"/nix/store/..."}
```

`kind` is one of `result`, `snapshot`, `eval`. `status` is one of `pass`, `fail`, `skip`. `suite` is the suite name for grouped checks, or `null` for flat checks.

The `drv` field is the store path of the check derivation. It is useful for locating the derivation's outputs directly (e.g. `${drv.stdout}`, `${drv.stderr}`). Note that `nix log <drvPath>` shows the builder's own stderr — not the command output, which is always redirected into the derivation's output files.

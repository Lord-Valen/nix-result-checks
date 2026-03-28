# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Run pure Nix evaluation tests as a result check.

  Tests are evaluated at Nix eval time using `lib.debug.runTests` — no
  sandbox, no builder, no I/O. The outcomes are then written into a
  result check derivation at build time. Failures are reported in the
  `stderr` output; the derivation always succeeds.

  Use this for testing pure Nix functions. For testing shell commands or
  build-time behaviour, use `mkResult` or `mkResultSnapshot`.

  # Type

  ```
  mkEvalTests :: String -> AttrSet -> AttrSet -> Derivation
  ```

  # Arguments

  name
  : Check name. Becomes the derivation name `eval-tests-<name>`.

  env
  : Extra derivation attributes (see `mkResult`). Pass `{ }` if unused.

  tests
  : Attribute set of test cases in `lib.debug.runTests` format. Each
    entry must have `expr` (the value under test) and `expected` (the
    expected value).

  # Example

  ```nix
  mkEvalTests "my-lib" { }
    {
      testAdd = {
        expr = myLib.add 1 2;
        expected = 3;
      };
    }
  ```
*/
{ lib, mkResultWith }:
name: env: tests:
let
  failures = lib.debug.runTests tests;
  failureCount = builtins.length failures;
  formatFailure =
    { name, expected, result }:
    "FAIL: ${name}\n  expected: ${lib.generators.toPretty { } expected}\n  got:      ${lib.generators.toPretty { } result}";
  report = lib.concatMapStringsSep "\n" formatFailure failures;
  failed = failureCount > 0;
  stdout = lib.optionalString failed "${report}\n";
  stderrMsg = lib.optionalString failed "${toString failureCount} test(s) failed\n";
  exitCode = if failed then "1" else "0";
in
mkResultWith
  {
    name = "eval-tests-${name}";
    derivationArgs = lib.recursiveUpdate {
      passthru.type = "eval";
    } env;
  }
  ''
    touch "$out"
    printf '%s' ${lib.escapeShellArg stdout} > "$stdout"
    printf '%s' ${lib.escapeShellArg stderrMsg} > "$stderr"
    printf '%s' ${lib.escapeShellArg exitCode} > "$exitCode"
    exit 0
  ''

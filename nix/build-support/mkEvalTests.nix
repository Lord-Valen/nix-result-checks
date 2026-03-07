# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

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

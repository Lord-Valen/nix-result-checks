# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Compute per-test result entries for an eval check.

  Each test in the check becomes an entry of the form

  ```
  { kind = "eval"; status; exitCode; stdout; stderr; }
  ```

  where `status` is `"pass"`, `"fail"`, or `"skip"`. Failures carry a
  formatted report in `stdout` and a summary in `stderr`, matching the
  conventions of derivation-based checks. Entries are computed lazily:
  a test's expression is only forced when its entry is, and skipped
  tests are never forced at all.

  Skipping is controlled by `skip = true` on the check (set by
  `mkSkip`) or by listing test names in `skipTests` on the check.

  # Type

  ```
  mkEntries :: EvalCheck -> AttrSet
  ```

  # Arguments

  check
  : An eval check produced by `mkEval`.

  # Example

  ```nix
  mkEntries (mkEval {
    testAdd = {
      expr = 1 + 2;
      expected = 3;
    };
  })
  => { testAdd = { kind = "eval"; status = "pass"; exitCode = "0"; stdout = ""; stderr = ""; }; }
  ```
*/
{ lib }:
check:
let
  skipEntry = {
    kind = "eval";
    status = "skip";
    exitCode = "";
    stdout = "";
    stderr = "";
  };

  runEntry =
    name: test:
    let
      # Direct comparison rather than lib.debug.runTests: runTests only
      # runs attributes named test*, which would silently pass any other
      # name. Every attribute of an eval check is a test.
      failed = test.expr != test.expected;
      report = "FAIL: ${name}\n  expected: ${lib.generators.toPretty { } test.expected}\n  got:      ${
        lib.generators.toPretty { } test.expr
      }";
    in
    {
      kind = "eval";
      status = if failed then "fail" else "pass";
      exitCode = if failed then "1" else "0";
      stdout = lib.optionalString failed "${report}\n";
      stderr = lib.optionalString failed "1 test(s) failed\n";
    };

  skipped = name: (check.skip or false) || lib.elem name (check.skipTests or [ ]);
in
lib.mapAttrs (name: test: if skipped name then skipEntry else runEntry name test) check.tests

# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Declare a suite of pure Nix evaluation tests as an eval check.

  An eval check is plain data — no derivation, no store access. Tests
  are evaluated lazily, one attribute per test, so runners such as
  `nrc` can shard them across `nix-eval-jobs` workers. Use
  `mkEntries` to compute the per-test results.

  Registered in `resultChecks.checks`, an eval check displays as a
  suite with one entry per test. Use this for testing pure Nix
  functions. For testing shell commands or build-time behaviour, use
  `mkResult` or `mkSnapshot`.

  # Type

  ```
  mkEval :: AttrSet -> EvalCheck
  ```

  # Arguments

  tests
  : Attribute set of test cases. Each entry must have `expr` (the
    value under test) and `expected` (the expected value). Unlike
    `lib.debug.runTests`, every attribute is a test regardless of
    its name.

  # Example

  ```nix
  resultChecks.checks.my-lib = mkEval {
    testAdd = {
      expr = myLib.add 1 2;
      expected = 3;
    };
  };
  ```
*/
{ }:
tests: {
  kind = "eval";
  inherit tests;
}

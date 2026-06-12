# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Compute the entries tree for the eval checks in a check set.

  Takes checks in the same shape as `resultChecks.checks`
  and computes per-test entries for the eval half,
  keyed by check then test name.
  Entries are lazy, so runners can force them in parallel.
  This is the `evalChecks` half of the value nrc consumes,
  behind the `resultChecks.<system>` flake output
  and the `--file` convention.

  # Type

  ```
  mkEvalChecks :: AttrSet -> AttrSet
  ```

  # Arguments

  checks
  : Attribute set of checks.
    Derivation checks are ignored;
    they are covered by `mkReport`.

  # Example

  ```nix
  mkEvalChecks {
    my-lib = mkEval {
      testAdd = {
        expr = 1 + 2;
        expected = 3;
      };
    };
  }
  => { my-lib.testAdd = { kind = "eval"; status = "pass"; exitCode = "0"; stdout = ""; stderr = ""; }; }
  ```
*/
{ lib, mkEntries }:
checks:
lib.mapAttrs (_name: mkEntries) (
  lib.filterAttrs (_name: value: value.kind or null == "eval") checks
)

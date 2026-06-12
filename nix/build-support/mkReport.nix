# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Generate the report package for the derivation checks in a check set.

  Takes checks in the same shape as `resultChecks.checks` —
  flat derivations, suites, and eval checks —
  and reports on the derivation half.
  Eval checks are not derivations;
  they are covered by `mkEvalChecks` instead.

  # Type

  ```
  mkReport :: AttrSet -> Derivation
  ```

  # Arguments

  checks
  : Attribute set of checks.
    Flat: `name = drv`.
    Suite: `name = { checkName = drv; ... }`.
    Eval checks are ignored.

  # Example

  ```nix
  mkReport {
    my-test = mkResult "my-test" "exit 0";
    db.schema = mkResult "db-schema" "exit 0";
  }
  ```
*/
{ lib, json }:
checks:
let
  drvChecks = lib.filterAttrs (_name: value: value.kind or null != "eval") checks;

  # Normalize to the generator's { key -> { check, suite } } shape.
  # Flat checks: key = name, suite = null.
  # Suite checks: key = "suite:name", suite = suite name.
  normalize =
    outerName: value:
    # Discriminate via drvPath presence rather than lib.isDerivation:
    # user attrsets could shadow type.
    if value ? drvPath then
      {
        ${outerName} = {
          check = value;
          suite = null;
        };
      }
    else
      lib.mapAttrs' (
        checkName: check:
        lib.nameValuePair "${outerName}:${checkName}" {
          inherit check;
          suite = outerName;
        }
      ) value;
in
json.override { checks = lib.concatMapAttrs normalize drvChecks; }

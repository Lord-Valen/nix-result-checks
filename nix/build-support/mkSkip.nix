# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Mark a check as skipped.

  For result check derivations,
  clears all build inputs,
  replaces the build command with a no-op,
  and sets `passthru.skip = true`,
  which generators use to report the check as skipped
  rather than passed or failed.

  For eval checks, sets `skip = true`;
  `mkEntries` then reports every test as skipped
  without forcing its expression.

  # Type

  ```
  mkSkip :: (Derivation | EvalCheck) -> (Derivation | EvalCheck)
  ```

  # Arguments

  check
  : A result check derivation
    produced by `mkResult`, `mkResultWith`, or `mkSnapshot`,
    or an eval check produced by `mkEval`.

  # Example

  ```nix
  mkResult "my-check" "echo hello" |> mkSkip
  ```
*/
{ lib }:
check:
if !lib.isDerivation check && check.kind or null == "eval" then
  check // { skip = true; }
else
  check.overrideAttrs (prev: {
    passthru = (prev.passthru or { }) // {
      skip = true;
    };
    buildCommand = ''
      touch $out
      touch $stdout
      touch $stderr
      touch $exitCode
    '';
    buildInputs = [ ];
    nativeBuildInputs = [ ];
  })

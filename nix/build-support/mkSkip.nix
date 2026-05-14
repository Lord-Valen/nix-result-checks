# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Mark a result check derivation as skipped.

  Clears all build inputs and replaces the build command with a no-op.
  Sets `passthru.skip = true`, which generators use to report the check
  as skipped rather than passed or failed.

  # Type

  ```
  mkSkip :: Derivation -> Derivation
  ```

  # Arguments

  drv
  : A result check derivation produced by `mkResult`, `mkResultWith`,
    `mkSnapshot`, or `mkEval`.

  # Example

  ```nix
  mkResult "my-check" "echo hello" |> mkSkip
  ```
*/
{ }:
drv:
drv.overrideAttrs (prev: {
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

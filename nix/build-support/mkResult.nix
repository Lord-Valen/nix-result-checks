# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  The primary way to define a result check.

  Runs `command` in a derivation with `set +e`. Exit code, stdout, and
  stderr are captured in separate outputs (`exitCode`, `stdout`, `stderr`)
  rather than failing the build — the derivation always succeeds regardless
  of the command's outcome.

  For extra derivation attributes (e.g. `nativeBuildInputs`), use
  `mkResultWith` directly.

  # Type

  ```
  mkResult :: String -> String -> Derivation
  ```

  # Arguments

  name
  : Check name. Becomes the derivation name `result-<name>`.

  command
  : Shell command to run as the check body.

  # Example

  ```nix
  mkResult "hello" "hello --version"
  ```

  ```nix
  mkResultWith {
    name = "result-grep-output";
    nativeBuildInputs = [ pkgs.ripgrep ];
    buildCommand = mkResult.buildCommand "rg 'pattern' somefile";
  }
  ```
*/
{ mkResultWith }:
name: command:
mkResultWith {
  name = "result-${name}";
  passthru.kind = "result";
  inherit command;
}

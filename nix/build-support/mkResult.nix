# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  The primary way to define a result check.

  Runs `command` in a derivation with `set +e`. Exit code, stdout, and
  stderr are captured in separate outputs (`exitCode`, `stdout`,
  `stderr`) rather than failing the build — the derivation always
  succeeds regardless of the command's outcome.

  Use `env` to inject build-time dependencies or set `passthru.skip =
  true` to mark the check as skipped.

  # Type

  ```
  mkResult :: String -> AttrSet -> String -> Derivation
  ```

  # Arguments

  name
  : Check name. Becomes the derivation name `result-<name>`.

  env
  : Extra derivation attributes merged via `lib.recursiveUpdate`. Pass
    `{ }` if unused. Common keys: `buildInputs`, `nativeBuildInputs`,
    `passthru`.

  command
  : Shell command to run as the check body.

  # Example

  ```nix
  mkResult "hello" { } "hello --version"
  ```

  ```nix
  mkResult "grep-output"
    { nativeBuildInputs = [ pkgs.ripgrep ]; }
    "rg 'pattern' somefile"
  ```
*/
{ lib, mkResultWith }:
name: env: command:
mkResultWith
  {
    name = "result-${name}";
    derivationArgs = lib.recursiveUpdate {
      passthru.type = "result";
    } env;
  }
  ''
    set +e
    (
      ${command}
    ) > "$stdout" 2> "$stderr"
    printf '%s' "$?" > "$exitCode"
    set -e
    touch "$out"

    # Always exit successfully - failures are captured in exitCode
    exit 0
  ''

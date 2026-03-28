# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Low-level result check builder. Prefer `mkResult` for most use cases.

  Produces a derivation with four outputs: `out` (sentinel), `stdout`,
  `stderr`, and `exitCode`. The build script is responsible for writing
  to each output path — they are available as `$out`, `$stdout`,
  `$stderr`, and `$exitCode`. The derivation always exits successfully.

  If `derivationArgs.passthru.skip` is `true`, delegates to `mkSkip`
  automatically.

  # Type

  ```
  mkResultWith :: AttrSet -> String -> Derivation
  ```

  # Arguments

  runCommandAttrs
  : Attribute set passed to `runCommandWith`. Required key: `name`.
    Optional keys: `derivationArgs` (merged with the fixed `outputs`
    list), `stdenv`. All other `runCommandWith` keys are supported.

  buildCommand
  : Shell script for the check body. Must write to `$stdout`, `$stderr`,
    `$exitCode`, and `$out`.

  # Example

  ```nix
  mkResultWith
    {
      name = "result-my-check";
      derivationArgs.nativeBuildInputs = [ pkgs.hello ];
    }
    ''
      set +e
      hello --greeting "hi" > "$stdout" 2> "$stderr"
      printf '%s' "$?" > "$exitCode"
      set -e
      touch "$out"
    ''
  ```
*/
{
  lib,
  mkSkip,
  runCommandWith,
  stdenvNoCC,
}:
{
  stdenv ? stdenvNoCC,
  ...
}@runCommandAttrs:
buildCommand:
let
  attrs = lib.recursiveUpdate {
    derivationArgs = {
      outputs = [
        "out"
        "stdout"
        "stderr"
        "exitCode"
      ];
    };
  } runCommandAttrs;
  isSkip = attrs.derivationArgs.passthru.skip or false;
  result = runCommandWith attrs buildCommand;
in
if isSkip then mkSkip result else result

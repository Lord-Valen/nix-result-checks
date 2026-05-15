# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Low-level result check builder. Prefer `mkResult` for most use cases.

  Produces a derivation with four outputs: `out` (sentinel), `stdout`,
  `stderr`, and `exitCode`. The `command` attribute is a shell script body
  whose stdout, stderr, and exit code are captured automatically. The
  derivation always exits successfully.

  For full control over the build script, use `buildCommand` directly
  (bypasses capture wrapping).

  If `passthru.skip` is `true`, delegates to `mkSkip` automatically.

  # Type

  ```
  mkResultWith :: AttrSet -> Derivation
  ```

  # Arguments

  attrs
  : Attribute set passed to `stdenvNoCC.mkDerivation`. Required keys: `name`
    and either `command` or `buildCommand`. All other `mkDerivation` keys are
    supported.

  # Example

  ```nix
  mkResultWith {
    name = "result-my-check";
    nativeBuildInputs = [ pkgs.hello ];
    command = ''hello --greeting "hi"'';
  }
  ```
*/
{
  lib,
  mkSkip,
  stdenvNoCC,
}:
lib.extendMkDerivation {
  constructDrv = stdenvNoCC.mkDerivation;
  excludeDrvArgNames = [
    "command"
    "buildCommand"
  ];
  extendDrvArgs =
    _finalAttrs:
    {
      command ? null,
      ...
    }@args:
    {
      outputs = [
        "out"
        "stdout"
        "stderr"
        "exitCode"
      ];
      buildCommand =
        args.buildCommand or ''
          set +e
          (
            ${command}
          ) > "$stdout" 2> "$stderr"
          printf '%s' "$?" > "$exitCode"
          set -e
          touch "$out"
          exit 0
        '';
    };
  transformDrv = drv: if drv.passthru.skip or false then mkSkip drv else drv;
}

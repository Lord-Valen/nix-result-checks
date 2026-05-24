# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Assert the outputs of a result check match expected values.

  Low-level form of `mkSnapshot`. Accepts the full `mkResultWith`
  attribute set alongside snapshot-specific keys. Use `mkSnapshot`
  for the common case.

  # Type

  ```
  mkSnapshotWith :: AttrSet -> Derivation
  ```

  # Arguments

  attrs
  : Attribute set. Required keys: `name`, `resultCheck`. Optional keys:
    `exitCode`, `stdout`, `stderr` (each defaults to "don't assert").
    All other `mkResultWith` keys are supported.

  # Example

  ```nix
  mkSnapshotWith {
    name = "hello-snapshot";
    resultCheck = mkResult "hello" "hello --version";
    exitCode = "0";
  }
  ```
*/
{
  lib,
  mkResultWith,
}:
lib.extendMkDerivation {
  constructDrv = mkResultWith;
  excludeDrvArgNames = [
    "resultCheck"
    "exitCode"
    "stdout"
    "stderr"
  ];
  extendDrvArgs =
    _finalAttrs:
    {
      name,
      resultCheck,
      exitCode ? null,
      stdout ? null,
      stderr ? null,
      ...
    }:
    {
      name = "snapshot-${name}";
      passthru.kind = "snapshot";
      command =
        lib.optionalString (exitCode != null) ''
          printf '%s' ${lib.escapeShellArg exitCode} > "$TMPDIR/expected-exitCode"
          if ! cmp -s ${resultCheck.exitCode} "$TMPDIR/expected-exitCode"; then
            echo "Exit code mismatch: expected ${exitCode}, got $(cat ${resultCheck.exitCode})" >&2
            exit 1
          fi
        ''
        + lib.optionalString (stdout != null) ''
          printf '%s' ${lib.escapeShellArg stdout} > "$TMPDIR/expected-stdout"
          if ! cmp -s ${resultCheck.stdout} "$TMPDIR/expected-stdout"; then
            echo "Stdout mismatch" >&2
            echo "Expected:" >&2
            cat "$TMPDIR/expected-stdout" >&2
            echo "Got:" >&2
            cat ${resultCheck.stdout} >&2
            exit 1
          fi
        ''
        + lib.optionalString (stderr != null) ''
          printf '%s' ${lib.escapeShellArg stderr} > "$TMPDIR/expected-stderr"
          if ! cmp -s ${resultCheck.stderr} "$TMPDIR/expected-stderr"; then
            echo "Stderr mismatch" >&2
            echo "Expected:" >&2
            cat "$TMPDIR/expected-stderr" >&2
            echo "Got:" >&2
            cat ${resultCheck.stderr} >&2
            exit 1
          fi
        '';
    };
}

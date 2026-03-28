# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Assert the outputs of a result check match expected values.

  Compares `exitCode`, `stdout`, and/or `stderr` of `resultCheck`
  against expected strings. Any mismatch is reported to the snapshot's
  own `stderr` output. At least one of `exitCode`, `stdout`, or `stderr`
  must be provided.

  # Type

  ```
  mkResultSnapshot :: String -> AttrSet -> AttrSet -> Derivation
  ```

  # Arguments

  name
  : Check name. Becomes the derivation name `snapshot-<name>`.

  env
  : Extra derivation attributes (see `mkResult`). Pass `{ }` if unused.

  snapshot
  : Attribute set with the following keys:

    `resultCheck` *(required)*
    : The result check derivation to test.

    `exitCode` *(optional)*
    : Expected exit code string, e.g. `"0"` or `"1"`.

    `stdout` *(optional)*
    : Expected stdout content.

    `stderr` *(optional)*
    : Expected stderr content.

  # Example

  ```nix
  mkResultSnapshot "hello-snapshot" { }
    {
      resultCheck = mkResult "hello" { } "echo hi";
      exitCode = "0";
      stdout = "hi\n";
    }
  ```
*/
{ lib, mkResultWith }:
name: env:
{
  exitCode ? null,
  stdout ? null,
  stderr ? null,
  resultCheck,
}:
mkResultWith
  {
    name = "snapshot-${name}";
    derivationArgs = lib.recursiveUpdate {
      passthru.type = "snapshot";
    } env;
  }
  (
    ''
      set +e
      (
    ''
    + lib.optionalString (exitCode != null) ''
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
    ''
    + ''
      ) > "$stdout" 2> "$stderr"
      printf '%s' "$?" > "$exitCode"
      set -e
      touch "$out"

      exit 0
    ''
  )

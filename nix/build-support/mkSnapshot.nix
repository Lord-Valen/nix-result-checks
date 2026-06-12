# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Assert the outputs of a result check match expected values.

  Compares `exitCode`, `stdout`, and/or `stderr` of the wrapped check
  against expected strings.
  Any mismatch is reported to the snapshot's own `stderr` output.
  At least one of `exitCode`, `stdout`, or `stderr` must be provided.

  For extra derivation attributes,
  use `mkSnapshotWith` directly.

  # Type

  ```
  mkSnapshot :: String -> AttrSet -> Derivation -> Derivation
  ```

  # Arguments

  name
  : Check name.
    Becomes the derivation name `snapshot-<name>`.

  expectations
  : Attribute set with the following keys:

    `exitCode` *(optional)*
    : Expected exit code string, e.g. `"0"` or `"1"`.

    `stdout` *(optional)*
    : Expected stdout content.

    `stderr` *(optional)*
    : Expected stderr content.

  resultCheck
  : The result check derivation to test.

  # Example

  ```nix
  mkSnapshot "hello-snapshot" { exitCode = "0"; stdout = "hi\n"; }
  <| mkResult "hello" "echo hi"
  ```
*/
{ mkSnapshotWith }:
name: expectations: resultCheck:
mkSnapshotWith ({ inherit name resultCheck; } // expectations)

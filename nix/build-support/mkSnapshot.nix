# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Assert the outputs of a result check match expected values.

  Compares `exitCode`, `stdout`, and/or `stderr` of `resultCheck` against
  expected strings. Any mismatch is reported to the snapshot's own `stderr`
  output. At least one of `exitCode`, `stdout`, or `stderr` must be provided.

  For extra derivation attributes, use `mkSnapshotWith` directly.

  # Type

  ```
  mkSnapshot :: String -> AttrSet -> Derivation
  ```

  # Arguments

  name
  : Check name. Becomes the derivation name `snapshot-<name>`.

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
  mkSnapshot "hello-snapshot"
    {
      resultCheck = mkResult "hello" "echo hi";
      exitCode = "0";
      stdout = "hi\n";
    }
  ```
*/
{ mkSnapshotWith }:
name: snapshot:
mkSnapshotWith ({ inherit name; } // snapshot)

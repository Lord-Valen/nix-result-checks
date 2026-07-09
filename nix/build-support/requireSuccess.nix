# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Guard a command on one or more other checks' exit codes.

  The failure message is derived from each check's own name —
  `passthru.checkName` if set (the bare name, without a mk*-added
  prefix), otherwise the derivation's own `name`. Use `requireSuccessWith`
  for a custom message instead.

  # Type

  ```
  requireSuccess :: (Derivation | [Derivation]) -> String -> String
  ```

  # Arguments

  check
  : A result check derivation, or a list of them (from `mkResult`,
    `mkResultWith`, or `mkSkip`), each contributing its `exitCode`
    output to the guard.

  command
  : Shell command to run once every check has succeeded.

  # Example

  ```nix
  mkResult "join" <| requireSuccess [ checkA checkB ] "join ${checkA} ${checkB}"
  ```
*/
{ lib, requireSuccessWith }:
check: command:
requireSuccessWith {
  checks = lib.toList check;
  message =
    c: "${c.passthru.checkName or c.name} is unavailable, we require it to succeed before proceeding";
  inherit command;
}

# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Guard a command on a list of other checks' exit codes, with an
  explicit failure message.

  Low-level form of `requireSuccess`. Prefer `requireSuccess` unless a
  check's own name doesn't say enough about what failing means to this
  particular consumer.

  A result check derivation always succeeds regardless of its command's
  outcome (see `mkResultWith`), so a check's `$out` is touched whether
  the command passed, failed, or was skipped. Consumers that trust a
  check's `stdout`/`stderr` as fixture data must check its `exitCode`
  first; this wraps `command` with that check instead of hand-writing
  it at every call site.

  Every check is checked before the guard exits, so a broken second
  check isn't hidden behind a first one that failed first.

  # Type

  ```
  requireSuccessWith :: AttrSet -> String
  ```

  # Arguments

  attrs
  : Attribute set with the following keys:

    `checks`
    : A list of result check derivations (from `mkResult`,
      `mkResultWith`, or `mkSkip`), each contributing its `exitCode`
      output to the guard.

    `message`
    : Message printed to stderr for a failing check.
      Either a literal string (used for every failure)
      or a function from the failing check to its message
      (for a list where each member needs its own).

    `command`
    : Shell command to run once every check has succeeded.

  # Example

  ```nix
  mkResult "round-trip" <| requireSuccessWith {
    checks = [ check ];
    message = "fixture unavailable";
    command = "cat ${check}";
  }
  ```
*/
{ lib }:
{
  checks,
  message,
  command,
}:
let
  messageFor = c: if lib.isFunction message then message c else message;
in
''
  _requireSuccess_failed=""
''
+ lib.concatMapStrings (c: ''
  [ "$(cat ${c.exitCode})" = "0" ] || {
    echo ${lib.escapeShellArg (messageFor c)} >&2
    _requireSuccess_failed=1
  }
'') checks
+ ''
  [ -z "$_requireSuccess_failed" ] || exit 1
''
+ command

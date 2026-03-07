# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

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

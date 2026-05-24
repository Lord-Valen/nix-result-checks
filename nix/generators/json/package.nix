# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Generate a newline-delimited JSON report from result check outputs.

  Each line of the output is a JSON object with the following fields:

  - `name`: the attribute name of the check
  - `suite`: suite name, or `null` for flat checks
  - `kind`: the check type (`"result"`, `"snapshot"`, or `"eval"`)
  - `status`: `"pass"`, `"fail"`, or `"skip"`
  - `exitCode`: the raw exit code string
  - `stdout`: captured stdout
  - `stderr`: captured stderr
  - `drvPath`: path to the check derivation in the Nix store

  `kind` reflects `passthru.kind` on the result check derivation.
  `status` is `"skip"` when `exitCode` is empty (set by `mkSkip`).

  # Type

  ```
  json :: AttrSet -> Derivation
  ```

  # Arguments

  checks
  : Attribute set of `{ check, suite }` pairs, keyed by entry key.

  # Example

  ```nix
  pkgs.resultChecks.json.override { inherit checks; }
  ```
*/
{
  checks ? { },
  jq,
  lib,
  runCommand,
}:
runCommand "check-report.json"
  {
    nativeBuildInputs = [ jq ];
  }
  ''
    touch $out
    ${lib.concatStringsSep "\n" (
      lib.mapAttrsToList (
        name:
        { check, suite }:
        let
          displayName = if suite != null then lib.removePrefix "${suite}:" name else name;
        in
        ''
          exitCode=$(cat ${check.exitCode})
          if [ -z "$exitCode" ]; then
            status="skip"
          elif [ "$exitCode" = "0" ]; then
            status="pass"
          else
            status="fail"
          fi

          jq -n \
            --arg type "${check.passthru.kind or "result"}" \
            --arg status "$status" \
            --arg name "${displayName}" \
            --argjson suite '${builtins.toJSON suite}' \
            --arg exitCode "$exitCode" \
            --rawfile stdout ${check.stdout} \
            --rawfile stderr ${check.stderr} \
            --arg drvPath "${check}" \
            '{kind: $type, status: $status, name: $name, suite: $suite, exitCode: $exitCode, stdout: $stdout, stderr: $stderr, drvPath: $drvPath}' >> $out
        ''
      ) checks
    )}
  ''

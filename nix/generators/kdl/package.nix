# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Generate a KDL report from result check outputs.

  Renders a KDL document via a Mustache template. Each check becomes a
  node with the following properties:

  - `name`: the attribute name of the check
  - `kind`: the check type (`"result"`, `"snapshot"`, or `"eval"`)
  - `status`: `"pass"`, `"fail"`, or `"skip"`
  - `exitCode`: the raw exit code string
  - `stdout`: captured stdout (lines indented with four spaces)
  - `stderr`: captured stderr (lines indented with four spaces)
  - `drvPath`: path to the check derivation in the Nix store

  `kind` reflects `passthru.type` on the result check derivation.
  `status` is `"skip"` when `exitCode` is empty (set by `mkSkip`).

  # Type

  ```
  kdl :: AttrSet -> Derivation
  ```

  # Arguments

  checks
  : Attribute set of result check derivations.

  # Example

  ```nix
  pkgs.resultChecks.kdl.override { inherit checks; }
  ```
*/
{
  checks ? { },
  jq,
  lib,
  mustache-go,
  runCommand,
}:
let
  template = ./template.mustache;
in
runCommand "check-report.kdl"
  {
    nativeBuildInputs = [
      jq
      mustache-go
    ];
  }
  ''
    json='{"checks":[]}'
    ${lib.concatStringsSep "\n" (
      lib.mapAttrsToList (name: check: ''
        exitCode=$(cat ${check.exitCode})
        if [ -z "$exitCode" ]; then
          status="skip"
        elif [ "$exitCode" = "0" ]; then
          status="pass"
        else
          status="fail"
        fi

        json=$(echo "$json" | jq \
          --arg kind "${check.passthru.type or "result"}" \
          --arg status "$status" \
          --arg name "${name}" \
          --arg exitCode "$exitCode" \
          --rawfile stdout ${check.stdout} \
          --rawfile stderr ${check.stderr} \
          --arg drvPath "${check}" \
          '.checks += [{kind: $kind, status: $status, name: $name, exitCode: $exitCode, stdout: ($stdout | rtrimstr("\n") | split("\n") | map("    " + .) | join("\n")), stderr: ($stderr | rtrimstr("\n") | split("\n") | map("    " + .) | join("\n")), drvPath: $drvPath}]')
      '') checks
    )}

    echo "$json" | mustache ${template} > $out
  ''

# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

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

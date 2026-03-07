# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

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
      lib.mapAttrsToList (name: check: ''
        exitCode=$(cat ${check.exitCode})
        if [ -z "$exitCode" ]; then
          status="skip"
        elif [ "$exitCode" = "0" ]; then
          status="pass"
        else
          status="fail"
        fi

        jq -n \
          --arg type "${check.passthru.type or "result"}" \
          --arg status "$status" \
          --arg name "${name}" \
          --arg exitCode "$exitCode" \
          --rawfile stdout ${check.stdout} \
          --rawfile stderr ${check.stderr} \
          --arg drvPath "${check}" \
          '{kind: $type, status: $status, name: $name, exitCode: $exitCode, stdout: $stdout, stderr: $stderr, drvPath: $drvPath}' >> $out
      '') checks
    )}
  ''

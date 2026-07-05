# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

/**
  Generate a newline-delimited JSON report from result check outputs.

  Prefer `mkReport`,
  which accepts checks in their natural shape;
  this generator takes the normalized form it produces.

  Each line of the output is a JSON object with the following fields:

  - `name`: the attribute name of the check
  - `suite`: suite name, or `null` for flat checks
  - `kind`: the check type (`"result"`, `"snapshot"`, or `"eval"`)
  - `status`: `"pass"`, `"fail"`, or `"skip"`
  - `exitCode`: the raw exit code string
  - `stdout`: captured stdout
  - `stderr`: captured stderr
  - `drvPath`: path to the check derivation in the Nix store
  - `children`: nested entries of the same shape, from `passthru.children`
    on the check derivation (a list of `{ name; check; }`) — empty unless
    the check exposes any (snapshot checks expose their wrapped check as
    `"actual"`)

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
let
  # Emits shell code that writes a single check's report entry as JSON to
  # its stdout. Any check can nest further entries via `passthru.children`
  # (a list of `{ name; check; }`), embedded inline as `children` through
  # command substitution — each nesting runs in its own subshell, so a
  # parent and its children never share the `exitCode`/`status` variable
  # names below.
  mkEntry =
    {
      check,
      name,
      suite,
    }:
    let
      children = check.passthru.children or [ ];
      childrenJson =
        "["
        + lib.concatMapStringsSep "," (
          child:
          "$("
          + mkEntry {
            inherit (child) name check;
            suite = null;
          }
          + ")"
        ) children
        + "]";
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
        --arg name "${name}" \
        --argjson suite '${builtins.toJSON suite}' \
        --arg exitCode "$exitCode" \
        --rawfile stdout ${check.stdout} \
        --rawfile stderr ${check.stderr} \
        --arg drvPath "${check}" \
        --argjson children "${childrenJson}" \
        '{kind: $type, status: $status, name: $name, suite: $suite, exitCode: $exitCode, stdout: $stdout, stderr: $stderr, drvPath: $drvPath, children: $children}'
    '';
in
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
          {
          ${mkEntry {
            inherit check suite;
            name = displayName;
          }}
          } >> $out
        ''
      ) checks
    )}
  ''

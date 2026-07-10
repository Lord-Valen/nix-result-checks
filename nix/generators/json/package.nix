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
  `status` is `"skip"` when `passthru.skip` is set (by `mkSkip`); a
  skipped check's command output is never referenced, so it never
  builds, and `exitCode`/`stdout`/`stderr` are all empty strings.
  `drvPath` is still the real `.drv` path: computing it only requires
  instantiation, not a build.

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
  #
  # A skipped check's command output is never referenced: interpolating
  # a derivation output forces it to build, and mkSkip's build (four
  # touches) is real but pointless work the report doesn't need to pay
  # for just to learn what it already knows at eval time. drvPath is
  # the exception: computing a derivation's .drv only requires
  # instantiation, not a build, so it stays real. Nix normally treats
  # any reference to .drvPath as a build dependency on that derivation's
  # outputs too (a historical default for meta-programming use cases);
  # unsafeDiscardOutputDependency strips that so only instantiation is
  # required. This is the same trick nix-eval-jobs itself relies on to
  # report a drvPath for every job without building any of them.
  #
  # TODO: revisit whether a future major version should drop drvPath from
  # skip entries entirely (it points at mkSkip's four-touch build, which
  # has nothing worth inspecting) rather than paying even the
  # instantiation cost.
  mkSkippedEntry =
    {
      check,
      name,
      suite,
    }:
    ''
      jq -n \
        --arg type "${check.passthru.kind or "result"}" \
        --arg status "skip" \
        --arg name "${name}" \
        --argjson suite '${lib.toJSON suite}' \
        --arg exitCode "" \
        --arg stdout "" \
        --arg stderr "" \
        --arg drvPath "${lib.unsafeDiscardOutputDependency check.drvPath}" \
        --argjson children "[]" \
        '{kind: $type, status: $status, name: $name, suite: $suite, exitCode: $exitCode, stdout: $stdout, stderr: $stderr, drvPath: $drvPath, children: $children}'
    '';

  mkBuiltEntry =
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
        --argjson suite '${lib.toJSON suite}' \
        --arg exitCode "$exitCode" \
        --rawfile stdout ${check.stdout} \
        --rawfile stderr ${check.stderr} \
        --arg drvPath "${check}" \
        --argjson children "${childrenJson}" \
        '{kind: $type, status: $status, name: $name, suite: $suite, exitCode: $exitCode, stdout: $stdout, stderr: $stderr, drvPath: $drvPath, children: $children}'
    '';

  mkEntry =
    args: if args.check.passthru.skip or false then mkSkippedEntry args else mkBuiltEntry args;
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

# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      cfg = config.resultChecks;

      # Normalize cfg.checks (after apply) to { key -> { check, suite } }.
      # Flat checks: key = name, suite = null.
      # Suite checks: key = "suite:name", suite = suite name.
      _flat = lib.concatMapAttrs (
        outerName: value:
        if value ? drvPath then
          {
            "${outerName}" = {
              check = value;
              suite = null;
            };
          }
        else
          lib.mapAttrs' (
            checkName: check:
            lib.nameValuePair "${outerName}:${checkName}" {
              inherit check;
              suite = outerName;
            }
          ) value
      ) cfg.checks;
    in
    {
      options.resultChecks = {
        enable = lib.mkOption {
          description = "Enable Result monad checks.";
          type = lib.types.bool;
          default = true;
        };

        checks = lib.mkOption {
          type = lib.types.attrsOf (lib.types.either lib.types.package (lib.types.attrsOf lib.types.package));
          default = { };
          description = ''
            Checks to run.

            Values are either a derivation (flat check) or an attrset of
            derivations (suite). Suite checks are grouped under a named
            header in the TUI and keyed as `"suite:name"` in reports.
          '';
          apply =
            x:
            lib.mapAttrs (
              outerName: value:
              # Discriminate flat checks from suites via drvPath presence.
              # lib.isDerivation checks value.type == "derivation", which is
              # unreliable — user attrsets could shadow type. drvPath is safer.
              if value ? drvPath then
                if lib.elem outerName cfg.skipChecks then pkgs.resultChecks.mkSkip value else value
              else
                lib.mapAttrs (
                  checkName: drv:
                  let
                    key = "${outerName}:${checkName}";
                  in
                  if lib.elem key cfg.skipChecks then pkgs.resultChecks.mkSkip drv else drv
                ) value
            ) x;
        };

        skipChecks = lib.mkOption {
          type = with lib.types; listOf str;
          default = [ ];
          description = ''
            Check keys to skip.

            Flat checks are identified by name (e.g. `"lint"`).
            Suite checks are identified as `"suite:name"` (e.g. `"db:schema"`).

            Skipped checks are replaced with placeholder derivations and
            marked as skipped in the report.
          '';
        };

        enableFlakeChecks = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = ''
            Whether to automatically add Result checks to flake.checks.

            When enabled, each check defined in cfg.checks will have a
            corresponding wrapper added to flake.checks. The wrapper fails
            if the exit code indicates failure.
          '';
        };

        reportGenerator = lib.mkOption {
          type = with lib.types; functionTo package;
          default = checks: pkgs.resultChecks.json.override { inherit checks; };
          defaultText = lib.literalExpression "checks: pkgs.resultChecks.json.override { inherit checks; }";
          description = ''
            Function that generates the report package from the normalized
            check set. Receives `{ key -> { check, suite } }` pairs.
          '';
        };

        report = lib.mkOption {
          type = lib.types.package;
          default = cfg.reportGenerator _flat;
          defaultText = lib.literalExpression "cfg.reportGenerator _flat";
          readOnly = true;
          description = "The generated report package.";
        };
      };

      config = lib.mkIf cfg.enable {
        checks = lib.mkIf cfg.enableFlakeChecks (
          lib.mapAttrs' (
            key:
            { check, suite }:
            let
              flakeKey = builtins.replaceStrings [ ":" ] [ "-" ] key;
            in
            lib.nameValuePair flakeKey (
              pkgs.runCommand "check-${flakeKey}" { } ''
                exitCode=$(cat ${check.exitCode})

                echo "Check '${key}' exit code: $exitCode"
                echo ""
                echo "stdout:"
                cat ${check.stdout}
                echo "stderr:"
                cat ${check.stderr}

                if [ "$exitCode" -ne 0 ]; then
                  exit "$exitCode"
                fi

                install -D ${cfg.reportGenerator { "${key}" = { inherit check suite; }; }} $out
              ''
            )
          ) (lib.filterAttrs (_key: { check, ... }: !(check.passthru.skip or false)) _flat)
        );
      };
    };
}

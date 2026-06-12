# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  imports = [ ./flakeOutput.nix ];

  perSystem =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      cfg = config.resultChecks;

      isEval = value: value.kind or null == "eval";

      drvChecks = lib.filterAttrs (_name: value: !isEval value) cfg.checks;
      evalCheckSet = lib.filterAttrs (_name: value: isEval value) cfg.checks;

      # Normalize derivation checks (after apply) to
      # { key -> { check, suite } } for the report generator.
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
      ) drvChecks;

      evalCheckType = lib.types.addCheck lib.types.attrs isEval // {
        name = "evalCheck";
        description = "eval check (mkEval)";
      };

      applySkips =
        outerName: value:
        # Discriminate via drvPath presence rather than lib.isDerivation:
        # user attrsets could shadow type. Eval checks are tagged.
        if value ? drvPath then
          if lib.elem outerName cfg.skipChecks then pkgs.resultChecks.mkSkip value else value
        else if isEval value then
          let
            prefix = "${outerName}:";
            skipTests = map (lib.removePrefix prefix) (lib.filter (lib.hasPrefix prefix) cfg.skipChecks);
            skipped = if lib.elem outerName cfg.skipChecks then pkgs.resultChecks.mkSkip value else value;
          in
          skipped // { inherit skipTests; }
        else
          lib.mapAttrs (
            checkName: drv:
            let
              key = "${outerName}:${checkName}";
            in
            if lib.elem key cfg.skipChecks then pkgs.resultChecks.mkSkip drv else drv
          ) value;

      evalLine =
        key: entry:
        {
          pass = "PASS: ${key}\n";
          skip = "SKIP: ${key}\n";
          fail = entry.stdout;
        }
        .${entry.status};
      evalReport = lib.concatStrings (
        lib.concatLists (
          lib.mapAttrsToList (
            checkName: lib.mapAttrsToList (testName: evalLine "${checkName}:${testName}")
          ) cfg.evalChecks
        )
      );
      evalFailed = lib.any (entry: entry.status == "fail") (
        lib.concatMap lib.attrValues (lib.attrValues cfg.evalChecks)
      );
    in
    {
      options.resultChecks = {
        enable = lib.mkOption {
          description = "Enable Result monad checks.";
          type = lib.types.bool;
          default = true;
        };

        checks = lib.mkOption {
          type = lib.types.attrsOf (
            lib.types.oneOf [
              lib.types.package
              evalCheckType
              (lib.types.attrsOf lib.types.package)
            ]
          );
          default = { };
          description = ''
            Checks to run.

            Values are a derivation (flat check), an attrset of
            derivations (suite), or an eval check (`mkEval`). Suite
            and eval checks are grouped under a named header in the
            TUI and keyed as `"suite:name"` in reports.
          '';
          apply = lib.mapAttrs applySkips;
        };

        skipChecks = lib.mkOption {
          type = with lib.types; listOf str;
          default = [ ];
          description = ''
            Check keys to skip.

            Flat checks are identified by name (e.g. `"lint"`).
            Suite checks and eval tests are identified as
            `"suite:name"` (e.g. `"db:schema"`).

            Skipped derivation checks are replaced with placeholder
            derivations; skipped eval tests are never evaluated. Both
            are marked as skipped in the report.
          '';
        };

        enableFlakeChecks = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = ''
            Whether to add an aggregate Result check to flake.checks.

            The single `resultChecks` flake check depends on every
            derivation check (built in parallel by the scheduler) and
            bakes in eval check verdicts. Its log carries the full
            per-check report; it fails if any check failed.
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
          description = ''
            The generated report package.

            Covers derivation checks only; eval check results are
            exposed through `evalChecks` so that runners can evaluate
            them in parallel without the report forcing them.
          '';
        };

        reportChecks = lib.mkOption {
          type = with lib.types; listOf str;
          default = lib.attrNames _flat;
          defaultText = lib.literalExpression "lib.attrNames _flat";
          readOnly = true;
          description = "Keys of the checks covered by the report.";
        };

        evalChecks = lib.mkOption {
          type = with lib.types; lazyAttrsOf (lazyAttrsOf raw);
          default = lib.mapAttrs (_name: pkgs.resultChecks.mkEntries) evalCheckSet;
          defaultText = lib.literalExpression "lib.mapAttrs (_name: pkgs.resultChecks.mkEntries) evalCheckSet";
          readOnly = true;
          description = ''
            Per-test result entries of all eval checks, keyed by check
            then test name. Entries are computed lazily so runners can
            force them in parallel (e.g. via nix-eval-jobs).
          '';
        };
      };

      config = lib.mkIf cfg.enable {
        checks = lib.mkIf cfg.enableFlakeChecks {
          resultChecks =
            pkgs.runCommand "result-checks"
              {
                nativeBuildInputs = [ pkgs.jq ];
              }
              ''
                jq -r '"\(.status | ascii_upcase): \(if .suite != null then .suite + ":" else "" end + .name)"' \
                  ${cfg.report}
                jq -r 'select(.status == "fail") | .stdout + .stderr' ${cfg.report}
                printf '%s' ${lib.escapeShellArg evalReport}

                failed=$(jq -s 'map(select(.status == "fail")) | length' ${cfg.report})
                ${lib.optionalString evalFailed "failed=$((failed + 1))"}
                [ "$failed" -eq 0 ] || exit 1
                touch $out
              '';
        };
      };
    };
}

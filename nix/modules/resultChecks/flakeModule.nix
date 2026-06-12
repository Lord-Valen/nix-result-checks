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

        report = lib.mkOption {
          type = lib.types.package;
          default = pkgs.resultChecks.mkReport cfg.checks;
          defaultText = lib.literalExpression "pkgs.resultChecks.mkReport cfg.checks";
          readOnly = true;
          description = ''
            The generated report package.

            Covers derivation checks only; eval check results are
            exposed through `evalChecks` so that runners can evaluate
            them in parallel without the report forcing them.
          '';
        };

        evalChecks = lib.mkOption {
          type = with lib.types; lazyAttrsOf (lazyAttrsOf raw);
          default = pkgs.resultChecks.mkEvalChecks cfg.checks;
          defaultText = lib.literalExpression "pkgs.resultChecks.mkEvalChecks cfg.checks";
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

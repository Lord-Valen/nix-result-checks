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
    in
    {
      options.resultChecks = {
        enable = lib.mkOption {
          description = "Enable Result monad checks.";
          type = lib.types.bool;
          default = true;
        };

        checks = lib.mkOption {
          type = lib.types.attrsOf lib.types.package;
          default = { };
          description = "Checks that produce Result monad outputs (out, stdout, stderr, exitCode).";
          apply =
            x:
            lib.mapAttrs (
              name: check: if lib.elem name cfg.skipChecks then pkgs.resultChecks.mkSkip check else check
            ) x;
        };

        skipChecks = lib.mkOption {
          type = with lib.types; listOf str;
          default = [ ];
          description = ''
            List of check names to skip.

            This option allows you to specify checks that should be skipped.
            Checks listed here will be replaced with placeholder derivations.
            These checks will not be included in the flake.checks wrappers and will be marked as skipped in the report.
            This is useful for temporarily disabling checks without removing them from the configuration.
          '';
        };

        enableFlakeChecks = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = ''
            Whether to automatically add Result checks to flake.checks.

            When this option is enabled, each check defined in cfg.checks will have a corresponding wrapper check added to flake.checks.
            These wrapper checks will execute the original check and fail if the exit code indicates failure.
            This allows you to run `nix flake check` and have it report failures based on the Result checks you've defined.
          '';
        };

        reportGenerator = lib.mkOption {
          type = with lib.types; functionTo package;
          default = checks: pkgs.resultChecks.json.override { inherit checks; };
          defaultText = lib.literalExpression "checks: pkgs.resultChecks.json.override { inherit checks; }";
          description = ''
            Function that generates the report package from the check results.

            This function takes the set of checks as input and produces a package that generates the report output.
            The default implementation generates a JSON report using the provided checks.
          '';
        };

        report = lib.mkOption {
          type = lib.types.package;
          default = cfg.reportGenerator cfg.checks;
          defaultText = lib.literalExpression "cfg.reportGenerator cfg.checks";
          readOnly = true;
          description = ''
            The generated report package.
          '';
        };
      };

      config = lib.mkIf cfg.enable {
        # Add wrapper checks to flake.checks that fail appropriately
        checks = lib.mkIf cfg.enableFlakeChecks (
          lib.mapAttrs (
            name: resultCheck:
            pkgs.runCommand "check-${name}" { } ''
              exitCode=$(cat ${resultCheck.exitCode})

              echo "Check '${name}' exit code: $exitCode"
              echo ""
              echo "stdout:"
              cat ${resultCheck.stdout}
              echo "stderr:"
              cat ${resultCheck.stderr}

              if [ "$exitCode" -ne 0 ]; then
                exit "$exitCode"
              fi

              install -D ${cfg.reportGenerator { "${name}" = resultCheck; }} $out
            ''
          ) (lib.filterAttrs (_name: check: !(check.passthru.skip or false)) cfg.checks)
        );
      };
    };
}

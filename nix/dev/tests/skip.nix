# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    {
      lib,
      pkgs,
      ...
    }:
    let
      inherit (pkgs.resultChecks)
        mkEval
        mkReport
        mkResult
        mkSkip
        mkSnapshot
        ;
      module = (import ../../modules/resultChecks/flakeModule.nix).perSystem;
      evalCfg =
        cfg:
        (lib.evalModules {
          modules = [
            module
            {
              resultChecks = cfg // {
                enableFlakeChecks = false;
              };
            }
            { _module.check = false; }
          ];
          specialArgs = { inherit pkgs lib; };
        }).config.resultChecks.checks;
    in
    {
      resultChecks = {
        skipChecks = [ "skip:by-key" ];

        checks.skip = {
          func =
            mkSnapshot "skip-func" {
              exitCode = "";
              stdout = "";
              stderr = "";
            }
            <| mkSkip
            <| mkResult "skip-func-actual" ''
              echo "This should be skipped"
              exit 1
            '';

          args =
            mkSnapshot "skip-args" {
              exitCode = "";
              stdout = "";
              stderr = "";
            }
            <| mkSkip
            <| mkResult "skip-args-actual" ''
              echo "This should be skipped"
              exit 1
            '';

          by-key = mkResult "skip-by-key" ''
            echo "This should be skipped"
            exit 1
          '';
        };

        checks.skip-module = mkEval {
          testSkipChecksApplied = {
            expr =
              (evalCfg {
                skipChecks = [ "my-check" ];
                checks.my-check = mkResult "my-check" "exit 0";
              }).my-check.passthru.skip;
            expected = true;
          };
          testNonSkippedCheckUnaffected = {
            expr =
              (evalCfg {
                checks.my-check = mkResult "my-check" "exit 0";
              }).my-check.passthru.skip or false;
            expected = false;
          };
        };

        # $out is always touched regardless of a command's outcome, so the
        # report generator can't tell a skip apart from a real build just
        # by looking at outputs; it must consult passthru.skip and, when
        # set, never reference the check's command output at all.
        # Otherwise the trivial mkSkip build still happens on every
        # report build, for every skipped check, defeating the point of
        # skipping. drvPath stays real (only instantiation, not a build).
        checks.report-skip = mkEval {
          testSkippedCheckNeverForcesItsCommandOutput = {
            expr =
              lib.isString
                (mkReport {
                  fixture = {
                    drvPath = "/nix/store/fake-fixture.drv";
                    passthru = {
                      skip = true;
                      kind = "result";
                    };
                    exitCode = throw "never forced";
                    stdout = throw "never forced";
                    stderr = throw "never forced";
                  };
                }).drvPath;
            expected = true;
          };
        };
      };
    };
}

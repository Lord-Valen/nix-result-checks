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
      };
    };
}

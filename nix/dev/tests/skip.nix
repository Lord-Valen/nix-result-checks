# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { pkgs, lib, ... }:
    {
      resultChecks.checks =
        let
          inherit (pkgs.resultChecks)
            mkResult
            mkSkip
            mkResultSnapshot
            mkEvalTests
            ;
          module = (import ../../modules/resultChecks/flakeModule.nix).perSystem;
          evalChecks =
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
          skip-func = mkResultSnapshot "skip-func" { } {
            resultCheck =
              mkResult "skip-func-actual" { } ''
                echo "This should be skipped"
                exit 1
              ''
              |> mkSkip;

            exitCode = "";
            stdout = "";
            stderr = "";
          };

          skip-args = mkResultSnapshot "skip-args" { } {
            resultCheck = mkResult "skip-args-actual" { passthru.skip = true; } ''
              echo "This should be skipped"
              exit 1
            '';

            exitCode = "";
            stdout = "";
            stderr = "";
          };

          skip-list = mkResultSnapshot "skip-list" { } {
            resultCheck = mkEvalTests "skip-list" { } {
              testSkipChecksApplied = {
                expr =
                  (evalChecks {
                    skipChecks = [ "my-check" ];
                    checks.my-check = mkResult "my-check" { } "exit 0";
                  }).my-check.passthru.skip;
                expected = true;
              };
              testNonSkippedCheckUnaffected = {
                expr =
                  (evalChecks {
                    checks.my-check = mkResult "my-check" { } "exit 0";
                  }).my-check.passthru.skip or false;
                expected = false;
              };
            };

            exitCode = "0";
            stdout = "";
            stderr = "";
          };
        };
    };
}

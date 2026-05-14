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
    {
      resultChecks = {
        skipChecks = [ "skip-list-actual" ];
        checks =
          let
            inherit (pkgs.resultChecks)
              mkResult
              mkSkip
              mkSnapshot
              mkEval
              ;
            inherit (config.resultChecks) checks;
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
            skip-func = mkSnapshot "skip-func" {
              resultCheck =
                mkResult "skip-func-actual" ''
                  echo "This should be skipped"
                  exit 1
                ''
                |> mkSkip;

              exitCode = "";
              stdout = "";
              stderr = "";
            };

            skip-args = mkSnapshot "skip-args" {
              resultCheck =
                mkResult "skip-args-actual" ''
                  echo "This should be skipped"
                  exit 1
                ''
                |> mkSkip;

              exitCode = "";
              stdout = "";
              stderr = "";
            };

            skip-list-actual = mkResult "skip-list-actual" ''
              echo "This should be skipped"
              exit 1
            '';

            skip-list = mkSnapshot "skip-list" {
              resultCheck = mkEval "skip-list" {
                testSkipChecksApplied = {
                  expr =
                    (evalChecks {
                      skipChecks = [ "my-check" ];
                      checks.my-check = mkResult "my-check" "exit 0";
                    }).my-check.passthru.skip;
                  expected = true;
                };
                testNonSkippedCheckUnaffected = {
                  expr =
                    (evalChecks {
                      checks.my-check = mkResult "my-check" "exit 0";
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
    };
}

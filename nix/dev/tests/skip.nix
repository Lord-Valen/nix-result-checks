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
        skipChecks = [ "skip:list-actual" ];
        checks.skip =
          let
            inherit (pkgs.resultChecks)
              mkResult
              mkSkip
              mkSnapshot
              mkEval
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

            list-actual = mkResult "skip-list-actual" ''
              echo "This should be skipped"
              exit 1
            '';

            list =
              mkSnapshot "skip-list" {
                exitCode = "0";
                stdout = "";
                stderr = "";
              }
              <| mkEval "skip-list" {
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
          };
      };
    };
}

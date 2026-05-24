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
    {
      resultChecks.checks.suite =
        let
          inherit (pkgs.resultChecks)
            mkResult
            mkEval
            mkSnapshot
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

          # A minimal report built from a controlled suite check, used to verify
          # the generator emits bare names and the correct suite field.
          minimalSuiteReport = pkgs.resultChecks.json.override {
            checks."example:check" = {
              check = mkResult "check" "exit 0";
              suite = "example";
            };
          };
        in
        {
          module =
            mkSnapshot "suite-module" {
              exitCode = "0";
              stdout = "";
              stderr = "";
            }
            <| mkEval "suite-module" {
              testSuiteCheckSkippedByKey = {
                expr =
                  (evalChecks {
                    skipChecks = [ "my-suite:my-check" ];
                    checks.my-suite.my-check = mkResult "my-check" "exit 0";
                  }).my-suite.my-check.passthru.skip;
                expected = true;
              };

              testOtherSuiteMemberUnaffected = {
                expr =
                  (evalChecks {
                    skipChecks = [ "my-suite:my-check" ];
                    checks.my-suite = {
                      my-check = mkResult "my-check" "exit 0";
                      other = mkResult "other" "exit 0";
                    };
                  }).my-suite.other.passthru.skip or false;
                expected = false;
              };
            };

          generator =
            mkSnapshot "suite-generator" {
              exitCode = "0";
              stdout = ''
                name=check
                suite=example
              '';
              stderr = "";
            }
            <| mkResult "suite-generator-actual" ''
              name=$(${pkgs.jq}/bin/jq -r '.name' ${minimalSuiteReport})
              suite=$(${pkgs.jq}/bin/jq -r '.suite' ${minimalSuiteReport})
              echo "name=$name"
              echo "suite=$suite"
              [ "$name" = "check" ] || { echo "Expected name=check, got: $name" >&2; exit 1; }
              [ "$suite" = "example" ] || { echo "Expected suite=example, got: $suite" >&2; exit 1; }
            '';
        };
    };
}

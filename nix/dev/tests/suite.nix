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

      # A minimal report built from a controlled suite check, used to verify
      # normalization emits bare names and the correct suite field.
      minimalSuiteReport = pkgs.resultChecks.mkReport {
        example.check = mkResult "check" "exit 0";
      };
    in
    {
      resultChecks.checks = {
        suite.generator =
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

        suite-module = mkEval {
          testSuiteCheckSkippedByKey = {
            expr =
              (evalCfg {
                skipChecks = [ "my-suite:my-check" ];
                checks.my-suite.my-check = mkResult "my-check" "exit 0";
              }).my-suite.my-check.passthru.skip;
            expected = true;
          };

          testOtherSuiteMemberUnaffected = {
            expr =
              (evalCfg {
                skipChecks = [ "my-suite:my-check" ];
                checks.my-suite = {
                  my-check = mkResult "my-check" "exit 0";
                  other = mkResult "other" "exit 0";
                };
              }).my-suite.other.passthru.skip or false;
            expected = false;
          };
        };
      };
    };
}

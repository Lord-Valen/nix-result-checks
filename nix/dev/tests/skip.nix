# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { config, pkgs, ... }:
    {
      resultChecks = {
        skipChecks = [ "skip-list-actual" ];
        checks =
          let
            inherit (pkgs.resultChecks) mkResult mkSkip mkResultSnapshot;
            inherit (config.resultChecks) checks;
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

            skip-list-actual = mkResult "skip-list-actual" { } ''
              echo "This should be skipped"
              exit 1
            '';

            skip-list = mkResultSnapshot "skip-list" { } {
              resultCheck = checks.skip-list-actual;

              exitCode = "";
              stdout = "";
              stderr = "";
            };
          };
      };
    };
}

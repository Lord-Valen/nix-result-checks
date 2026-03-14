# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { config, pkgs, ... }:
    {
      resultChecks.checks =
        let
          inherit (pkgs.resultChecks) mkResult mkResultSnapshot;
          inherit (config.resultChecks) checks;
        in
        {
          test-passing-actual = mkResult "test-passing-actual" { } ''
            echo "Starting test suite..." >&2
            echo "Running unit tests..." >&2
            echo "Test 1: PASS"
            echo "Test 2: PASS"
            echo "All tests completed successfully" >&2
            exit 0
          '';

          test-passing = mkResultSnapshot "test-passing" { } {
            resultCheck = checks.test-passing-actual;
            exitCode = "0";
            stdout = ''
              Test 1: PASS
              Test 2: PASS
            '';
            stderr = ''
              Starting test suite...
              Running unit tests...
              All tests completed successfully
            '';
          };

          test-failing = mkResultSnapshot "test-failing" { } {
            resultCheck = mkResult "test-failing-actual" { } ''
              echo "Starting validation checks..." >&2
              echo "Checking configuration..." >&2
              echo "Warning: Deprecated option detected" >&2
              echo "ERROR: Validation failed - missing required field" >&2
              echo "Failed at line 42" >&2
              exit 1
            '';
            exitCode = "1";
            stderr = ''
              Starting validation checks...
              Checking configuration...
              Warning: Deprecated option detected
              ERROR: Validation failed - missing required field
              Failed at line 42
            '';
          };
        };
    };
}

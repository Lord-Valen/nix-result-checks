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
              mkResultSnapshot
              mkEvalTests
              ;
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

            snapshot-exit-code-only = mkResultSnapshot "snapshot-exit-code-only" { } {
              resultCheck = mkResult "snapshot-exit-code-only-actual" { } ''
                echo "some output"
                exit 42
              '';
              exitCode = "42";
            };

            snapshot-stdout-only = mkResultSnapshot "snapshot-stdout-only" { } {
              resultCheck = mkResult "snapshot-stdout-only-actual" { } ''
                echo "expected stdout line"
                exit 1
              '';
              stdout = ''
                expected stdout line
              '';
            };

            snapshot-exit-code-mismatch = mkResultSnapshot "snapshot-exit-code-mismatch" { } {
              resultCheck = mkResultSnapshot "snapshot-exit-code-mismatch-actual" { } {
                resultCheck = mkResult "snapshot-exit-code-mismatch-inner" { } ''
                  echo "actual output"
                  exit 0
                '';
                exitCode = "1";
              };
              exitCode = "1";
              stderr = ''
                Exit code mismatch: expected 1, got 0
              '';
            };

            snapshot-stdout-mismatch = mkResultSnapshot "snapshot-stdout-mismatch" { } {
              resultCheck = mkResultSnapshot "snapshot-stdout-mismatch-actual" { } {
                resultCheck = mkResult "snapshot-stdout-mismatch-inner" { } ''
                  echo "actual output"
                  exit 0
                '';
                stdout = ''
                  wrong output
                '';
              };
              exitCode = "1";
              stderr = ''
                Stdout mismatch
                Expected:
                wrong output
                Got:
                actual output
              '';
            };

            snapshot-stderr-only = mkResultSnapshot "snapshot-stderr-only" { } {
              resultCheck = mkResult "snapshot-stderr-only-actual" { } ''
                echo "expected stderr line" >&2
                exit 1
              '';
              stderr = ''
                expected stderr line
              '';
            };

            snapshot-stderr-mismatch = mkResultSnapshot "snapshot-stderr-mismatch" { } {
              resultCheck = mkResultSnapshot "snapshot-stderr-mismatch-actual" { } {
                resultCheck = mkResult "snapshot-stderr-mismatch-inner" { } ''
                  echo "actual output" >&2
                  exit 0
                '';
                stderr = ''
                  wrong output
                '';
              };
              exitCode = "1";
              stderr = ''
                Stderr mismatch
                Expected:
                wrong output
                Got:
                actual output
              '';
            };

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

            eval-passing = mkResultSnapshot "eval-passing" { } {
              resultCheck = mkEvalTests "eval-passing" { } {
                testAddition = {
                  expr = 1 + 1;
                  expected = 2;
                };
                testConcat = {
                  expr = "hello" + " " + "world";
                  expected = "hello world";
                };
              };
              exitCode = "0";
              stdout = "";
              stderr = "";
            };

            eval-failing = mkResultSnapshot "eval-failing" { } {
              resultCheck = mkEvalTests "eval-failing" { } {
                testWrong = {
                  expr = 1 + 1;
                  expected = 3;
                };
              };
              exitCode = "1";
              stdout = ''
                FAIL: testWrong
                  expected: 3
                  got:      2
              '';
              stderr = ''
                1 test(s) failed
              '';
            };

            eval-mixed = mkResultSnapshot "eval-mixed" { } {
              resultCheck = mkEvalTests "eval-mixed" { } {
                testPass = {
                  expr = true;
                  expected = true;
                };
                testFail = {
                  expr = "foo";
                  expected = "bar";
                };
              };
              exitCode = "1";
              stdout = ''
                FAIL: testFail
                  expected: "bar"
                  got:      "foo"
              '';
              stderr = ''
                1 test(s) failed
              '';
            };

            eval-skip = mkResultSnapshot "eval-skip" { } {
              resultCheck = mkEvalTests "eval-skip" { passthru.skip = true; } {
                testSkipped = {
                  expr = 1;
                  expected = 2;
                };
              };
              exitCode = "";
              stdout = "";
              stderr = "";
            };
          };
      };
    };
}

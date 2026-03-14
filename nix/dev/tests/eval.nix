# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { pkgs, ... }:
    {
      resultChecks.checks =
        let
          inherit (pkgs.resultChecks) mkEvalTests mkResultSnapshot;
        in
        {
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
}

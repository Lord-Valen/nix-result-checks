# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { pkgs, ... }:
    {
      resultChecks.checks =
        let
          inherit (pkgs.resultChecks) mkEval mkSnapshot mkSkip;
        in
        {
          eval-passing = mkSnapshot "eval-passing" {
            resultCheck = mkEval "eval-passing" {
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

          eval-failing = mkSnapshot "eval-failing" {
            resultCheck = mkEval "eval-failing" {
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

          eval-mixed = mkSnapshot "eval-mixed" {
            resultCheck = mkEval "eval-mixed" {
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

          eval-skip = mkSnapshot "eval-skip" {
            resultCheck =
              mkEval "eval-skip" {
                testSkipped = {
                  expr = 1;
                  expected = 2;
                };
              }
              |> mkSkip;
            exitCode = "";
            stdout = "";
            stderr = "";
          };
        };
    };
}

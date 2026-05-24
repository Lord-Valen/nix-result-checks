# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { pkgs, ... }:
    {
      resultChecks.checks.eval =
        let
          inherit (pkgs.resultChecks) mkEval mkSnapshot mkSkip;
        in
        {
          passing =
            mkSnapshot "eval-passing" {
              exitCode = "0";
              stdout = "";
              stderr = "";
            }
            <| mkEval "eval-passing" {
              testAddition = {
                expr = 1 + 1;
                expected = 2;
              };
              testConcat = {
                expr = "hello" + " " + "world";
                expected = "hello world";
              };
            };

          failing =
            mkSnapshot "eval-failing" {
              exitCode = "1";
              stdout = ''
                FAIL: testWrong
                  expected: 3
                  got:      2
              '';
              stderr = ''
                1 test(s) failed
              '';
            }
            <| mkEval "eval-failing" {
              testWrong = {
                expr = 1 + 1;
                expected = 3;
              };
            };

          mixed =
            mkSnapshot "eval-mixed" {
              exitCode = "1";
              stdout = ''
                FAIL: testFail
                  expected: "bar"
                  got:      "foo"
              '';
              stderr = ''
                1 test(s) failed
              '';
            }
            <| mkEval "eval-mixed" {
              testPass = {
                expr = true;
                expected = true;
              };
              testFail = {
                expr = "foo";
                expected = "bar";
              };
            };

          skip =
            mkSnapshot "eval-skip" {
              exitCode = "";
              stdout = "";
              stderr = "";
            }
            <| mkSkip
            <| mkEval "eval-skip" {
              testSkipped = {
                expr = 1;
                expected = 2;
              };
            };
        };
    };
}

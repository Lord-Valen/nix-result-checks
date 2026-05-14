# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { pkgs, ... }:
    {
      resultChecks.checks =
        let
          inherit (pkgs.resultChecks) mkResult mkSnapshot;
        in
        {
          snapshot-exit-code-only = mkSnapshot "snapshot-exit-code-only" {
            resultCheck = mkResult "snapshot-exit-code-only-actual" ''
              echo "some output"
              exit 42
            '';
            exitCode = "42";
          };

          snapshot-stdout-only = mkSnapshot "snapshot-stdout-only" {
            resultCheck = mkResult "snapshot-stdout-only-actual" ''
              echo "expected stdout line"
              exit 1
            '';
            stdout = ''
              expected stdout line
            '';
          };

          snapshot-stderr-only = mkSnapshot "snapshot-stderr-only" {
            resultCheck = mkResult "snapshot-stderr-only-actual" ''
              echo "expected stderr line" >&2
              exit 1
            '';
            stderr = ''
              expected stderr line
            '';
          };

          snapshot-exit-code-mismatch = mkSnapshot "snapshot-exit-code-mismatch" {
            resultCheck = mkSnapshot "snapshot-exit-code-mismatch-actual" {
              resultCheck = mkResult "snapshot-exit-code-mismatch-inner" ''
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

          snapshot-stdout-mismatch = mkSnapshot "snapshot-stdout-mismatch" {
            resultCheck = mkSnapshot "snapshot-stdout-mismatch-actual" {
              resultCheck = mkResult "snapshot-stdout-mismatch-inner" ''
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

          snapshot-stderr-mismatch = mkSnapshot "snapshot-stderr-mismatch" {
            resultCheck = mkSnapshot "snapshot-stderr-mismatch-actual" {
              resultCheck = mkResult "snapshot-stderr-mismatch-inner" ''
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
        };
    };
}

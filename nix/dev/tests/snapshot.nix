# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { pkgs, ... }:
    {
      resultChecks.checks =
        let
          inherit (pkgs.resultChecks) mkResult mkResultSnapshot;
        in
        {
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

          snapshot-stderr-only = mkResultSnapshot "snapshot-stderr-only" { } {
            resultCheck = mkResult "snapshot-stderr-only-actual" { } ''
              echo "expected stderr line" >&2
              exit 1
            '';
            stderr = ''
              expected stderr line
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
        };
    };
}

# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { pkgs, ... }:
    {
      resultChecks.checks.snapshot =
        let
          inherit (pkgs.resultChecks) mkResult mkSnapshot;
        in
        {
          exit-code-only =
            mkSnapshot "snapshot-exit-code-only" { exitCode = "42"; }
            <| mkResult "snapshot-exit-code-only-actual" ''
              echo "some output"
              exit 42
            '';

          stdout-only =
            mkSnapshot "snapshot-stdout-only" {
              stdout = ''
                expected stdout line
              '';
            }
            <| mkResult "snapshot-stdout-only-actual" ''
              echo "expected stdout line"
              exit 1
            '';

          stderr-only =
            mkSnapshot "snapshot-stderr-only" {
              stderr = ''
                expected stderr line
              '';
            }
            <| mkResult "snapshot-stderr-only-actual" ''
              echo "expected stderr line" >&2
              exit 1
            '';

          exit-code-mismatch =
            mkSnapshot "snapshot-exit-code-mismatch" {
              exitCode = "1";
              stderr = ''
                Exit code mismatch: expected 1, got 0
              '';
            }
            <| mkSnapshot "snapshot-exit-code-mismatch-actual" { exitCode = "1"; }
            <| mkResult "snapshot-exit-code-mismatch-inner" ''
              echo "actual output"
              exit 0
            '';

          stdout-mismatch =
            mkSnapshot "snapshot-stdout-mismatch" {
              exitCode = "1";
              stderr = ''
                Stdout mismatch
                Expected:
                wrong output
                Got:
                actual output
              '';
            }
            <| mkSnapshot "snapshot-stdout-mismatch-actual" {
              stdout = ''
                wrong output
              '';
            }
            <| mkResult "snapshot-stdout-mismatch-inner" ''
              echo "actual output"
              exit 0
            '';

          stderr-mismatch =
            mkSnapshot "snapshot-stderr-mismatch" {
              exitCode = "1";
              stderr = ''
                Stderr mismatch
                Expected:
                wrong output
                Got:
                actual output
              '';
            }
            <| mkSnapshot "snapshot-stderr-mismatch-actual" {
              stderr = ''
                wrong output
              '';
            }
            <| mkResult "snapshot-stderr-mismatch-inner" ''
              echo "actual output" >&2
              exit 0
            '';
        };
    };
}

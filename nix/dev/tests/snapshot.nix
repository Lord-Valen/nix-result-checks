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

          # A report built from a single snapshot check, used to verify the
          # generator nests the snapshot's underlying result check as a
          # `children` entry rather than only exposing the mismatch text.
          minimalSnapshotReport = pkgs.resultChecks.mkReport {
            example =
              mkSnapshot "example" { stdout = "hello\n"; }
              <| mkResult "example-actual" ''
                echo hello
              '';
          };
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

          children-generator =
            mkSnapshot "snapshot-children-generator" {
              exitCode = "0";
              stdout = ''
                childName=example-actual
                childStdout=hello
              '';
              stderr = "";
            }
            <| mkResult "snapshot-children-generator-actual" ''
              childName=$(${pkgs.jq}/bin/jq -r '.children[0].name' ${minimalSnapshotReport})
              childStdout=$(${pkgs.jq}/bin/jq -r '.children[0].stdout' ${minimalSnapshotReport})
              echo "childName=$childName"
              echo "childStdout=$childStdout"
              [ "$childName" = "example-actual" ] || { echo "Expected childName=example-actual, got: $childName" >&2; exit 1; }
              [ "$childStdout" = "hello" ] || { echo "Expected childStdout=hello, got: $childStdout" >&2; exit 1; }
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

# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    {
      pkgs,
      ...
    }:
    {
      # Artifact semantics: a command may write real content to $out;
      # the capture wrapper's `touch` must preserve it. Unwritten and
      # skipped checks leave an empty sentinel, so consumers guard on
      # the producer's exit code.
      resultChecks.checks.artifact =
        let
          inherit (pkgs.resultChecks) mkResult mkSkip mkSnapshot;
          producer = mkResult "artifact-producer" ''echo data > "$out"'';
          empty-producer = mkResult "artifact-empty-producer" "true";
          skipped-producer = mkSkip (mkResult "artifact-skipped-producer" ''echo data > "$out"'');
        in
        {
          round-trip =
            mkSnapshot "artifact-round-trip" {
              exitCode = "0";
              stdout = ''
                data
              '';
            }
            <| mkResult "artifact-round-trip-actual" "cat ${producer}";

          unwritten-is-empty =
            mkSnapshot "artifact-unwritten-is-empty" {
              exitCode = "0";
              stdout = "";
            }
            <| mkResult "artifact-unwritten-is-empty-actual" "cat ${empty-producer}";

          skipped-fails-the-guard =
            mkSnapshot "artifact-skipped-fails-the-guard" {
              exitCode = "1";
              stdout = "";
              stderr = ''
                fixture unavailable
              '';
            }
            <| mkResult "artifact-skipped-fails-the-guard-actual" ''
              [ "$(cat ${skipped-producer.exitCode})" = "0" ] || {
                echo "fixture unavailable" >&2
                exit 1
              }
              cat ${skipped-producer}
            '';
        };

      resultChecks.checks.result =
        let
          inherit (pkgs.resultChecks) mkResult mkSnapshot;
        in
        {
          passing =
            mkSnapshot "test-passing" {
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
            }
            <| mkResult "test-passing-actual" ''
              echo "Starting test suite..." >&2
              echo "Running unit tests..." >&2
              echo "Test 1: PASS"
              echo "Test 2: PASS"
              echo "All tests completed successfully" >&2
              exit 0
            '';

          failing =
            mkSnapshot "test-failing" {
              exitCode = "1";
              stderr = ''
                Starting validation checks...
                Checking configuration...
                Warning: Deprecated option detected
                ERROR: Validation failed - missing required field
                Failed at line 42
              '';
            }
            <| mkResult "test-failing-actual" ''
              echo "Starting validation checks..." >&2
              echo "Checking configuration..." >&2
              echo "Warning: Deprecated option detected" >&2
              echo "ERROR: Validation failed - missing required field" >&2
              echo "Failed at line 42" >&2
              exit 1
            '';
        };
    };
}

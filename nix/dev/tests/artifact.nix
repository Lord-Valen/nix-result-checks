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
      # the capture wrapper's `touch` must preserve it.
      resultChecks.checks.artifact =
        let
          inherit (pkgs.resultChecks) mkResult mkSnapshot;
          check = mkResult "artifact-check" ''echo data > "$out"'';
          empty-check = mkResult "artifact-empty-check" "true";
        in
        {
          round-trip =
            mkSnapshot "artifact-round-trip" {
              exitCode = "0";
              stdout = ''
                data
              '';
            }
            <| mkResult "artifact-round-trip-actual" "cat ${check}";

          unwritten-is-empty =
            mkSnapshot "artifact-unwritten-is-empty" {
              exitCode = "0";
              stdout = "";
            }
            <| mkResult "artifact-unwritten-is-empty-actual" "cat ${empty-check}";
        };
    };
}

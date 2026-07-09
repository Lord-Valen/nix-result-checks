# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    {
      lib,
      pkgs,
      ...
    }:
    {
      # Unwritten and skipped checks leave an empty sentinel
      # indistinguishable at the filesystem level from a real empty
      # result, so consumers guard on the check's exit code via
      # `requireSuccess`/`requireSuccessWith`.
      resultChecks.checks.require-success =
        let
          inherit (pkgs.resultChecks)
            mkResult
            mkSkip
            mkSnapshot
            requireSuccess
            requireSuccessWith
            ;
          checks = import ./_lib/resultMocks.nix { inherit lib mkResult mkSkip; };

          ok-check = checks.pass."1";
          another-ok-check = checks.pass."2";
          failing-check = checks.fail."1";
          skipped-check = checks.skip."1";
          another-skipped-check = checks.skip."2";
        in
        {
          # requireSuccess canaries: just prove it works, pass and fail,
          # single check and a list. Every detailed structural property
          # (escaping, precise naming) is tested through
          # requireSuccessWith below instead, where a custom message
          # gives the precision to check for it.
          single-passes =
            mkSnapshot "single-passes" {
              exitCode = "0";
              stdout = ''
                ok
              '';
            }
            <| mkResult "single-passes-actual" (requireSuccess ok-check "echo ok");

          # Uses a genuine command failure rather than mkSkip's empty
          # sentinel: both land as != "0" in the guard, and only the
          # skip case was otherwise proven.
          single-fails =
            mkSnapshot "single-fails" { exitCode = "1"; }
            <| mkResult "single-fails-actual" (requireSuccess failing-check "echo should-not-run");

          list-passes =
            mkSnapshot "list-passes" {
              exitCode = "0";
              stdout = ''
                ok
              '';
            }
            <| mkResult "list-passes-actual" (requireSuccess [ ok-check another-ok-check ] "echo ok");

          list-fails =
            mkSnapshot "list-fails" { exitCode = "1"; }
            <| mkResult "list-fails-actual" (requireSuccess [ ok-check skipped-check ] "echo should-not-run");

          # requireSuccessWith structural properties, using a names-only
          # custom message so each test stays about the property under
          # test rather than requireSuccess's specific wording.
          names-only-broken-checks =
            mkSnapshot "names-only-broken-checks" {
              exitCode = "1";
              stderr = ''
                mock-skip-1
              '';
            }
            <| mkResult "names-only-broken-checks-actual" (requireSuccessWith {
              checks = [
                ok-check
                skipped-check
              ];
              message = c: c.passthru.checkName or c.name;
              command = "echo should-not-run";
            });

          names-every-broken-check-in-the-list =
            mkSnapshot "names-every-broken-check" {
              exitCode = "1";
              stderr = ''
                mock-skip-1
                mock-skip-2
              '';
            }
            <| mkResult "names-every-broken-check-actual" (requireSuccessWith {
              checks = [
                skipped-check
                another-skipped-check
              ];
              message = c: c.passthru.checkName or c.name;
              command = "echo should-not-run";
            });

          # A message containing a shell metacharacter must survive real
          # bash execution intact, not just look right as generated text.
          escapes-shell-metacharacters =
            mkSnapshot "escapes-shell-metacharacters" {
              exitCode = "1";
              stderr = ''
                it's broken
              '';
            }
            <| mkResult "escapes-shell-metacharacters" (requireSuccessWith {
              checks = [ skipped-check ];
              message = "it's broken";
              command = "echo should-not-run";
            });
        };
    };
}

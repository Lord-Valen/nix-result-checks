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
    let
      inherit (pkgs.resultChecks) mkEval mkSkip mkEntries;
      module = (import ../../modules/resultChecks/flakeModule.nix).perSystem;
      evalCfg =
        cfg:
        (lib.evalModules {
          modules = [
            module
            { resultChecks = cfg; }
            { _module.check = false; }
          ];
          specialArgs = { inherit pkgs lib; };
        }).config;

      passEntry = {
        kind = "eval";
        status = "pass";
        exitCode = "0";
        stdout = "";
        stderr = "";
      };
      skipEntry = {
        kind = "eval";
        status = "skip";
        exitCode = "";
        stdout = "";
        stderr = "";
      };
    in
    {
      resultChecks.checks = {
        # A live eval check, registered directly. Exercises module acceptance,
        # evalChecks exposure, and flake check wrappers end to end.
        eval = mkEval {
          testAddition = {
            expr = 1 + 1;
            expected = 2;
          };
          testConcat = {
            expr = "hello" + " " + "world";
            expected = "hello world";
          };
        };

        eval-entries = mkEval {
          testCheckIsTagged = {
            expr = (mkEval { }).kind;
            expected = "eval";
          };

          testSkipIsTagged = {
            expr = (mkSkip (mkEval { })).skip;
            expected = true;
          };

          testPassEntry = {
            expr = mkEntries (mkEval {
              t = {
                expr = 1;
                expected = 1;
              };
            });
            expected.t = passEntry;
          };

          testFailEntry = {
            expr = mkEntries (mkEval {
              t = {
                expr = 1 + 1;
                expected = 3;
              };
            });
            expected.t = {
              kind = "eval";
              status = "fail";
              exitCode = "1";
              stdout = ''
                FAIL: t
                  expected: 3
                  got:      2
              '';
              stderr = ''
                1 test(s) failed
              '';
            };
          };

          # A skipped check must not force test expressions.
          testSkippedCheckIsLazy = {
            expr = mkEntries (
              mkSkip (mkEval {
                t = {
                  expr = throw "never forced";
                  expected = 1;
                };
              })
            );
            expected.t = skipEntry;
          };

          testSkippedTestIsLazy = {
            expr = mkEntries (
              mkEval {
                t = {
                  expr = throw "never forced";
                  expected = 1;
                };
                u = {
                  expr = 1;
                  expected = 1;
                };
              }
              // {
                skipTests = [ "t" ];
              }
            );
            expected = {
              t = skipEntry;
              u = passEntry;
            };
          };
        };

        eval-module = mkEval {
          testEvalChecksExposed = {
            expr =
              (evalCfg {
                checks.fixture = mkEval {
                  t = {
                    expr = 1;
                    expected = 1;
                  };
                };
              }).resultChecks.evalChecks;
            expected.fixture.t = passEntry;
          };

          testSkipChecksByTest = {
            expr =
              (evalCfg {
                skipChecks = [ "fixture:t" ];
                checks.fixture = mkEval {
                  t = {
                    expr = throw "never forced";
                    expected = 1;
                  };
                  u = {
                    expr = 1;
                    expected = 1;
                  };
                };
              }).resultChecks.evalChecks.fixture;
            expected = {
              t = skipEntry;
              u = passEntry;
            };
          };

          testSkipChecksWholeCheck = {
            expr =
              (evalCfg {
                skipChecks = [ "fixture" ];
                checks.fixture = mkEval {
                  t = {
                    expr = throw "never forced";
                    expected = 1;
                  };
                };
              }).resultChecks.evalChecks.fixture.t;
            expected = skipEntry;
          };

          # The report covers derivation checks only; an eval check must not
          # reach the generator nor force its test expressions.
          testReportExcludesEvalChecks = {
            expr =
              lib.isDerivation
                (evalCfg {
                  checks.fixture = mkEval {
                    t = {
                      expr = throw "never forced";
                      expected = 1;
                    };
                  };
                }).resultChecks.report;
            expected = true;
          };

          testEvalChecksAbsentFromReportSet = {
            expr =
              (evalCfg {
                checks.fixture = mkEval {
                  t = {
                    expr = 1;
                    expected = 1;
                  };
                };
                checks.plain = pkgs.resultChecks.mkResult "plain" "exit 0";
              }).resultChecks.reportChecks;
            expected = [ "plain" ];
          };
        };

        # The select script nrc embeds for nix-eval-jobs: wraps each pre-entry
        # in a stub derivation so results ride the derivation-shaped output.
        eval-select = mkEval {
          testWrapsLeavesInDerivations = {
            expr =
              let
                mapped = import ../../../src/runner/select.nix {
                  fixture.t = passEntry;
                };
              in
              {
                isDrv = lib.isDerivation mapped.fixture.t;
                result = mapped.fixture.t.result;
              };
            expected = {
              isDrv = true;
              result = passEntry;
            };
          };

          # Wrapping must not force entry values; nix-eval-jobs workers force
          # them, that is the entire point.
          testWrapIsLazy = {
            expr =
              let
                mapped = import ../../../src/runner/select.nix {
                  fixture.t = throw "never forced";
                };
              in
              lib.isDerivation mapped.fixture.t;
            expected = true;
          };
        };
      };
    };
}

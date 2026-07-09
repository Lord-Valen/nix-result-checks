# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

# A static, fixed-shape set of dummy result checks of every status
# ("pass", "fail", "skip"), keyed by status then "1".."maxCount". No
# count parameter: every caller gets the identical shape, so imports
# from different test files can't drift into asking for different
# counts and generating mismatched sets. Numbers are assigned by this
# range, not by hand, so a caller can't skip or duplicate one.
# Unreferenced status/number combinations cost nothing: Nix never
# builds a value nobody reaches.
#
# "pass"/"fail" run a real command; "skip" wraps a would-be-passing one
# in mkSkip, so its exitCode is the empty-sentinel case rather than a
# real exit code.
#
# Named plainly ("mock-<status>-<number>", no caller-specific prefix)
# so the same call from different test files produces the identical
# derivation and shares one build, rather than each file's fixtures
# building their own copy.
{
  lib,
  mkResult,
  mkSkip,
}:
let
  maxCount = 3;
in
lib.genAttrs [ "pass" "fail" "skip" ] (
  status:
  lib.genAttrs (map toString (lib.range 1 maxCount)) (
    n:
    let
      name = "mock-${status}-${n}";
    in
    if status == "pass" then
      mkResult name ''echo data > "$out"''
    else if status == "fail" then
      mkResult name "exit 1"
    else
      mkSkip (mkResult name ''echo data > "$out"'')
  )
)

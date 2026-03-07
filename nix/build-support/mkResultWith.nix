# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  lib,
  mkSkip,
  runCommandWith,
  stdenvNoCC,
}:
{
  stdenv ? stdenvNoCC,
  ...
}@runCommandAttrs:
buildCommand:
let
  attrs = lib.recursiveUpdate {
    derivationArgs = {
      outputs = [
        "out"
        "stdout"
        "stderr"
        "exitCode"
      ];
    };
  } runCommandAttrs;
  isSkip = attrs.derivationArgs.passthru.skip or false;
  result = runCommandWith attrs buildCommand;
in
if isSkip then mkSkip result else result

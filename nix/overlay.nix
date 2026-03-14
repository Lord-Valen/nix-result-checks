# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

final: prev:
let
  inherit (prev) lib;
  discover =
    directories:
    lib.makeScope prev.newScope (
      self:
      lib.foldl' (
        acc: directory:
        acc
        // lib.packagesFromDirectoryRecursive {
          inherit (self) callPackage newScope;
          inherit directory;
        }
      ) { } directories
    );
in
{
  resultChecks = discover [
    ./build-support
    ./generators
    ./docs
    ./apps
  ];
}

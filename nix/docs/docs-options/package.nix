# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{ lib, nixosOptionsDoc }:
let
  evaluated = lib.evalModules {
    modules = [
      (import ../../modules/resultChecks/flakeModule.nix).perSystem
      { _module.check = false; }
    ];
    specialArgs = {
      pkgs = { };
      inherit lib;
    };
  };
in
(nixosOptionsDoc {
  options = lib.removeAttrs evaluated.options [ "_module" ];
}).optionsCommonMark

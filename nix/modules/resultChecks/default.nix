# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT-0

{
  flake.flakeModules.default = {
    imports = [ ./flakeModule.nix ];
  };
  flake.overlays.default = import ../../overlay.nix;
  flake.lib = pkgs: (import ../../overlay.nix pkgs pkgs).resultChecks;
}

# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT-0

{
  overlays.default = import ./nix/overlay.nix;
  flakeModules.default = import ./nix/modules/resultChecks/flakeModule.nix;
}

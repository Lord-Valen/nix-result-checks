# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

# Everything nrc needs from a flake lives under this one reserved
# output: resultChecks.<system> = { report; evalChecks; }.
# Defined manually instead of via transposition to keep the public
# surface to exactly these two attributes.
{ config, lib, ... }:
{
  flake.resultChecks = lib.mapAttrs (_system: v: {
    inherit (v.resultChecks) report evalChecks;
  }) config.allSystems;
}

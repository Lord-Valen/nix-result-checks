# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT-0

{ config, inputs, ... }:
let
  inherit (config.flake) overlays;
in
{
  imports = [
    ../modules/resultChecks/flakeModule.nix
    ./tests.nix
  ];
  perSystem =
    {
      config,
      pkgs,
      system,
      ...
    }:
    {
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [
          overlays.default
        ];
      };

      packages.checks-report = config.resultChecks.report;
      apps.run-checks = {
        type = "app";
        program = toString (
          pkgs.writeShellScript "run-checks" ''
            cat ${config.resultChecks.report}
          ''
        );
      };
    };
}

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
    ./devshell.nix
    ./nrc.nix
    (inputs.import-tree ./tests)
  ];
  perSystem =
    {
      config,
      lib,
      pkgs,
      system,
      ...
    }:
    {
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [
          overlays.default
          inputs.rust-overlay.overlays.default
        ];
      };

      packages.docs-options = pkgs.resultChecks.docs-options;
      packages.docs-mdbook = pkgs.resultChecks.docs-mdbook;
      packages.docs-man = pkgs.resultChecks.docs-man;
      packages.checks-report = config.resultChecks.report;
      packages.nrc = pkgs.resultChecks.nrc;
      apps.run-checks = {
        type = "app";
        program = toString (
          pkgs.writeShellScript "run-checks" ''
            cat ${config.resultChecks.report}
          ''
        );
      };
      apps.nrc = {
        type = "app";
        program = toString (
          pkgs.writeShellScript "nrc" ''
            ${pkgs.resultChecks.nrc}/bin/nrc ${config.resultChecks.report}
          ''
        );
      };
    };
}

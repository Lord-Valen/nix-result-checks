# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT-0

{
  description = "Derivation based testing framework";

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = with inputs.flake-parts.flakeModules; [
        flakeModules
        partitions
        ./nix/modules/resultChecks
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      partitions.dev = {
        extraInputsFlake = ./nix/dev;
        module = ./nix/dev/flake-module.nix;
      };
      partitionedAttrs = {
        checks = "dev";
        devShells = "dev";
        packages = "dev";
        apps = "dev";
      };
    };

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
  };
}

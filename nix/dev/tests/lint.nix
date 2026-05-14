# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    {
      resultChecks.checks =
        let
          inherit (pkgs.resultChecks) mkResult;
        in
        {
          rustfmt =
            let
              src = config.packages.nrc-dev.src;
            in
            (mkResult "rustfmt" "cargo fmt --manifest-path ${src}/Cargo.toml -- --check").overrideAttrs {
              nativeBuildInputs = [
                (pkgs.rust-bin.fromRustupToolchainFile ../../../rust-toolchain.toml)
              ];
            };

          nixfmt = (mkResult "nixfmt" ''
            nixfmt --check \
              $(find ${../../../nix} -name '*.nix') \
              ${../../../flake.nix}
          '').overrideAttrs {
            nativeBuildInputs = [
              pkgs.nixfmt
              pkgs.findutils
            ];
          };
        };
    };
}

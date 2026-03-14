# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  perSystem =
    { config, pkgs, ... }:
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
            mkResult "rustfmt"
              {
                nativeBuildInputs = [
                  (pkgs.rust-bin.fromRustupToolchainFile ../../../rust-toolchain.toml)
                ];
              }
              ''
                cargo fmt --manifest-path ${src}/Cargo.toml -- --check
              '';

          nixfmt =
            mkResult "nixfmt"
              {
                nativeBuildInputs = [ pkgs.nixfmt pkgs.findutils ];
              }
              ''
                find ${../../../nix} -name '*.nix' -exec nixfmt --check {} +
                nixfmt --check ${../../../flake.nix}
              '';
        };
    };
}

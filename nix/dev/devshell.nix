# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT-0

{
  perSystem =
    { pkgs, ... }:
    {
      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          (rust-bin.fromRustupToolchainFile ../../rust-toolchain.toml)
          cargo-insta
        ];
      };
    };
}

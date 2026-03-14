# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT-0

{
  inputs = {
    crane.url = "github:ipetkov/crane";
    import-tree.url = "github:vic/import-tree";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.xz";
  };
  outputs = _: { };
}

# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  lib,
  makeWrapper,
  nix-eval-jobs,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "nrc";
  version = "2.1.0";

  src = ../../../.;

  cargoLock.lockFile = ../../../Cargo.lock;

  nativeBuildInputs = [ makeWrapper ];

  # nix-eval-jobs powers the parallel evalChecks path in flake
  # convention mode; without it nrc falls back to sequential nix eval.
  postInstall = ''
    wrapProgram $out/bin/nrc \
      --prefix PATH : ${lib.makeBinPath [ nix-eval-jobs ]}
  '';

  meta.mainProgram = "nrc";
}

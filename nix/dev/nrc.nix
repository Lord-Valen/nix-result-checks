# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT-0

{ inputs, ... }:
{
  perSystem =
    { lib, pkgs, ... }:
    let
      craneLib = inputs.crane.mkLib pkgs;
      src = lib.fileset.toSource {
        root = ../../.;
        fileset = lib.fileset.unions [
          (craneLib.fileset.commonCargoSources ../../.)
          ../../keymaps
          ../../src/render/snapshots
          ../../src/runner/select.nix
        ];
      };
      cargoArtifacts = craneLib.buildDepsOnly { inherit src; };
    in
    {
      packages.nrc-dev = craneLib.buildPackage {
        inherit src cargoArtifacts;
        doCheck = false;
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postInstall = ''
          wrapProgram $out/bin/nrc \
            --prefix PATH : ${lib.makeBinPath [ pkgs.nix-eval-jobs ]}
        '';
      };
      checks.nrc-build = craneLib.buildPackage { inherit src cargoArtifacts; };
    };
}

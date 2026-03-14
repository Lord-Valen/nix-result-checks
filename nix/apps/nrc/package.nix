# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "nrc";
  version = "0.1.0";

  src = ../../../.;

  cargoLock.lockFile = ../../../Cargo.lock;

  meta.mainProgram = "nrc";
}

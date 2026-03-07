# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{ }:
drv:
drv.overrideAttrs (prev: {
  passthru = (prev.passthru or { }) // {
    skip = true;
  };
  buildCommand = ''
    touch $out
    touch $stdout
    touch $stderr
    touch $exitCode
  '';
  buildInputs = [ ];
  nativeBuildInputs = [ ];
})

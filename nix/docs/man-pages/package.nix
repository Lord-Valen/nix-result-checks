# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  lib,
  nixdoc,
  options-doc,
  pandoc,
  runCommand,
}:
let
  helpers = {
    mkSkip = ../../build-support/mkSkip.nix;
    mkResultWith = ../../build-support/mkResultWith.nix;
    mkResult = ../../build-support/mkResult.nix;
    mkSnapshot = ../../build-support/mkSnapshot.nix;
    mkEval = ../../build-support/mkEval.nix;
  };
  generators = {
    json = ../../generators/json/package.nix;
    kdl = ../../generators/kdl/package.nix;
  };
  manPage = section: name: file: ''
    nixdoc --category "${name}" --description "" --file ${file} \
      | sed "s/^# *{#sec-functions-library-\([^}]*\)}$/# \1/" \
      | pandoc -f markdown -t man --standalone \
          --shift-heading-level-by=-1 \
          --metadata title="${name}" \
          --metadata section="${toString section}" \
      | sed \
          -e '0,/^\.PP$/{s/^\.PP$/.SH DESCRIPTION\n.PP/}' \
          -e '/^\.IP$/{N; s/^\.IP\n\.EX$/.EX/}' \
          -e 's/^\.SH Type$/.SH TYPE/' \
          -e 's/^\.SH Arguments$/.SH ARGUMENTS/' \
          -e 's/^\.SH Example$/.SH EXAMPLES/' \
      > $out/share/man/man${toString section}/${name}.${toString section}
  '';
in
runCommand "man-pages"
  {
    nativeBuildInputs = [
      nixdoc
      pandoc
    ];
  }
  ''
    mkdir -p $out/share/man/man3 $out/share/man/man5
    ${lib.concatMapStrings (name: manPage 3 name helpers.${name}) (lib.attrNames helpers)}
    ${lib.concatMapStrings (name: manPage 3 name generators.${name}) (lib.attrNames generators)}
    pandoc -f markdown -t man --standalone \
      --shift-heading-level-by=-1 \
      --metadata title="nix-result-checks" \
      --metadata section="5" \
      ${options-doc} \
      | sed -e '/^\.IP$/{N; s/^\.IP\n\.EX$/.EX/}' \
      > $out/share/man/man5/nix-result-checks.5
  ''

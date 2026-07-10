# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

{
  lib,
  mdbook,
  nixdoc,
  docs-options,
  runCommand,
  writeText,
}:
let
  bookToml = writeText "book.toml" ''
    [book]
    title = "nix-result-checks"
    src = "src"
  '';
  summary = writeText "SUMMARY.md" ''
    # Summary

    - [Build Support]()
      - [mkResult](helpers/mkResult.md)
      - [mkResultWith](helpers/mkResultWith.md)
      - [mkSnapshot](helpers/mkSnapshot.md)
      - [mkEval](helpers/mkEval.md)
      - [mkSkip](helpers/mkSkip.md)
      - [mkEntries](helpers/mkEntries.md)
      - [mkReport](helpers/mkReport.md)
      - [mkEvalChecks](helpers/mkEvalChecks.md)
    - [Generators]()
      - [json](generators/json.md)
    - [Options](options.md)
  '';
  helpers = {
    mkSkip = ../../build-support/mkSkip.nix;
    mkEntries = ../../build-support/mkEntries.nix;
    mkEvalChecks = ../../build-support/mkEvalChecks.nix;
    mkReport = ../../build-support/mkReport.nix;
    mkResultWith = ../../build-support/mkResultWith.nix;
    mkResult = ../../build-support/mkResult.nix;
    mkSnapshot = ../../build-support/mkSnapshot.nix;
    mkEval = ../../build-support/mkEval.nix;
    requireSuccess = ../../build-support/requireSuccess.nix;
    requireSuccessWith = ../../build-support/requireSuccessWith.nix;
  };
  generators = {
    json = ../../generators/json/package.nix;
  };
  nixdocPage = name: file: ''
    nixdoc --category "${name}" --description "" --file ${file} \
      | sed "s/^# *{#sec-functions-library-\([^}]*\)}$/# \1/" \
      > src/${name}.md
  '';
in
runCommand "docs-mdbook"
  {
    nativeBuildInputs = [
      mdbook
      nixdoc
    ];
  }
  ''
    mkdir -p src/helpers src/generators
    cp ${bookToml} book.toml
    cp ${summary} src/SUMMARY.md
    ${lib.concatMapStrings (name: nixdocPage "helpers/${name}" helpers.${name}) (lib.attrNames helpers)}
    ${lib.concatMapStrings (name: nixdocPage "generators/${name}" generators.${name}) (
      lib.attrNames generators
    )}
    cp ${docs-options} src/options.md
    mdbook build -d $out
  ''

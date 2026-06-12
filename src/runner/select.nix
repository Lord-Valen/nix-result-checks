# SPDX-FileCopyrightText: 2026 Lord-Valen
#
# SPDX-License-Identifier: MIT

# nix-eval-jobs emits one JSON line per derivation it finds, so each
# pre-entry leaf of the evalChecks tree is wrapped in a stub derivation
# that is never instantiated (nrc passes --no-instantiate). The entry
# itself rides along in `result`, which nrc extracts with
# --apply 'drv: drv.result'. The stub's fields are constants: only the
# wrapper's attribute path and `result` carry information.
evalChecks:
builtins.mapAttrs (
  _check:
  builtins.mapAttrs (
    _test: entry:
    derivation {
      name = "eval-check";
      system = "x86_64-linux";
      builder = "/bin/sh";
    }
    // {
      result = entry;
    }
  )
) evalChecks

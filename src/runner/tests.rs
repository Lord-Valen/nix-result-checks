// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use super::*;

fn fail_line() -> &'static str {
    r#"{"attr":"testFail","attrPath":["fixture","testFail"],"constituents":[],"drvPath":"/nix/store/x-eval-check.drv","extraValue":{"exitCode":"1","kind":"eval","status":"fail","stderr":"1 test(s) failed\n","stdout":"FAIL: testFail\n  expected: 3\n  got:      2\n"},"name":"eval-check","outputs":{"out":"/nix/store/x-eval-check"},"system":"x86_64-linux"}"#
}

#[test]
fn parses_extra_value_into_entry() {
    let entry = entry_from_line(fail_line()).unwrap();
    assert_eq!(entry.suite.as_deref(), Some("fixture"));
    assert_eq!(entry.name, "testFail");
    assert_eq!(entry.kind, EntryKind::Eval);
    assert_eq!(entry.status, Status::Fail);
    assert_eq!(entry.exit_code, "1");
    assert_eq!(
        entry.stdout,
        "FAIL: testFail\n  expected: 3\n  got:      2\n"
    );
    assert_eq!(entry.stderr, "1 test(s) failed\n");
}

#[test]
fn parses_error_line_into_failing_entry() {
    let line = r#"{"attr":"testError","attrPath":["fixture","testError"],"error":"error: boom","fatal":false}"#;
    let entry = entry_from_line(line).unwrap();
    assert_eq!(entry.suite.as_deref(), Some("fixture"));
    assert_eq!(entry.name, "testError");
    assert_eq!(entry.status, Status::Fail);
    assert_eq!(entry.exit_code, "1");
    assert_eq!(entry.stderr, "error: boom");
}

#[test]
fn rejects_line_without_value_or_error() {
    let line = r#"{"attr":"x","attrPath":["fixture","x"]}"#;
    assert!(entry_from_line(line).is_err());
}

#[test]
fn parses_eval_json_tree() {
    let tree = serde_json::json!({
        "fixture": {
            "passes": {
                "kind": "eval", "status": "pass", "exitCode": "0",
                "stdout": "", "stderr": ""
            },
            "skipped": {
                "kind": "eval", "status": "skip", "exitCode": "",
                "stdout": "", "stderr": ""
            }
        }
    });
    let entries = entries_from_tree(tree).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .all(|entry| entry.suite.as_deref() == Some("fixture"))
    );
    let skipped = entries.iter().find(|e| e.name == "skipped").unwrap();
    assert_eq!(skipped.status, Status::Skip);
    assert_eq!(skipped.exit_code, "");
}

#[test]
fn nej_args_target_eval_checks() {
    let args = nej_args(".", "x86_64-linux", 8);
    assert!(args.contains(&"--no-instantiate".to_owned()));
    assert!(args.contains(&"--force-recurse".to_owned()));
    let option_at = args.iter().position(|a| a == "--option").unwrap();
    assert_eq!(args[option_at + 1], "eval-cache");
    assert_eq!(args[option_at + 2], "false");
    assert!(args.contains(&".#resultChecks.x86_64-linux.evalChecks".to_owned()));
    assert!(args.contains(&APPLY.to_owned()));
    assert!(args.contains(&SELECT.to_owned()));
    let workers_at = args.iter().position(|a| a == "--workers").unwrap();
    assert_eq!(args[workers_at + 1], "8");
}

#[test]
fn select_script_is_embedded() {
    assert!(SELECT.contains("mapAttrs"));
    assert!(SELECT.contains("result = entry"));
}

#[test]
fn file_mode_projects_eval_checks_from_the_data_type() {
    let select = file_select();
    assert!(select.starts_with("root: ("));
    assert!(select.ends_with(") root.evalChecks"));

    let args = nej_file_args("./checks.nix", 4);
    assert!(args.contains(&"--no-instantiate".to_owned()));
    assert!(args.contains(&select));
    assert_eq!(args.last().map(String::as_str), Some("./checks.nix"));
}

#[test]
fn report_attr_follows_convention() {
    assert_eq!(
        report_attr("github:foo/bar", "aarch64-darwin"),
        "github:foo/bar#resultChecks.aarch64-darwin.report"
    );
}

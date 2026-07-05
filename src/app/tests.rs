// SPDX-FileCopyrightText: 2026 Lord-Valen
//
// SPDX-License-Identifier: MIT

use super::*;

fn entry(name: &str, status: Status, suite: Option<&str>) -> CheckEntry {
    CheckEntry {
        name: name.to_string(),
        status,
        kind: EntryKind::Result,
        exit_code: "0".to_string(),
        stdout: String::new(),
        stderr: String::new(),
        suite: suite.map(str::to_owned),
        children: Vec::new(),
    }
}

fn flat(name: &str) -> CheckEntry {
    entry(name, Status::Pass, None)
}

fn suite_check(suite: &str, name: &str) -> CheckEntry {
    entry(name, Status::Pass, Some(suite))
}

fn with_children(mut parent: CheckEntry, children: Vec<CheckEntry>) -> CheckEntry {
    parent.children = children;
    parent
}

#[test]
fn upsert_flat_check() {
    let mut app = App::new();
    app.upsert(flat("lint"));
    assert!(app.entries.contains_key("lint"));
    assert!(matches!(&app.order[0], OrderItem::Check(k) if k == "lint"));
}

#[test]
fn upsert_suite_check_creates_suite() {
    let mut app = App::new();
    app.upsert(suite_check("db", "schema"));
    assert!(app.entries.contains_key("db:schema"));
    assert!(
        matches!(&app.order[0], OrderItem::Suite { name, checks } if name == "db" && checks == &["db:schema"])
    );
}

#[test]
fn upsert_suite_check_appends_to_existing_suite() {
    let mut app = App::new();
    app.upsert(suite_check("db", "schema"));
    app.upsert(suite_check("db", "migration"));
    assert_eq!(app.order.len(), 1);
    if let OrderItem::Suite { checks, .. } = &app.order[0] {
        assert_eq!(checks, &["db:schema", "db:migration"]);
    } else {
        panic!("expected suite");
    }
}

#[test]
fn upsert_is_idempotent() {
    let mut app = App::new();
    app.upsert(flat("lint"));
    app.upsert(flat("lint"));
    assert_eq!(app.order.len(), 1);
    assert_eq!(app.entries.len(), 1);
}

#[test]
fn prune_removes_stale_flat() {
    let mut app = App::new();
    app.upsert(flat("lint"));
    app.bump_generation();
    app.upsert(flat("fmt"));
    app.prune();
    assert!(!app.entries.contains_key("lint"));
    assert!(app.entries.contains_key("fmt"));
}

#[test]
fn prune_removes_stale_suite_check() {
    let mut app = App::new();
    app.upsert(suite_check("db", "schema"));
    app.bump_generation();
    app.upsert(suite_check("db", "migration"));
    app.prune();
    assert!(!app.entries.contains_key("db:schema"));
    assert!(app.entries.contains_key("db:migration"));
    if let OrderItem::Suite { checks, .. } = &app.order[0] {
        assert_eq!(checks, &["db:migration"]);
    }
}

#[test]
fn prune_removes_empty_suite() {
    let mut app = App::new();
    app.upsert(suite_check("db", "schema"));
    app.bump_generation();
    app.prune();
    assert!(app.order.is_empty());
}

#[test]
fn prune_clears_folded_state_for_removed_suite() {
    let mut app = App::new();
    app.upsert(suite_check("db", "schema"));
    app.toggle_suite("db");
    assert!(app.folded_suites.contains("db"));
    app.bump_generation();
    app.prune();
    assert!(!app.folded_suites.contains("db"));
}

#[test]
fn all_keys_flat_and_suite() {
    let mut app = App::new();
    app.upsert(flat("lint"));
    app.upsert(suite_check("db", "schema"));
    app.upsert(suite_check("db", "migration"));
    let keys: Vec<&str> = app.all_keys().map(|s| s.as_str()).collect();
    assert_eq!(keys, ["lint", "db:schema", "db:migration"]);
}

#[test]
fn visible_items_flat_checks() {
    let mut app = App::new();
    app.upsert(flat("lint"));
    app.upsert(flat("fmt"));
    let vis = app.visible_items();
    assert!(matches!(&vis[0], VisibleItem::Check { key: k, .. } if k == "lint"));
    assert!(matches!(&vis[1], VisibleItem::Check { key: k, .. } if k == "fmt"));
}

#[test]
fn visible_items_suite_unfolded() {
    let mut app = App::new();
    app.upsert(suite_check("db", "schema"));
    app.upsert(suite_check("db", "migration"));
    let vis = app.visible_items();
    assert!(matches!(&vis[0], VisibleItem::Suite(n) if n == "db"));
    assert!(matches!(&vis[1], VisibleItem::Check { key: k, .. } if k == "db:schema"));
    assert!(matches!(&vis[2], VisibleItem::Check { key: k, .. } if k == "db:migration"));
}

#[test]
fn visible_items_suite_folded() {
    let mut app = App::new();
    app.upsert(suite_check("db", "schema"));
    app.upsert(suite_check("db", "migration"));
    app.toggle_suite("db");
    let vis = app.visible_items();
    assert_eq!(vis.len(), 1);
    assert!(matches!(&vis[0], VisibleItem::Suite(n) if n == "db"));
}

#[test]
fn toggle_suite_unfolds() {
    let mut app = App::new();
    app.upsert(suite_check("db", "schema"));
    app.toggle_suite("db");
    assert!(app.folded_suites.contains("db"));
    app.toggle_suite("db");
    assert!(!app.folded_suites.contains("db"));
}

// -- Children --

#[test]
fn upsert_links_children_under_the_parent_key() {
    let mut app = App::new();
    let snapshot = with_children(flat("snap"), vec![flat("actual")]);
    app.upsert(snapshot);
    assert!(app.entries.contains_key("snap"));
    assert!(app.entries.contains_key("snap:actual"));
    // Children are not their own order entry — only reachable via child_keys.
    assert_eq!(app.order.len(), 1);
    assert_eq!(
        app.child_keys.get("snap").map(Vec::as_slice),
        Some(["snap:actual".to_string()].as_slice())
    );
}

#[test]
fn upsert_links_children_of_a_suite_check() {
    let mut app = App::new();
    let snapshot = with_children(suite_check("db", "snap"), vec![flat("actual")]);
    app.upsert(snapshot);
    assert!(app.entries.contains_key("db:snap:actual"));
    assert_eq!(
        app.child_keys.get("db:snap").map(Vec::as_slice),
        Some(["db:snap:actual".to_string()].as_slice())
    );
}

#[test]
fn children_are_folded_by_default() {
    let mut app = App::new();
    app.upsert(with_children(flat("snap"), vec![flat("actual")]));
    let vis = app.visible_items();
    assert_eq!(vis.len(), 1, "child should be hidden until unfolded");
    assert!(app.folded_checks.contains("snap"));
}

#[test]
fn toggle_children_unfolds() {
    let mut app = App::new();
    app.upsert(with_children(flat("snap"), vec![flat("actual")]));
    app.toggle_children("snap");
    assert!(!app.folded_checks.contains("snap"));
    assert_eq!(app.visible_items().len(), 2);
    app.toggle_children("snap");
    assert!(app.folded_checks.contains("snap"));
    assert_eq!(app.visible_items().len(), 1);
}

#[test]
fn visible_items_nest_children_by_depth() {
    let mut app = App::new();
    let grandchild = flat("actual");
    let child = with_children(flat("actual"), vec![grandchild]);
    let snapshot = with_children(flat("snap"), vec![child]);
    app.upsert(snapshot);
    app.toggle_children("snap");
    app.toggle_children("snap:actual");
    let vis = app.visible_items();
    assert!(matches!(&vis[0], VisibleItem::Check { key, depth: 0 } if key == "snap"));
    assert!(matches!(&vis[1], VisibleItem::Check { key, depth: 1 } if key == "snap:actual"));
    assert!(matches!(&vis[2], VisibleItem::Check { key, depth: 2 } if key == "snap:actual:actual"));
}

#[test]
fn prune_removes_children_of_a_pruned_check() {
    let mut app = App::new();
    app.upsert(with_children(flat("snap"), vec![flat("actual")]));
    app.bump_generation();
    app.prune();
    assert!(!app.entries.contains_key("snap:actual"));
    assert!(app.child_keys.get("snap").is_none());
}

// -- Counts --

#[test]
fn counts_global() {
    let mut app = App::new();
    app.upsert(entry("a", Status::Pass, None));
    app.upsert(entry("b", Status::Pass, None));
    app.upsert(entry("c", Status::Fail, None));
    app.upsert(entry("d", Status::Skip, None));
    assert_eq!(app.counts(), (2, 1, 1));
}

#[test]
fn suite_counts_per_suite() {
    let mut app = App::new();
    app.upsert(entry("schema", Status::Pass, Some("db")));
    app.upsert(entry("migration", Status::Fail, Some("db")));
    app.upsert(entry("lint", Status::Pass, None));
    assert_eq!(app.suite_counts("db"), (1, 1, 0));
}

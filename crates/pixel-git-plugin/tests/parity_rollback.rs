// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! `plan-rollback` against the built-in `pixel plan-rollback`.

use pixel_git_plugin::testing::{Setup, assert_parity, git, git_fixture, run, write};

const BIN: &str = env!("CARGO_BIN_EXE_pixel-git");

fn parity(case: &str, args: &'static [&'static str]) {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "plan-rollback",
        case,
        &|| Setup::new(git_fixture(), args),
    );
}

/// `--apply` needs the first commit's id, which differs per fixture.
fn apply_case(case: &str, extra: &'static [&'static str]) {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "plan-rollback",
        case,
        &|| {
            let repo = git_fixture();
            let first = git(repo.path(), &["rev-list", "--max-parents=0", "HEAD"]);
            let mut args = vec!["--apply", first.as_str(), "--file", "src/lib.rs"];
            args.extend_from_slice(extra);
            Setup::new(repo, &args)
        },
    );
}

#[test]
fn a_plan_for_an_explicit_file() {
    parity(
        "plan-file",
        &["alpha stopped working", "--file", "src/lib.rs"],
    );
}

#[test]
fn a_plan_for_an_explicit_file_as_json() {
    parity(
        "plan-file-json",
        &["alpha stopped working", "--file", "src/lib.rs", "--json"],
    );
}

#[test]
fn a_shallow_depth_caps_the_history() {
    parity(
        "plan-depth-1-json",
        &[
            "alpha stopped working",
            "--file",
            "src/lib.rs",
            "--depth",
            "1",
            "--json",
        ],
    );
}

#[test]
fn a_file_with_no_history_is_planned_empty() {
    parity(
        "plan-unknown-file-json",
        &["x", "--file", "nope.rs", "--json"],
    );
}

#[test]
fn targets_are_located_from_the_problem_when_no_file_is_given() {
    parity("plan-located", &["alpha stopped working"]);
}

#[test]
fn targets_are_located_from_the_problem_as_json() {
    parity("plan-located-json", &["alpha stopped working", "--json"]);
}

#[test]
fn a_plan_needs_a_problem() {
    parity("missing-problem-json", &["--json"]);
}

#[test]
fn apply_restores_the_file() {
    apply_case("apply", &[]);
}

#[test]
fn apply_reports_the_restore_as_json() {
    apply_case("apply-json", &["--json"]);
}

#[test]
fn a_ref_that_looks_like_an_option_is_refused() {
    parity(
        "apply-option-injection-json",
        &["--apply=-bad", "--file", "src/lib.rs", "--json"],
    );
}

#[test]
fn apply_really_rewrites_the_working_tree_and_leaves_head_alone() {
    let repo = git_fixture();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let first = git(repo.path(), &["rev-list", "--max-parents=0", "HEAD"]);
    let seen = run(
        BIN.as_ref(),
        &["plan-rollback", "--apply", &first, "--file", "src/lib.rs"],
        repo.path(),
        &[],
    );
    assert_eq!(seen.code, 0, "{}", seen.stderr);
    let text = std::fs::read_to_string(repo.path().join("src/lib.rs")).unwrap();
    assert!(
        text.contains("    1\n"),
        "restored to the first version: {text}"
    );
    assert_eq!(git(repo.path(), &["rev-parse", "HEAD"]), head);
}

#[test]
fn apply_refuses_dirty_files_unless_told_how() {
    let repo = git_fixture();
    let first = git(repo.path(), &["rev-list", "--max-parents=0", "HEAD"]);
    write(
        repo.path(),
        "src/lib.rs",
        "pub fn alpha() -> u32 {\n    99\n}\n",
    );
    let seen = run(
        BIN.as_ref(),
        &["plan-rollback", "--apply", &first, "--file", "src/lib.rs"],
        repo.path(),
        &[],
    );
    assert_ne!(seen.code, 0, "{}", seen.stdout);
    let text = std::fs::read_to_string(repo.path().join("src/lib.rs")).unwrap();
    assert!(
        text.contains("99"),
        "in-progress edit survives a refused apply"
    );
}

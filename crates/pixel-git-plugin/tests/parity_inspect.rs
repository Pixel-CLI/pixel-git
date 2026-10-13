// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The read-only commands answered by `pixel-ops`, against the built-in ones.

use pixel_git_plugin::testing::{Setup, add_origin, assert_parity, git, git_fixture, write};

const BIN: &str = env!("CARGO_BIN_EXE_pixel-git");

fn parity(command: &str, case: &str, args: &'static [&'static str]) {
    assert_parity(env!("CARGO_MANIFEST_DIR"), BIN, command, case, &|| {
        Setup::new(git_fixture(), args)
    });
}

/// A tracked file edited, a new file staged and one left untracked.
fn dirty_repo() -> tempfile::TempDir {
    let repo = git_fixture();
    write(
        repo.path(),
        "src/lib.rs",
        "pub fn alpha() -> u32 {\n    3\n}\n",
    );
    write(repo.path(), "staged.txt", "staged\n");
    git(repo.path(), &["add", "staged.txt"]);
    write(repo.path(), "untracked.txt", "untracked\n");
    repo
}

fn parity_dirty(command: &str, case: &str, args: &'static [&'static str]) {
    assert_parity(env!("CARGO_MANIFEST_DIR"), BIN, command, case, &|| {
        Setup::new(dirty_repo(), args)
    });
}

// --- repo-state ------------------------------------------------------------

#[test]
fn repo_state_of_a_clean_tree() {
    parity("repo-state", "repo-state-clean", &[]);
}

#[test]
fn repo_state_of_a_clean_tree_as_json() {
    parity("repo-state", "repo-state-clean-json", &["--json"]);
}

#[test]
fn repo_state_can_list_the_clean_files() {
    parity(
        "repo-state",
        "repo-state-include-clean-json",
        &[".", "--include-clean", "--json"],
    );
}

#[test]
fn repo_state_of_a_dirty_tree_as_json() {
    parity_dirty("repo-state", "repo-state-dirty-json", &["--json"]);
}

#[test]
fn repo_state_of_a_dirty_tree() {
    parity_dirty("repo-state", "repo-state-dirty", &[]);
}

#[test]
fn repo_state_restricted_to_files() {
    parity_dirty(
        "repo-state",
        "repo-state-files-json",
        &[
            "--files",
            "src/lib.rs",
            "--files",
            "untracked.txt",
            "--include-clean",
            "--json",
        ],
    );
}

// --- review-changes --------------------------------------------------------

#[test]
fn review_changes_of_a_dirty_tree_as_json() {
    parity_dirty("review-changes", "review-changes-dirty-json", &["--json"]);
}

#[test]
fn review_changes_of_a_dirty_tree() {
    parity_dirty("review-changes", "review-changes-dirty", &[]);
}

#[test]
fn review_changes_of_a_clean_tree_as_json() {
    parity("review-changes", "review-changes-clean-json", &["--json"]);
}

#[test]
fn review_changes_honours_a_byte_cap() {
    parity_dirty(
        "review-changes",
        "review-changes-byte-cap-json",
        &["--byte-cap", "120", "--json"],
    );
}

// --- commit-history --------------------------------------------------------

#[test]
fn commit_history_is_compact_by_default() {
    parity("commit-history", "commit-history", &[]);
}

#[test]
fn commit_history_as_json() {
    parity("commit-history", "commit-history-json", &["--json"]);
}

#[test]
fn commit_history_full_detail() {
    parity(
        "commit-history",
        "commit-history-full-json",
        &["--detail", "full", "--json"],
    );
}

#[test]
fn commit_history_of_another_ref_with_a_limit() {
    parity(
        "commit-history",
        "commit-history-ref-limit-json",
        &["--ref", "main", "--limit", "1", "--json"],
    );
}

#[test]
fn commit_history_pages_with_a_cursor() {
    parity(
        "commit-history",
        "commit-history-cursor-json",
        &["--limit", "1", "--cursor", "1", "--json"],
    );
}

#[test]
fn commit_history_rejects_an_unknown_detail_level() {
    parity(
        "commit-history",
        "commit-history-bad-detail-json",
        &["--detail", "everything", "--json"],
    );
}

#[test]
fn commit_history_rejects_a_ref_that_looks_like_an_option() {
    parity(
        "commit-history",
        "commit-history-option-ref-json",
        &["--ref=--output=x", "--json"],
    );
}

// --- diff ------------------------------------------------------------------

#[test]
fn diff_between_two_refs() {
    parity("diff", "diff-refs", &["main", "feature"]);
}

#[test]
fn diff_between_two_refs_as_json() {
    parity("diff", "diff-refs-json", &["main", "feature", "--json"]);
}

#[test]
fn diff_restricted_to_paths() {
    parity(
        "diff",
        "diff-paths-json",
        &["main", "feature", "--paths", "src/extra.rs", "--json"],
    );
}

#[test]
fn diff_honours_a_byte_cap() {
    parity(
        "diff",
        "diff-byte-cap-json",
        &["main", "feature", "--byte-cap", "60", "--json"],
    );
}

#[test]
fn diff_to_the_working_tree() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "diff",
        "diff-working-tree-json",
        &|| Setup::new(dirty_repo(), &["HEAD", "--json"]),
    );
}

#[test]
fn diff_of_an_unknown_ref_fails() {
    parity(
        "diff",
        "diff-unknown-ref-json",
        &["nope", "feature", "--json"],
    );
}

// --- list-branches ---------------------------------------------------------

#[test]
fn list_branches_without_a_remote() {
    parity("list-branches", "list-branches-json", &["--json"]);
}

#[test]
fn list_branches_in_words() {
    parity("list-branches", "list-branches", &[]);
}

#[test]
fn list_branches_against_a_remote() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "list-branches",
        "list-branches-origin-json",
        &|| {
            let mut setup = Setup::new(git_fixture(), &["--fetch", "--stale-days", "0", "--json"]);
            let scratch = setup.scratch();
            add_origin(setup.dir.path(), &scratch);
            git(
                setup.dir.path(),
                &["commit", "-q", "--allow-empty", "-m", "unpushed"],
            );
            setup
        },
    );
}

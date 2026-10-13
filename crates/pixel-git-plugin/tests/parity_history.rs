// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The history engines (`search-history`, `file-history`, `dig-history`,
//! `sync-branch`), against the built-in ones.

use pixel_git_plugin::testing::{Setup, add_origin, assert_parity, git, git_fixture, run};

const BIN: &str = env!("CARGO_BIN_EXE_pixel-git");

fn parity(command: &str, case: &str, args: &'static [&'static str]) {
    assert_parity(env!("CARGO_MANIFEST_DIR"), BIN, command, case, &|| {
        Setup::new(git_fixture(), args)
    });
}

fn with_origin(args: &[&str]) -> Setup {
    let mut setup = Setup::new(git_fixture(), args);
    let scratch = setup.scratch();
    add_origin(setup.dir.path(), &scratch);
    setup
}

// --- search-history --------------------------------------------------------

#[test]
fn search_history_finds_a_message() {
    parity(
        "search-history",
        "search-history-message-json",
        &["gamma", "--facet", "message", "--json"],
    );
}

#[test]
fn search_history_finds_a_path() {
    parity(
        "search-history",
        "search-history-path-json",
        &["extra", "--facet", "path", "--json"],
    );
}

#[test]
fn search_history_finds_diff_text() {
    parity(
        "search-history",
        "search-history-diff-json",
        &["alpha", "--facet", "diff", "--limit", "5", "--json"],
    );
}

#[test]
fn search_history_searches_every_facet_by_default() {
    parity(
        "search-history",
        "search-history-all-json",
        &["alpha", "--json"],
    );
}

#[test]
fn search_history_in_words() {
    parity("search-history", "search-history", &["alpha"]);
}

// --- file-history ----------------------------------------------------------

#[test]
fn file_history_of_a_path() {
    parity(
        "file-history",
        "file-history-path-json",
        &["--file", "src/lib.rs", "--json"],
    );
}

#[test]
fn file_history_of_a_token() {
    parity(
        "file-history",
        "file-history-token-json",
        &["--token", "gamma", "--json"],
    );
}

#[test]
fn file_history_in_words() {
    parity("file-history", "file-history", &["--file", "src/extra.rs"]);
}

#[test]
fn file_history_needs_a_path_or_a_token() {
    parity("file-history", "file-history-none-json", &["--json"]);
}

// --- dig-history -----------------------------------------------------------

#[test]
fn dig_history_finds_a_phrase() {
    parity(
        "dig-history",
        "dig-history-phrase-json",
        &["--phrase", "gamma", "--json"],
    );
}

#[test]
fn dig_history_restricted_to_a_file_and_range() {
    parity(
        "dig-history",
        "dig-history-file-range-json",
        &[
            "--phrase",
            "alpha",
            "--file",
            "src/lib.rs",
            "--from",
            "main",
            "--to",
            "feature",
            "--limit",
            "3",
            "--json",
        ],
    );
}

#[test]
fn dig_history_without_a_phrase_lists_the_ingest_state() {
    parity("dig-history", "dig-history-state-json", &["--json"]);
}

/// A `--show` setup: the first commit's id differs per fixture.
fn show_case(case: &str, extra: &'static [&'static str]) {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "dig-history",
        case,
        &|| {
            let repo = git_fixture();
            let first = git(repo.path(), &["rev-list", "--max-parents=0", "HEAD"]);
            let mut args = vec!["--show", first.as_str(), "--file", "src/lib.rs"];
            args.extend_from_slice(extra);
            Setup::new(repo, &args)
        },
    );
}

#[test]
fn dig_history_shows_a_file_as_it_was() {
    show_case("dig-history-show", &[]);
}

#[test]
fn dig_history_shows_a_file_as_json() {
    show_case("dig-history-show-json", &["--json"]);
}

#[test]
fn dig_history_shows_the_parent_version() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "dig-history",
        "dig-history-show-parent",
        &|| {
            let repo = git_fixture();
            let head = git(repo.path(), &["rev-parse", "HEAD~1"]);
            Setup::new(
                repo,
                &["--show", head.as_str(), "--file", "src/lib.rs", "--parent"],
            )
        },
    );
}

#[test]
fn dig_history_shows_the_content_before_a_deletion() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "dig-history",
        "dig-history-show-deleted-json",
        &|| {
            let repo = git_fixture();
            git(repo.path(), &["rm", "-q", "src/extra.rs"]);
            git(repo.path(), &["commit", "-qm", "drop extra"]);
            let head = git(repo.path(), &["rev-parse", "HEAD"]);
            Setup::new(
                repo,
                &["--show", head.as_str(), "--file", "src/extra.rs", "--json"],
            )
        },
    );
}

#[test]
fn dig_history_show_needs_a_file() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "dig-history",
        "dig-history-show-no-file-json",
        &|| {
            let repo = git_fixture();
            let head = git(repo.path(), &["rev-parse", "HEAD"]);
            Setup::new(repo, &["--show", head.as_str(), "--json"])
        },
    );
}

#[test]
fn dig_history_show_of_a_missing_file_fails() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "dig-history",
        "dig-history-show-missing-json",
        &|| {
            let repo = git_fixture();
            let head = git(repo.path(), &["rev-parse", "HEAD"]);
            Setup::new(
                repo,
                &["--show", head.as_str(), "--file", "nope.rs", "--json"],
            )
        },
    );
}

// --- sync-branch -----------------------------------------------------------

#[test]
fn sync_branch_reports_the_state_against_the_remote() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "sync-branch",
        "sync-branch-json",
        &|| with_origin(&["--json"]),
    );
}

#[test]
fn sync_branch_in_words() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "sync-branch",
        "sync-branch",
        &|| with_origin(&[]),
    );
}

#[test]
fn sync_branch_without_a_remote() {
    parity("sync-branch", "sync-branch-no-remote-json", &["--json"]);
}

/// `origin/feature` is one commit ahead of the local `feature`, pushed from
/// a second clone.
fn behind_origin(args: &[&str]) -> Setup {
    let mut setup = with_origin(args);
    let other = setup.scratch().join("other");
    let url = git(setup.dir.path(), &["remote", "get-url", "origin"]);
    git(
        setup.dir.path(),
        &[
            "clone",
            "-q",
            "-b",
            "feature",
            url.as_str(),
            other.to_str().unwrap(),
        ],
    );
    git(&other, &["config", "user.name", "Other"]);
    git(&other, &["config", "user.email", "other@example.com"]);
    git(
        &other,
        &["commit", "-q", "--allow-empty", "-m", "remote work"],
    );
    git(&other, &["push", "-q", "origin", "feature"]);
    setup
}

#[test]
fn sync_branch_brings_a_clean_branch_up_to_its_upstream() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "sync-branch",
        "sync-branch-rebase-json",
        &|| {
            behind_origin(&[
                "--strategy",
                "rebase-if-clean",
                "--push",
                "none",
                "--request-id",
                "r1",
                "--json",
            ])
        },
    );
}

#[test]
fn sync_branch_only_reports_when_behind_in_report_mode() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "sync-branch",
        "sync-branch-behind-json",
        &|| behind_origin(&["--json"]),
    );
}

#[test]
fn sync_branch_really_leaves_the_tree_alone_in_report_mode() {
    let setup = with_origin(&[]);
    let head = git(setup.dir.path(), &["rev-parse", "HEAD"]);
    let seen = run(
        BIN.as_ref(),
        &["sync-branch", "--json"],
        setup.dir.path(),
        &[],
    );
    assert_eq!(seen.code, 0, "{}", seen.stderr);
    assert_eq!(git(setup.dir.path(), &["rev-parse", "HEAD"]), head);
}

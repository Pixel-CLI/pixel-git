// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The commands that change a repository, against the built-in ones.

use pixel_git_plugin::testing::{Setup, add_origin, assert_parity, git, git_fixture, run, write};

const BIN: &str = env!("CARGO_BIN_EXE_pixel-git");

fn parity(command: &str, case: &str, make: &dyn Fn() -> Setup) {
    assert_parity(env!("CARGO_MANIFEST_DIR"), BIN, command, case, make);
}

/// `feature` checked out with an untracked `notes.txt` to commit.
fn dirty(args: &[&str]) -> Setup {
    let repo = git_fixture();
    write(repo.path(), "notes.txt", "hello\n");
    Setup::new(repo, args)
}

/// `dirty` with an `origin` that already holds `main` and `feature`.
fn dirty_with_origin(args: &[&str]) -> Setup {
    let mut setup = dirty(args);
    let scratch = setup.scratch();
    add_origin(setup.dir.path(), &scratch);
    setup
}

fn with_origin(args: &[&str]) -> Setup {
    let mut setup = Setup::new(git_fixture(), args);
    let scratch = setup.scratch();
    add_origin(setup.dir.path(), &scratch);
    setup
}

// --- commit ----------------------------------------------------------------

#[test]
fn commit_stages_the_named_file_and_reports_json() {
    parity("commit", "commit-json", &|| {
        dirty(&[
            "-m",
            "add notes",
            "--files",
            "notes.txt",
            "--request-id",
            "c1",
            "--json",
        ])
    });
}

#[test]
fn commit_reports_in_words() {
    parity("commit", "commit", &|| {
        dirty(&[
            "-m",
            "add notes",
            "--files",
            "notes.txt",
            "--request-id",
            "c2",
        ])
    });
}

#[test]
fn commit_reads_the_message_from_a_file() {
    parity("commit", "commit-message-file-json", &|| {
        let mut setup = dirty(&[]);
        let scratch = setup.scratch();
        let file = scratch.join("message.txt");
        std::fs::write(&file, "subject\n\nbody line\n\n").unwrap();
        setup.args = [
            "-F",
            file.to_str().unwrap(),
            "--files",
            "notes.txt",
            "--request-id",
            "c3",
            "--json",
        ]
        .map(String::from)
        .to_vec();
        setup
    });
}

#[test]
fn commit_amends_the_current_commit() {
    parity("commit", "commit-amend-json", &|| {
        dirty(&[
            "-m",
            "amended",
            "--files",
            "notes.txt",
            "--amend",
            "--request-id",
            "c4",
            "--json",
        ])
    });
}

#[test]
fn commit_with_nothing_to_commit_is_refused() {
    parity("commit", "commit-nothing-json", &|| {
        Setup::new(
            git_fixture(),
            &["-m", "nothing", "--request-id", "c5", "--json"],
        )
    });
}

#[test]
fn commit_refuses_a_stale_expected_head() {
    parity("commit", "commit-stale-head-json", &|| {
        dirty(&[
            "-m",
            "x",
            "--files",
            "notes.txt",
            "--expected-head",
            "0000000000000000000000000000000000000000",
            "--request-id",
            "c6",
            "--json",
        ])
    });
}

#[test]
fn commit_needs_a_message() {
    parity("commit", "commit-no-message", &|| {
        dirty(&["--files", "notes.txt", "--request-id", "c7"])
    });
}

#[test]
fn commit_without_a_request_id_gets_the_retry_tip() {
    parity("commit", "commit-missing-request-id", &|| {
        dirty(&["-m", "x", "--files", "notes.txt"])
    });
}

#[test]
fn commit_really_commits_only_the_named_file() {
    let repo = git_fixture();
    write(repo.path(), "notes.txt", "hello\n");
    write(repo.path(), "other.txt", "other\n");
    let seen = run(
        BIN.as_ref(),
        &[
            "commit",
            "-m",
            "add notes",
            "--files",
            "notes.txt",
            "--request-id",
            "c8",
        ],
        repo.path(),
        &[],
    );
    assert_eq!(seen.code, 0, "{}", seen.stderr);
    assert_eq!(
        git(repo.path(), &["show", "--name-only", "--format=%s", "HEAD"]),
        "add notes\n\nnotes.txt"
    );
    assert_eq!(git(repo.path(), &["status", "--porcelain"]), "?? other.txt");
}

#[test]
fn commit_replays_a_retried_request_instead_of_running_twice() {
    let repo = git_fixture();
    write(repo.path(), "notes.txt", "hello\n");
    let args = [
        "commit",
        "-m",
        "add notes",
        "--files",
        "notes.txt",
        "--request-id",
        "same",
        "--json",
    ];
    let first = run(BIN.as_ref(), &args, repo.path(), &[]);
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let second = run(BIN.as_ref(), &args, repo.path(), &[]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(second.code, 0, "{}", second.stderr);
    assert_eq!(git(repo.path(), &["rev-parse", "HEAD"]), head);
    assert_eq!(git(repo.path(), &["rev-list", "--count", "HEAD"]), "4");
}

// --- push ------------------------------------------------------------------

#[test]
fn push_sends_the_branch_to_the_remote() {
    parity("push", "push-json", &|| {
        let setup = with_origin(&["origin", "feature", "--request-id", "p1", "--json"]);
        write(setup.dir.path(), "n.txt", "n\n");
        git(setup.dir.path(), &["add", "n.txt"]);
        git(setup.dir.path(), &["commit", "-qm", "local only"]);
        setup
    });
}

#[test]
fn push_reports_in_words() {
    parity("push", "push", &|| {
        with_origin(&["origin", "feature", "--request-id", "p2"])
    });
}

#[test]
fn push_to_an_unknown_remote_fails() {
    parity("push", "push-unknown-remote-json", &|| {
        Setup::new(
            git_fixture(),
            &["nowhere", "feature", "--request-id", "p3", "--json"],
        )
    });
}

#[test]
fn push_force_with_lease_rewrites_the_remote_branch() {
    parity("push", "push-force-json", &|| {
        let setup = with_origin(&[
            "origin",
            "feature",
            "--force-with-lease",
            "--request-id",
            "p4",
            "--json",
        ]);
        git(
            setup.dir.path(),
            &["commit", "-q", "--amend", "-m", "rewritten"],
        );
        setup
    });
}

// --- commit-and-push -------------------------------------------------------

#[test]
fn commit_and_push_commits_then_pushes() {
    parity("commit-and-push", "commit-and-push-json", &|| {
        dirty_with_origin(&[
            "-m",
            "ship notes",
            "--files",
            "notes.txt",
            "origin",
            "feature",
            "--request-id",
            "s1",
            "--json",
        ])
    });
}

#[test]
fn commit_and_push_needs_a_remote_and_a_refspec() {
    parity("commit-and-push", "commit-and-push-usage", &|| {
        dirty(&["-m", "x", "--request-id", "s2"])
    });
}

// --- new-branch ------------------------------------------------------------

#[test]
fn new_branch_creates_it_from_head() {
    parity("new-branch", "new-branch-json", &|| {
        Setup::new(git_fixture(), &["topic", "--request-id", "b1", "--json"])
    });
}

#[test]
fn new_branch_reports_in_words_from_a_ref() {
    parity("new-branch", "new-branch-from", &|| {
        Setup::new(
            git_fixture(),
            &["topic", "--from", "main", "--request-id", "b2"],
        )
    });
}

#[test]
fn new_branch_refuses_an_existing_name() {
    parity("new-branch", "new-branch-exists-json", &|| {
        Setup::new(git_fixture(), &["main", "--request-id", "b3", "--json"])
    });
}

#[test]
fn new_branch_needs_a_request_id() {
    parity("new-branch", "new-branch-missing-request-id", &|| {
        Setup::new(git_fixture(), &["topic"])
    });
}

// --- fetch -----------------------------------------------------------------

#[test]
fn fetch_updates_from_the_remote() {
    parity("fetch", "fetch-json", &|| {
        with_origin(&["origin", "--json"])
    });
}

#[test]
fn fetch_reports_in_words_with_a_refspec() {
    parity("fetch", "fetch-refspec", &|| {
        with_origin(&[
            "origin",
            "--refspec",
            "refs/heads/main:refs/remotes/origin/main",
        ])
    });
}

#[test]
fn fetch_from_an_unknown_remote_fails() {
    parity("fetch", "fetch-unknown-json", &|| {
        Setup::new(git_fixture(), &["nowhere", "--json"])
    });
}

// --- fast-forward ----------------------------------------------------------

/// `main` checked out, one commit behind `feature`: the fast-forward the
/// command exists for.
fn behind(extra: &[&str], expected_is_head: bool) -> Setup {
    let repo = git_fixture();
    let main = git(repo.path(), &["rev-parse", "main"]);
    let feature = git(repo.path(), &["rev-parse", "feature"]);
    git(repo.path(), &["checkout", "-q", "main"]);
    let expected = if expected_is_head { &main } else { &feature };
    let mut args = vec![
        "--expected-head",
        expected.as_str(),
        "--target-oid",
        feature.as_str(),
        "--request-id",
        "req-1",
    ];
    args.extend_from_slice(extra);
    Setup::new(repo, &args)
}

#[test]
fn fast_forward_moves_the_checked_out_branch() {
    parity("fast-forward", "fast-forward-advances", &|| {
        behind(&[], true)
    });
}

#[test]
fn fast_forward_reports_the_move_as_json() {
    parity("fast-forward", "fast-forward-advances-json", &|| {
        behind(&["--json"], true)
    });
}

#[test]
fn fast_forward_refuses_a_wrong_expected_head_with_a_failure_envelope() {
    parity(
        "fast-forward",
        "fast-forward-stale-expected-head-json",
        &|| behind(&["--json"], false),
    );
}

#[test]
fn fast_forward_refuses_a_wrong_expected_head() {
    parity("fast-forward", "fast-forward-stale-expected-head", &|| {
        behind(&[], false)
    });
}

#[test]
fn fast_forward_without_a_request_id_gets_the_retry_tip() {
    parity("fast-forward", "fast-forward-missing-request-id", &|| {
        Setup::new(
            git_fixture(),
            &["--expected-head", "a", "--target-oid", "b"],
        )
    });
}

#[test]
fn fast_forward_really_moves_the_branch() {
    let setup = behind(&[], true);
    let feature = git(setup.dir.path(), &["rev-parse", "feature"]);
    let mut args = vec!["fast-forward"];
    args.extend(setup.args.iter().map(String::as_str));
    let seen = run(BIN.as_ref(), &args, setup.dir.path(), &[]);
    assert_eq!(seen.code, 0, "{}", seen.stderr);
    assert_eq!(git(setup.dir.path(), &["rev-parse", "HEAD"]), feature);
}

// --- squash-branch ---------------------------------------------------------

#[test]
fn squash_branch_squashes_onto_main() {
    parity("squash-branch", "squash-branch-squashes", &|| {
        Setup::new(
            git_fixture(),
            &["--onto", "main", "-m", "squashed", "--request-id", "req-1"],
        )
    });
}

#[test]
fn squash_branch_reports_the_squash_as_json() {
    parity("squash-branch", "squash-branch-squashes-json", &|| {
        Setup::new(
            git_fixture(),
            &[
                "--onto",
                "main",
                "-m",
                "squashed",
                "--request-id",
                "req-2",
                "--json",
            ],
        )
    });
}

#[test]
fn squash_branch_refuses_an_unknown_base() {
    parity("squash-branch", "squash-branch-unknown-base-json", &|| {
        Setup::new(
            git_fixture(),
            &["--onto", "nope", "--request-id", "req-3", "--json"],
        )
    });
}

#[test]
fn squash_branch_without_a_request_id_gets_the_retry_tip() {
    parity("squash-branch", "squash-branch-missing-request-id", &|| {
        Setup::new(git_fixture(), &["--onto", "main"])
    });
}

#[test]
fn squash_branch_leaves_one_commit_and_a_backup_ref() {
    let repo = git_fixture();
    let old_head = git(repo.path(), &["rev-parse", "HEAD"]);
    let seen = run(
        BIN.as_ref(),
        &[
            "squash-branch",
            "--onto",
            "main",
            "-m",
            "squashed",
            "--request-id",
            "req-4",
        ],
        repo.path(),
        &[],
    );
    assert_eq!(seen.code, 0, "{}", seen.stderr);
    assert_eq!(
        git(repo.path(), &["rev-list", "--count", "main..HEAD"]),
        "1"
    );
    assert_eq!(
        git(
            repo.path(),
            &["rev-parse", "refs/pixel/rewrite-backup/feature"]
        ),
        old_head
    );
}

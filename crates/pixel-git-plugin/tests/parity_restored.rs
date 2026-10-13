// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! `who-wrote` and `edit-env`, restored from before core deleted them
//! (#898); the goldens come from a core built at `87e17b41^`, the last commit
//! that has them.

use pixel_git_plugin::testing::{Setup, assert_parity, git, git_fixture, run, write};

const BIN: &str = env!("CARGO_BIN_EXE_pixel-git");

fn parity(command: &str, case: &str, args: &'static [&'static str]) {
    assert_parity(env!("CARGO_MANIFEST_DIR"), BIN, command, case, &|| {
        Setup::new(git_fixture(), args)
    });
}

// --- who-wrote -------------------------------------------------------------

#[test]
fn who_wrote_attributes_each_region_as_json() {
    parity("who-wrote", "who-wrote-json", &["src/lib.rs", "--json"]);
}

#[test]
fn who_wrote_in_words() {
    parity("who-wrote", "who-wrote", &["src/lib.rs"]);
}

#[test]
fn who_wrote_restricted_to_lines() {
    parity(
        "who-wrote",
        "who-wrote-lines-json",
        &["src/lib.rs", "--lines", "2,2", "--json"],
    );
}

#[test]
fn who_wrote_answers_whether_an_author_touched_it() {
    parity(
        "who-wrote",
        "who-wrote-author-json",
        &["src/lib.rs", "--author", "plugin", "--json"],
    );
}

#[test]
fn who_wrote_answers_no_for_an_author_who_did_not() {
    parity(
        "who-wrote",
        "who-wrote-other-author-json",
        &["src/lib.rs", "--author", "nobody", "--json"],
    );
}

#[test]
fn who_wrote_caps_the_regions() {
    parity(
        "who-wrote",
        "who-wrote-limit-json",
        &["src/lib.rs", "--limit-regions", "1", "--json"],
    );
}

#[test]
fn who_wrote_rejects_a_reversed_line_range() {
    parity(
        "who-wrote",
        "who-wrote-bad-range",
        &["src/lib.rs", "--lines", "5,2"],
    );
}

#[test]
fn who_wrote_rejects_a_range_without_a_comma() {
    parity(
        "who-wrote",
        "who-wrote-range-shape",
        &["src/lib.rs", "--lines", "5"],
    );
}

#[test]
fn who_wrote_refuses_a_missing_file() {
    parity(
        "who-wrote",
        "who-wrote-missing-json",
        &["nope.rs", "--json"],
    );
}

// --- edit-env --------------------------------------------------------------

const ENV: &str =
    "# database\nDB_URL=postgres://localhost\n\nexport STRIPE_KEY=\"sk_live_abc\"\nEMPTY=\n";
const SECRET: &str = "hunter2-SECRET-value";

fn env_repo() -> tempfile::TempDir {
    let repo = git_fixture();
    write(repo.path(), ".env", ENV);
    write(repo.path(), "apps/web/.env.local", "WEB_KEY=1\n");
    repo
}

fn parity_env(case: &str, args: &'static [&'static str]) {
    assert_parity(env!("CARGO_MANIFEST_DIR"), BIN, "edit-env", case, &|| {
        Setup::new(env_repo(), args)
    });
}

#[test]
fn edit_env_inventories_key_names_only() {
    parity_env("edit-env-inventory-json", &["inventory", "--json"]);
}

#[test]
fn edit_env_inventory_in_words() {
    parity_env("edit-env-inventory", &["inventory"]);
}

#[test]
fn edit_env_checks_required_keys() {
    parity_env(
        "edit-env-check-json",
        &[
            "check",
            "--file",
            ".env",
            "--require",
            "DB_URL",
            "--require",
            "MISSING",
            "--json",
        ],
    );
}

#[test]
fn edit_env_sets_a_key_without_printing_its_value() {
    parity_env(
        "edit-env-set-json",
        &[
            "set", "--file", ".env", "--key", "NEW_KEY", "--value", SECRET, "--json",
        ],
    );
}

#[test]
fn edit_env_creates_a_file_when_asked() {
    parity_env(
        "edit-env-set-create-json",
        &[
            "set",
            "--file",
            ".env.test",
            "--key",
            "A",
            "--value",
            SECRET,
            "--create-file",
            "--json",
        ],
    );
}

#[test]
fn edit_env_refuses_a_missing_file_without_create_file() {
    parity_env(
        "edit-env-set-missing-json",
        &[
            "set",
            "--file",
            ".env.nope",
            "--key",
            "A",
            "--value",
            SECRET,
            "--json",
        ],
    );
}

#[test]
fn edit_env_refuses_a_file_outside_the_repository() {
    parity_env(
        "edit-env-set-outside-json",
        &[
            "set",
            "--file",
            "/etc/hosts",
            "--key",
            "A",
            "--value",
            SECRET,
            "--json",
        ],
    );
}

#[test]
fn edit_env_lists_snapshots_after_a_change() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "edit-env",
        "edit-env-snapshots-json",
        &|| {
            Setup::new(env_repo(), &["snapshots", "--file", ".env", "--json"])
                .then(&["set", "--file", ".env", "--key", "K", "--value", SECRET])
        },
    );
}

#[test]
fn edit_env_restores_the_file() {
    assert_parity(
        env!("CARGO_MANIFEST_DIR"),
        BIN,
        "edit-env",
        "edit-env-restore-json",
        &|| {
            Setup::new(env_repo(), &["restore", "--file", ".env", "--json"])
                .then(&["set", "--file", ".env", "--key", "K", "--value", SECRET])
        },
    );
}

#[test]
fn edit_env_rejects_a_snapshot_id_that_climbs() {
    parity_env(
        "edit-env-restore-bad-snapshot-json",
        &["restore", "--file", ".env", "--snapshot", "../x", "--json"],
    );
}

#[test]
fn edit_env_needs_a_subcommand() {
    // Not a golden: core lists its `--metrics` option in this usage, the
    // plugin hides it (see `HostFlags`).
    let repo = env_repo();
    let seen = run(BIN.as_ref(), &["edit-env"], repo.path(), &[]);
    assert_eq!(seen.code, 2);
    assert!(
        seen.stderr.contains("Usage: pixel edit-env"),
        "{}",
        seen.stderr
    );
    for sub in ["inventory", "set", "restore", "snapshots", "check"] {
        assert!(seen.stderr.contains(sub), "{sub}: {}", seen.stderr);
    }
}

#[test]
fn edit_env_keeps_every_other_line_and_never_prints_the_value() {
    let repo = env_repo();
    let seen = run(
        BIN.as_ref(),
        &[
            "edit-env",
            "set",
            "--file",
            ".env",
            "--key",
            "STRIPE_KEY",
            "--value",
            SECRET,
            "--json",
        ],
        repo.path(),
        &[],
    );
    assert_eq!(seen.code, 0, "{}", seen.stderr);
    assert!(!seen.stdout.contains(SECRET) && !seen.stderr.contains(SECRET));
    let after = std::fs::read_to_string(repo.path().join(".env")).unwrap();
    assert!(
        after.contains(SECRET),
        "the value reaches the file: {after}"
    );
    for line in ["# database", "DB_URL=postgres://localhost", "EMPTY="] {
        assert!(after.contains(line), "{line} survives: {after}");
    }
    assert_eq!(
        git(repo.path(), &["status", "--porcelain", "--", "src"]),
        ""
    );
}

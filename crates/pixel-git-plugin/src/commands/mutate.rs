// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The commands that change a repository through `pixel-ops`: `commit`,
//! `push`, `commit-and-push`, `new-branch`, `fetch`, `fast-forward` and
//! `squash-branch`.
//!
//! The command lines and handlers are ported from `crates/pixel/src/main.rs`
//! of Pixel-CLI/pixel (MIT), where they were private to the `pixel` binary.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

use crate::env::resolve_path;
use crate::output::print_data;
use crate::root::discover_root;
use crate::run::{HostFlags, run};

/// The commit message of `commit` / `commit-and-push`: `-m <text>`, or the
/// content of `--message-file <path>` (`-` reads stdin). Clap guarantees
/// exactly one of the two is present.
fn commit_message(inline: Option<String>, file: Option<&Path>) -> Result<String, String> {
    let raw = match (inline, file) {
        (Some(text), _) => text,
        (None, Some(path)) if path == Path::new("-") => {
            let mut text = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)
                .map_err(|e| format!("cannot read commit message from stdin: {e}"))?;
            text
        }
        (None, Some(path)) => std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read commit message file {}: {e}", path.display()))?,
        (None, None) => return Err("a commit message is required (-m or --message-file)".into()),
    };
    normalize_commit_message(&raw)
}

/// Trailing whitespace goes (an editor's final newline would otherwise
/// become a blank trailer line); a blank message is refused before git
/// sees it, so no journal entry is written for a commit git would reject.
fn normalize_commit_message(raw: &str) -> Result<String, String> {
    let text = raw.trim_end();
    if text.trim().is_empty() {
        return Err("commit message is empty".into());
    }
    Ok(text.to_string())
}

/// Stage files, commit, and optionally push (crash-safe, idempotent).
#[derive(Parser)]
#[command(name = "commit", bin_name = "pixel commit")]
struct CommitCli {
    #[command(flatten)]
    _host: HostFlags,
    /// Commit message. Use `--message-file` for a multi-paragraph body.
    #[arg(
        short = 'm',
        long = "message",
        required_unless_present = "message_file"
    )]
    message: Option<String>,
    /// Read the commit message from this file (`-` for stdin), verbatim
    /// except for trailing whitespace. Mutually exclusive with `-m`.
    #[arg(short = 'F', long = "message-file", conflicts_with = "message")]
    message_file: Option<PathBuf>,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Files to stage (repo-relative). Repeat the flag once per file
    /// (`--files a --files b`) — a single `--files a b` does NOT work:
    /// `b` silently becomes the trailing PATH argument instead of a
    /// second file, or errors if a PATH was already given.
    #[arg(long = "files")]
    files: Vec<String>,
    /// Also push after committing.
    #[arg(long)]
    push: bool,
    /// Amend the current commit instead of creating a new one.
    #[arg(long)]
    amend: bool,
    /// Reject if HEAD does not match this OID.
    #[arg(long)]
    expected_head: Option<String>,
    /// Idempotency / recovery key.
    #[arg(long)]
    request_id: String,
    #[arg(long)]
    json: bool,
}

pub fn commit(args: Vec<OsString>) -> ExitCode {
    run::<CommitCli>("commit", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let message = commit_message(cli.message, cli.message_file.as_deref())?;
        let opts = pixel_ops::publish::PublishOptions {
            message,
            files: cli.files,
            expected_head: cli.expected_head,
            expected_fingerprints: std::collections::BTreeMap::new(),
            push: cli.push,
            amend: cli.amend,
            request_id: cli.request_id,
        };
        let data = pixel_ops::publish::publish(&root, &opts, None)?;
        print_data(&data, cli.json)
    })
}

/// Leased push to a remote (crash-safe, idempotent).
#[derive(Parser)]
#[command(name = "push", bin_name = "pixel push")]
struct PushCli {
    #[command(flatten)]
    _host: HostFlags,
    remote: String,
    refspec: String,
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long)]
    force_with_lease: bool,
    /// Idempotency / recovery key.
    #[arg(long)]
    request_id: String,
    #[arg(long)]
    json: bool,
}

pub fn push(args: Vec<OsString>) -> ExitCode {
    run::<PushCli>("push", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let opts = pixel_ops::push::PushOptions {
            remote: cli.remote,
            refspec: cli.refspec,
            request_id: cli.request_id,
            force_with_lease: cli.force_with_lease,
        };
        let data = pixel_ops::push::push(&root, &opts, None)?;
        print_data(&data, cli.json)
    })
}

/// Publish + push in one op (commit then leased push).
#[derive(Parser)]
#[command(name = "commit-and-push", bin_name = "pixel commit-and-push")]
struct CommitAndPushCli {
    #[command(flatten)]
    _host: HostFlags,
    /// Commit message. Use `--message-file` for a multi-paragraph body.
    #[arg(
        short = 'm',
        long = "message",
        required_unless_present = "message_file"
    )]
    message: Option<String>,
    /// Read the commit message from this file (`-` for stdin), verbatim
    /// except for trailing whitespace. Mutually exclusive with `-m`.
    #[arg(short = 'F', long = "message-file", conflicts_with = "message")]
    message_file: Option<PathBuf>,
    // Positional order matches Push: required remote + refspec first, then the
    // defaulted path. (A defaulted positional BEFORE required ones trips clap's
    // debug assertions.)
    remote: String,
    refspec: String,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Files to stage (repo-relative). Repeat the flag once per file
    /// (`--files a --files b`) — a single `--files a b` does NOT work:
    /// `b` silently becomes the trailing PATH argument instead of a
    /// second file, or errors if a PATH was already given.
    #[arg(long = "files")]
    files: Vec<String>,
    /// Leased force-push (`--force-with-lease`) for the push phase.
    #[arg(long)]
    force_with_lease: bool,
    /// Idempotency / recovery key.
    #[arg(long)]
    request_id: String,
    #[arg(long)]
    json: bool,
}

pub fn commit_and_push(args: Vec<OsString>) -> ExitCode {
    run::<CommitAndPushCli>("commit-and-push", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let message = commit_message(cli.message, cli.message_file.as_deref())?;
        let data = pixel_ops::ship::ship_with_lease(
            &root,
            &message,
            &cli.files,
            &cli.remote,
            &cli.refspec,
            &cli.request_id,
            cli.force_with_lease,
        )?;
        print_data(&data, cli.json)
    })
}

/// Create a new branch from HEAD (or --from <ref>).
#[derive(Parser)]
#[command(name = "new-branch", bin_name = "pixel new-branch")]
struct NewBranchCli {
    #[command(flatten)]
    _host: HostFlags,
    name: String,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Base ref (default HEAD).
    #[arg(long)]
    from: Option<String>,
    /// Idempotency / recovery key.
    #[arg(long)]
    request_id: String,
    #[arg(long)]
    json: bool,
}

pub fn new_branch(args: Vec<OsString>) -> ExitCode {
    run::<NewBranchCli>("new-branch", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let opts = pixel_ops::branch::BranchOptions {
            name: cli.name,
            from: cli.from,
            request_id: cli.request_id,
        };
        let data = pixel_ops::branch::branch(&root, &opts)?;
        print_data(&data, cli.json)
    })
}

/// Fetch from a remote (idempotent).
#[derive(Parser)]
#[command(name = "fetch", bin_name = "pixel fetch")]
struct FetchCli {
    #[command(flatten)]
    _host: HostFlags,
    remote: String,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Optional refspec.
    #[arg(long)]
    refspec: Option<String>,
    #[arg(long)]
    json: bool,
}

pub fn fetch(args: Vec<OsString>) -> ExitCode {
    run::<FetchCli>("fetch", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let data = pixel_ops::sync::sync(&root, &cli.remote, cli.refspec.as_deref())?;
        print_data(&data, cli.json)
    })
}

/// Fast-forward merge to a target OID (refuses non-ff + dirty intersection).
#[derive(Parser)]
#[command(name = "fast-forward", bin_name = "pixel fast-forward")]
struct FastForwardCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Reject if HEAD does not match this OID.
    #[arg(long)]
    expected_head: String,
    /// Fast-forward target OID.
    #[arg(long)]
    target_oid: String,
    /// Idempotency / recovery key.
    #[arg(long)]
    request_id: String,
    #[arg(long)]
    json: bool,
}

pub fn fast_forward(args: Vec<OsString>) -> ExitCode {
    run::<FastForwardCli>("fast-forward", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let opts = pixel_ops::update::UpdateOptions {
            expected_head: cli.expected_head,
            target_oid: cli.target_oid,
            request_id: cli.request_id,
        };
        let data = pixel_ops::update::update(&root, &opts)?;
        print_data(&data, cli.json)
    })
}

/// Squash every commit on the current branch since its base into ONE
/// commit (crash-safe, backup-ref'd), optionally force-pushing with lease.
#[derive(Parser)]
#[command(name = "squash-branch", bin_name = "pixel squash-branch")]
struct SquashBranchCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Explicit base ref (squash <onto>..HEAD). Default: merge-base with
    /// the branch upstream, else with the remote default branch.
    #[arg(long)]
    onto: Option<String>,
    /// Squash commit message (default: auto-generated subject list).
    #[arg(short = 'm', long = "message")]
    message: Option<String>,
    /// Push the rewritten branch with --force-with-lease afterwards.
    #[arg(long)]
    push: bool,
    /// Remote name.
    #[arg(long, default_value = "origin")]
    remote: String,
    /// Reject if HEAD does not match this OID.
    #[arg(long)]
    expected_head: Option<String>,
    /// Allow rewriting the default branch and published mainline commits.
    /// Overrides both default-branch and published-mainline protection.
    #[arg(long)]
    allow_default_branch: bool,
    /// Idempotency / recovery key.
    #[arg(long)]
    request_id: String,
    #[arg(long)]
    json: bool,
}

pub fn squash_branch(args: Vec<OsString>) -> ExitCode {
    run::<SquashBranchCli>("squash-branch", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let opts = pixel_ops::rewrite::RewriteOptions {
            onto: cli.onto,
            message: cli.message,
            push: cli.push,
            remote: cli.remote,
            request_id: cli.request_id,
            expected_head: cli.expected_head,
            allow_default_branch: cli.allow_default_branch,
        };
        let data = pixel_ops::rewrite::rewrite(&root, &opts)?;
        print_data(&data, cli.json)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_loses_trailing_whitespace_and_a_blank_one_is_refused() {
        assert_eq!(normalize_commit_message("fix: x\n\n").unwrap(), "fix: x");
        assert_eq!(normalize_commit_message("a\n\nb \n").unwrap(), "a\n\nb");
        assert_eq!(
            normalize_commit_message(" \n\t\n").unwrap_err(),
            "commit message is empty"
        );
    }

    #[test]
    fn the_message_comes_from_the_flag_or_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("m.txt");
        std::fs::write(&file, "from file\n").unwrap();
        assert_eq!(
            commit_message(Some("inline\n".into()), Some(&file)).unwrap(),
            "inline"
        );
        assert_eq!(commit_message(None, Some(&file)).unwrap(), "from file");
        assert!(
            commit_message(None, Some(&dir.path().join("nope")))
                .unwrap_err()
                .starts_with("cannot read commit message file")
        );
        assert!(commit_message(None, None).is_err());
    }
}

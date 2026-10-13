// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The history engines: `search-history`, `file-history`, `dig-history` and
//! `sync-branch`. Each sends one request to core's `Service`, run in this
//! process (see [`crate::engine`]).
//!
//! The command lines and handlers are ported from `crates/pixel/src/main.rs`
//! of Pixel-CLI/pixel (MIT), where they were private to the `pixel` binary.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use pixel_daemon::api::Request;
use serde_json::json;

use crate::engine::execute;
use crate::env::resolve_path;
use crate::output::{print_data, write_stdout};
use crate::root::discover_root;
use crate::run::{HostFlags, run};

/// History-wide fact + diff search.
#[derive(Parser)]
#[command(name = "search-history", bin_name = "pixel search-history")]
struct SearchHistoryCli {
    #[command(flatten)]
    _host: HostFlags,
    query: String,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// message | path | diff | all
    #[arg(long, default_value = "all")]
    facet: String,
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    json: bool,
}

pub fn search_history(args: Vec<OsString>) -> ExitCode {
    run::<SearchHistoryCli>("search-history", args, |cli| {
        let data = execute(
            &resolve_path(&cli.path),
            Request::History {
                query: cli.query,
                facet: Some(cli.facet),
                limit: cli.limit,
                read_only: false,
            },
        )?;
        print_data(&data, cli.json)
    })
}

/// Lifecycle of a path or token.
#[derive(Parser)]
#[command(name = "file-history", bin_name = "pixel file-history")]
struct FileHistoryCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Repo-relative path to inspect.
    #[arg(long)]
    file: Option<String>,
    /// Token to inspect.
    #[arg(long)]
    token: Option<String>,
    #[arg(long)]
    json: bool,
}

pub fn file_history(args: Vec<OsString>) -> ExitCode {
    run::<FileHistoryCli>("file-history", args, |cli| {
        let data = execute(
            &resolve_path(&cli.path),
            Request::Lifecycle {
                path: cli.file,
                token: cli.token,
                read_only: false,
            },
        )?;
        print_data(&data, cli.json)
    })
}

/// History-wide discovery.
#[derive(Parser)]
#[command(name = "dig-history", bin_name = "pixel dig-history")]
struct DigHistoryCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Phrase to search for in diff text.
    #[arg(long)]
    phrase: Option<String>,
    /// Restrict to a repo-relative path (with --show: the path to read).
    #[arg(long)]
    file: Option<String>,
    #[arg(long)]
    from: Option<String>,
    #[arg(long)]
    to: Option<String>,
    #[arg(long)]
    limit: Option<usize>,
    /// Print the FULL content of --file at this commit (safe
    /// `git show <oid>:<path>` equivalent). When the file does not
    /// exist at <oid> but does at <oid>^ — i.e. <oid> is the deletion
    /// commit — the parent's pre-deletion content is returned and
    /// flagged. One pixel call replaces the `git show` follow-ups.
    #[arg(long, value_name = "OID")]
    show: Option<String>,
    /// With --show: read from the commit's first parent (<oid>^)
    /// directly, skipping the read at <oid> itself.
    #[arg(long)]
    parent: bool,
    #[arg(long)]
    json: bool,
}

pub fn dig_history(args: Vec<OsString>) -> ExitCode {
    run::<DigHistoryCli>("dig-history", args, |cli| {
        let path = resolve_path(&cli.path);
        if let Some(oid) = cli.show {
            return show_file(&path, &oid, cli.file.as_deref(), cli.parent, cli.json);
        }
        let data = execute(
            &path,
            Request::Excavate {
                phrase: cli.phrase,
                path: cli.file,
                from: cli.from,
                to: cli.to,
                limit: cli.limit,
                read_only: false,
            },
        )?;
        print_data(&data, cli.json)
    })
}

/// `dig-history --show <oid> --file <path>`: full historical file content in
/// ONE call — the follow-up to a candidate list that otherwise forces raw
/// `git show`/`git log` rounds. Reads `<oid>:<path>` through the safe
/// `pixel_git::GitRunner` (ref-validated, output-capped); when the file does
/// not exist at `<oid>` (e.g. `<oid>` is the deletion commit itself) it falls
/// back to `<oid>^:<path>` — the pre-deletion content — and says so.
/// `--parent` skips straight to the parent read. The content lives in the
/// object store, not the facts db, so a direct git read is exact.
fn show_file(
    path: &Path,
    oid: &str,
    file: Option<&str>,
    parent: bool,
    json: bool,
) -> Result<(), String> {
    let Some(file) = file else {
        return Err("dig-history --show requires --file <repo-relative path>".to_string());
    };
    let root = discover_root(path)?;
    let runner = pixel_git::GitRunner::new(&root);
    let (content, source, parent_fallback) = if parent {
        let c = runner
            .show_blob_string_at_parent(oid, file)
            .map_err(|e| format!("cannot read {oid}^:{file}: {e}"))?;
        (c, format!("{oid}^:{file}"), false)
    } else {
        match runner.show_blob_string(oid, file) {
            Ok(c) => (c, format!("{oid}:{file}"), false),
            Err(at_oid_err) => match runner.show_blob_string_at_parent(oid, file) {
                Ok(c) => (c, format!("{oid}^:{file}"), true),
                Err(_) => {
                    return Err(format!(
                        "{file} exists neither at {oid} nor at {oid}^: {at_oid_err}"
                    ));
                }
            },
        }
    };
    if json {
        let data = json!({
            "oid": oid,
            "file": file,
            "source": source,
            "parent_fallback": parent_fallback,
            "content": content,
        });
        print_data(&data, true)
    } else {
        if parent_fallback {
            eprintln!(
                "pixel: {file} does not exist at {oid}; showing the parent's \
                 pre-deletion content ({source})"
            );
        } else {
            eprintln!("pixel: {source}");
        }
        write_stdout(&content)
    }
}

/// One-call deterministic branch sync.
#[derive(Parser)]
#[command(name = "sync-branch", bin_name = "pixel sync-branch")]
struct SyncBranchCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// report (default) | rebase-if-clean
    #[arg(long, default_value = "report")]
    strategy: String,
    /// auto (default) | none
    #[arg(long, default_value = "auto")]
    push: String,
    /// Integrate: rebase current branch onto origin/<TARGET>, then
    /// fast-forward local <TARGET> to the rebased head (never merge).
    /// `--strategy` is ignored in this mode.
    #[arg(long, value_name = "TARGET")]
    into: Option<String>,
    /// Idempotency / recovery key.
    #[arg(long)]
    request_id: Option<String>,
    #[arg(long)]
    json: bool,
}

pub fn sync_branch(args: Vec<OsString>) -> ExitCode {
    run::<SyncBranchCli>("sync-branch", args, |cli| {
        let data = execute(
            &resolve_path(&cli.path),
            Request::Reconcile {
                strategy: Some(cli.strategy),
                push: Some(cli.push),
                into: cli.into,
                request_id: cli.request_id,
            },
        )?;
        print_data(&data, cli.json)
    })
}

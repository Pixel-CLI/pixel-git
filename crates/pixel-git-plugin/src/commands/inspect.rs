// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The read-only commands answered by `pixel-ops`: `repo-state`,
//! `review-changes`, `commit-history`, `diff` and `list-branches`
//! (`list-branches --fetch` runs `git fetch --prune` first).
//!
//! The command lines and handlers are ported from `crates/pixel/src/main.rs`
//! of Pixel-CLI/pixel (MIT), where they were private to the `pixel` binary.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use serde_json::{Value, json};

use crate::env::resolve_path;
use crate::output::print_data;
use crate::root::discover_root;
use crate::run::{HostFlags, run};

/// Show repo state: HEAD, branch, dirty files, fingerprints.
///
/// The answer is compact: the exact `dirty_count`/`clean_count` and the
/// dirty files. The tracked-clean file list (200 paths, the bulk of the
/// answer on a clean tree) needs `--include-clean`.
#[derive(Parser)]
#[command(name = "repo-state", bin_name = "pixel repo-state")]
struct RepoStateCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Restrict the snapshot to these repo-relative paths.
    #[arg(long = "files")]
    files: Vec<String>,
    /// Include the capped tracked-clean file list (default: count only).
    #[arg(long)]
    include_clean: bool,
    #[arg(long)]
    json: bool,
}

/// Drop the tracked-clean file list from a `repo-state` answer.
///
/// On a clean tree the list is the bulk of the answer (200 of 351 paths,
/// ~7 KB measured) and no consumer reads it: `repo-state` answers "HEAD +
/// branch + dirty" and `clean_count` is what says how clean the rest of the
/// tree is. `--include-clean` asks for the full, capped list.
fn compact_repo_state(data: &mut Value) {
    if let Some(obj) = data.as_object_mut() {
        obj.remove("clean");
        obj.remove("clean_list_cap");
        obj.remove("clean_list_truncated");
    }
}

/// Keep only the requested paths in the `dirty` and `clean` lists and
/// recount them.
fn restrict_to_files(data: &mut Value, files: &[String]) {
    if let Some(dirty) = data.get_mut("dirty").and_then(Value::as_array_mut) {
        dirty.retain(|d| {
            d.get("path")
                .and_then(Value::as_str)
                .is_some_and(|p| files.iter().any(|f| f == p))
        });
    }
    if let Some(clean) = data.get_mut("clean").and_then(Value::as_array_mut) {
        clean.retain(|c| c.as_str().is_some_and(|p| files.iter().any(|f| f == p)));
    }
    data["dirty_count"] = json!(
        data.get("dirty")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    );
    data["clean_count"] = json!(
        data.get("clean")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    );
}

pub fn repo_state(args: Vec<OsString>) -> ExitCode {
    run::<RepoStateCli>("repo-state", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let mut data = pixel_ops::inspect::inspect(&root)?;
        if !cli.files.is_empty() {
            restrict_to_files(&mut data, &cli.files);
        }
        if !cli.include_clean {
            compact_repo_state(&mut data);
        }
        print_data(&data, cli.json)
    })
}

/// Review working-tree changes (staged, unstaged, untracked, conflicted).
#[derive(Parser)]
#[command(name = "review-changes", bin_name = "pixel review-changes")]
struct ReviewChangesCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Pagination cursor (opaque).
    #[arg(long)]
    cursor: Option<String>,
    /// Cap output bytes.
    #[arg(long)]
    byte_cap: Option<usize>,
    #[arg(long)]
    json: bool,
}

pub fn review_changes(args: Vec<OsString>) -> ExitCode {
    run::<ReviewChangesCli>("review-changes", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let data = pixel_ops::review::review(&root, cli.cursor.as_deref(), cli.byte_cap)?;
        print_data(&data, cli.json)
    })
}

/// Commit history with detail levels and byte caps.
#[derive(Parser)]
#[command(name = "commit-history", bin_name = "pixel commit-history")]
struct CommitHistoryCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Ref to log (default HEAD).
    #[arg(long = "ref")]
    ref_: Option<String>,
    /// Max commits (capped at 100).
    #[arg(long)]
    limit: Option<usize>,
    /// compact (oid+subject) or full (oid+author+date+subject+body).
    #[arg(long, default_value = "compact")]
    detail: String,
    /// Pagination cursor (skip N commits).
    #[arg(long)]
    cursor: Option<String>,
    /// Cap output bytes.
    #[arg(long)]
    byte_cap: Option<usize>,
    #[arg(long)]
    json: bool,
}

pub fn commit_history(args: Vec<OsString>) -> ExitCode {
    run::<CommitHistoryCli>("commit-history", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let data = pixel_ops::history::history(
            &root,
            cli.ref_.as_deref(),
            cli.limit,
            &cli.detail,
            cli.cursor.as_deref(),
            cli.byte_cap,
        )?;
        print_data(&data, cli.json)
    })
}

/// Structured diff between two refs (or ref → working tree).
#[derive(Parser)]
#[command(name = "diff", bin_name = "pixel diff")]
struct DiffCli {
    #[command(flatten)]
    _host: HostFlags,
    from: String,
    /// Optional target ref; if omitted, diff to working tree.
    to: Option<String>,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Restrict diff to these paths.
    #[arg(long)]
    paths: Vec<String>,
    /// Cap diff text bytes.
    #[arg(long)]
    byte_cap: Option<usize>,
    #[arg(long)]
    json: bool,
}

pub fn diff(args: Vec<OsString>) -> ExitCode {
    run::<DiffCli>("diff", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let paths = (!cli.paths.is_empty()).then_some(cli.paths.as_slice());
        let data = pixel_ops::diff::diff(&root, &cli.from, cli.to.as_deref(), paths, cli.byte_cap)?;
        print_data(&data, cli.json)
    })
}

/// One-call read-only branch inventory: ahead/behind, merged, stale,
/// unpushed — the deterministic "did you push everything?" answer.
#[derive(Parser)]
#[command(name = "list-branches", bin_name = "pixel list-branches")]
struct ListBranchesCli {
    #[command(flatten)]
    _host: HostFlags,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Run `git fetch --prune <remote>` first for a live view.
    #[arg(long)]
    fetch: bool,
    /// Remote name.
    #[arg(long, default_value = "origin")]
    remote: String,
    /// Days after which a branch counts as stale.
    #[arg(long, default_value_t = 30)]
    stale_days: u64,
    #[arg(long)]
    json: bool,
}

pub fn list_branches(args: Vec<OsString>) -> ExitCode {
    run::<ListBranchesCli>("list-branches", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let opts = pixel_ops::branches::BranchesOptions {
            fetch: cli.fetch,
            remote: cli.remote,
            stale_days: cli.stale_days,
        };
        let data = pixel_ops::branches::branches(&root, &opts)?;
        print_data(&data, cli.json)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compact_answer_drops_the_clean_list_and_its_markers() {
        let mut data = json!({"head": "a", "clean": ["x"], "clean_list_cap": 200,
            "clean_list_truncated": false, "clean_count": 1});
        compact_repo_state(&mut data);
        assert_eq!(data, json!({"head": "a", "clean_count": 1}));
    }

    #[test]
    fn restricting_to_files_keeps_those_paths_and_recounts() {
        let mut data = json!({
            "dirty": [{"path": "a.rs"}, {"path": "b.rs"}],
            "clean": ["c.rs", "d.rs"],
            "dirty_count": 2,
            "clean_count": 2,
        });
        restrict_to_files(&mut data, &["b.rs".to_string(), "c.rs".to_string()]);
        assert_eq!(data["dirty"], json!([{"path": "b.rs"}]));
        assert_eq!(data["clean"], json!(["c.rs"]));
        assert_eq!(data["dirty_count"], 1);
        assert_eq!(data["clean_count"], 1);
    }
}

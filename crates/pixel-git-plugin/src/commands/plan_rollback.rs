// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! `plan-rollback`: the surgical revert planner.
//!
//! The planner and the gated restore ([`crate::rescue`]) are ported from
//! `crates/pixel/src/rescue_cmd.rs` of Pixel-CLI/pixel (MIT); the command line
//! and the printing are ported from `crates/pixel/src/main.rs`. Both are
//! private to the `pixel` binary crate.
//!
//! One behaviour differs from the built-in: when no `--file` is given, targets
//! are located by core's `Service` run in this process (what the built-in does
//! when no daemon answers) and the daemon is never asked.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use pixel_daemon::api::{Request, Service};
use serde_json::Value;

use crate::env::resolve_path;
use crate::output::print_data;
use crate::rescue;
use crate::root::discover_root;
use crate::run::{HostFlags, run};

/// Surgical revert planner: locate the files a problem points at, list
/// recent versions with the likely-breaking commit flagged, recommend a
/// last-known-good candidate. Plan only — nothing is written without
/// --apply. Never resets; never touches the index or HEAD.
#[derive(Parser)]
#[command(name = "plan-rollback", bin_name = "pixel plan-rollback")]
struct Cli {
    #[command(flatten)]
    _host: HostFlags,
    /// Problem description ("login was working before ...").
    problem: Option<String>,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Explicit target file(s), repo-relative; skips target discovery.
    #[arg(long = "file")]
    files: Vec<String>,
    /// Commits of per-file history to inspect.
    #[arg(long, default_value_t = 10)]
    depth: usize,
    /// Restore the --file targets to this commit (gated action).
    #[arg(long)]
    apply: Option<String>,
    /// With --apply on dirty files: deterministic 3-way merge that keeps
    /// in-progress edits (may leave conflict markers).
    #[arg(long)]
    merge: bool,
    /// With --apply: `git stash push` the dirty planned files first.
    #[arg(long)]
    stash_first: bool,
    /// With --apply: overwrite dirty files (loses in-progress work).
    #[arg(long)]
    allow_dirty: bool,
    #[arg(long)]
    json: bool,
}

/// The sniper target engine's answer for `task`, from an in-process service.
fn targets_in_process(root: &Path, task: &str) -> Result<Value, String> {
    let mut service = Service::open(root).map_err(|e| e.to_string())?;
    let response = service.handle(Request::Targets {
        task: task.to_string(),
        limit: Some(10),
        max_tier: None,
        precision: false,
        regions: false,
    });
    if !response.ok {
        return Err(response.error_message());
    }
    Ok(response.result.unwrap_or(Value::Null))
}

pub fn plan_rollback(args: Vec<OsString>) -> ExitCode {
    run::<Cli>("plan-rollback", args, execute)
}

fn execute(cli: Cli) -> Result<(), String> {
    let Cli {
        problem,
        path,
        files,
        depth,
        apply,
        merge,
        stash_first,
        allow_dirty,
        json,
        ..
    } = cli;
    let root = discover_root(&resolve_path(&path))?;
    if let Some(oid) = apply {
        let result = rescue::apply(
            &root,
            &oid,
            &files,
            &rescue::ApplyOptions {
                merge,
                stash_first,
                allow_dirty,
            },
        )?;
        if json {
            return print_data(&result, true);
        }
        if let Some(applied) = result["files"].as_array() {
            for f in applied {
                println!(
                    "{}: {}{}",
                    f["path"].as_str().unwrap_or("?"),
                    f["action"].as_str().unwrap_or("?"),
                    f["conflicts"]
                        .as_i64()
                        .filter(|c| *c > 0)
                        .map(|c| format!(" ({c} conflict hunk(s) — resolve the markers)"))
                        .unwrap_or_default(),
                );
            }
        }
        println!("{}", result["note"].as_str().unwrap_or(""));
        return Ok(());
    }
    let problem = problem.ok_or_else(|| "missing problem description".to_string())?;
    // Locate targets: explicit --file hints win; otherwise the sniper
    // target engine points the problem at files (P0 slice).
    let (target_paths, keywords) = if files.is_empty() {
        let data = targets_in_process(&root, &problem)?;
        let all = data["targets"].as_array().cloned().unwrap_or_default();
        let mut paths: Vec<String> = all
            .iter()
            .filter(|t| t["tier"] == "P0")
            .filter_map(|t| t["path"].as_str().map(str::to_string))
            .take(5)
            .collect();
        if paths.is_empty() {
            paths = all
                .iter()
                .filter_map(|t| t["path"].as_str().map(str::to_string))
                .take(5)
                .collect();
        }
        let kws: Vec<String> = data["keywords"]
            .as_array()
            .map(|ks| {
                ks.iter()
                    .filter_map(|k| k.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        (paths, kws)
    } else {
        let q = pixel_rank::tokenize_task(&problem).unwrap_or_default();
        (files.clone(), q.keywords)
    };
    if target_paths.is_empty() {
        return Err(
            "could not locate target files for this problem — pass --file <path>".to_string(),
        );
    }
    let plan = rescue::plan(&root, &problem, &target_paths, &keywords, depth)?;
    if json {
        return print_data(&plan, true);
    }
    for t in plan["targets"].as_array().cloned().unwrap_or_default() {
        println!(
            "{}{}",
            t["path"].as_str().unwrap_or("?"),
            if t["dirty"].as_bool().unwrap_or(false) {
                "  [DIRTY — has uncommitted changes]"
            } else {
                ""
            }
        );
        for v in t["versions"].as_array().cloned().unwrap_or_default() {
            println!(
                "  {}  {}{}",
                v["short"].as_str().unwrap_or("?"),
                v["subject"].as_str().unwrap_or(""),
                if v["suspect"].as_bool().unwrap_or(false) {
                    "  [SUSPECT]"
                } else {
                    ""
                }
            );
        }
        if let Some(rec) = t["recommended"].as_object() {
            println!(
                "  → recommended: {} ({})",
                rec.get("oid").and_then(Value::as_str).unwrap_or("?"),
                rec.get("reason").and_then(Value::as_str).unwrap_or(""),
            );
        }
        println!();
    }
    for c in plan["decision"]["caveats"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        eprintln!("⚠ {}", c.as_str().unwrap_or(""));
    }
    if let Some(cmd) = plan["decision"]["options"][0]["command"].as_str() {
        println!("revert: {cmd}");
    }
    println!("fix forward: keep current code and fix the bug in place");
    Ok(())
}

// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! `edit-env`: additive-only, key-level `.env` mutations with snapshots and
//! restore. Values are NEVER printed in any output.
//!
//! Restored from `87e17b41^` of Pixel-CLI/pixel (MIT), where core deleted it
//! in #898: the command line and handler from `crates/pixel/src/main.rs`, the
//! engine as [`crate::envfile`].

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::env::resolve_path;
use crate::envfile::{EnvAction, envfile};
use crate::output::print_data;
use crate::root::discover_root;
use crate::run::{HostFlags, run};

/// Additive-only, key-level .env mutations with snapshots and restore.
/// Values are NEVER printed in any output.
#[derive(Parser)]
#[command(name = "edit-env", bin_name = "pixel edit-env")]
struct EditEnvCli {
    #[command(flatten)]
    _host: HostFlags,
    #[command(subcommand)]
    cmd: EnvCmd,
}

#[derive(Subcommand)]
enum EnvCmd {
    /// List .env files under root — key NAMES only, never values.
    Inventory {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Set one key (snapshot-first; every other line byte-preserved).
    Set {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: String,
        /// Create the file if it does not exist.
        #[arg(long)]
        create_file: bool,
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Restore from a snapshot (latest if --snapshot omitted; undoable).
    Restore {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        snapshot: Option<String>,
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// List snapshots recorded for a file.
    Snapshots {
        #[arg(long)]
        file: PathBuf,
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify required keys exist (names only).
    Check {
        #[arg(long)]
        file: PathBuf,
        /// Required key name; repeat the flag once per key.
        #[arg(long = "require")]
        require: Vec<String>,
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

pub fn edit_env(args: Vec<OsString>) -> ExitCode {
    run::<EditEnvCli>("edit-env", args, |cli| {
        let (path, json, action) = match cli.cmd {
            EnvCmd::Inventory { path, json } => (path, json, EnvAction::Inventory),
            EnvCmd::Set {
                file,
                key,
                value,
                create_file,
                path,
                json,
            } => (
                path,
                json,
                EnvAction::Set {
                    file,
                    key,
                    value,
                    create_file,
                },
            ),
            EnvCmd::Restore {
                file,
                snapshot,
                path,
                json,
            } => (path, json, EnvAction::Restore { file, snapshot }),
            EnvCmd::Snapshots { file, path, json } => (path, json, EnvAction::Snapshots { file }),
            EnvCmd::Check {
                file,
                require,
                path,
                json,
            } => (path, json, EnvAction::Check { file, require }),
        };
        let root = discover_root(&resolve_path(&path))?;
        let data = envfile(&root, &action)?;
        print_data(&data, json)
    })
}

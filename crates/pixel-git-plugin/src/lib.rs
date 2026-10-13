// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The `git` plugin of Pixel: nineteen commands behind one `pixel-git`
//! binary (plugin API 1).
//!
//! A plugin is a separate executable that Pixel core runs for `pixel <name> …`.
//! This one provides many names, so [`dispatch`] picks the command from the
//! first argument (`pixel-git commit …`) or from the name the binary was
//! started under (`pixel-commit`, a link made by `pixel-git install-links`).
//!
//! Most of the code is ported from `crates/pixel/src/main.rs` and
//! `crates/pixel-ops` of Pixel-CLI/pixel (MIT), where it was private to the
//! `pixel` binary or deleted from it; each module says where it came from.
//! What several commands need from the host lives in [`env`], [`root`],
//! [`output`] and [`run`].

pub mod commands;
pub mod engine;
pub mod env;
pub mod envfile;
pub mod links;
pub mod output;
pub mod provenance;
pub mod rescue;
pub mod root;
pub mod run;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

/// The graph store's file name inside `.pixel/`. Core spells it
/// `pixel_daemon::api::GRAPH_DB_FILE`; `tests/core_drift.rs` fails when the two
/// differ, so a pin bump that renames the file cannot pass unnoticed.
pub const GRAPH_DB_FILE: &str = "graph.v2.db";

/// What `pixel-git --help` prints.
fn usage() -> String {
    let mut text = String::from(
        "pixel-git: the git commands of Pixel\n\n\
         Usage: pixel-git <command> [args]...\n       \
         pixel-<command> [args]...        (after `pixel-git install-links`)\n       \
         pixel <command> [args]...        (the host runs the line above)\n\n\
         Commands:\n",
    );
    for c in &commands::COMMANDS {
        text.push_str(&format!("  {:<16}{}\n", c.name, c.about));
    }
    text.push_str(
        "\nOther:\n  install-links   Link pixel-<command> to this binary so `pixel <command>` finds it on PATH\n\n\
         `pixel-git <command> --help` shows a command's own options.\n",
    );
    text
}

/// The command an executable name selects: `pixel-commit` is `commit`, while
/// `pixel-git` itself selects none.
pub fn command_of_program(program: &Path) -> Option<&'static commands::Command> {
    let stem = program.file_name()?.to_str()?;
    commands::find(stem.strip_prefix("pixel-")?)
}

/// Run the plugin for `args` (the whole argv, program name first).
pub fn dispatch(args: &[OsString]) -> ExitCode {
    let Some(program) = args.first() else {
        return ExitCode::FAILURE;
    };
    if let Some(command) = command_of_program(Path::new(program)) {
        return command.run(&args[1..]);
    }
    let Some(first) = args.get(1) else {
        eprint!("{}", usage());
        return ExitCode::from(2);
    };
    match first.to_str() {
        Some("-h" | "--help" | "help") => {
            print!("{}", usage());
            ExitCode::SUCCESS
        }
        Some("-V" | "--version") => {
            println!("pixel-git {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("install-links") => links::run(&args[2..]),
        Some(name) => match commands::find(name) {
            Some(command) => command.run(&args[2..]),
            None => {
                eprintln!("pixel-git: unknown command `{name}`\n");
                eprint!("{}", usage());
                ExitCode::from(2)
            }
        },
        None => {
            eprintln!("pixel-git: the command name is not valid UTF-8");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_name_selects_its_command_and_pixel_git_none() {
        assert_eq!(
            command_of_program(Path::new("/x/bin/pixel-commit")).map(|c| c.name),
            Some("commit")
        );
        assert_eq!(
            command_of_program(Path::new("pixel-dig-history")).map(|c| c.name),
            Some("dig-history")
        );
        assert!(command_of_program(Path::new("/x/pixel-git")).is_none());
        assert!(command_of_program(Path::new("commit")).is_none());
        assert!(command_of_program(Path::new("pixel-nope")).is_none());
    }

    #[test]
    fn the_usage_lists_every_command() {
        let text = usage();
        for c in &commands::COMMANDS {
            assert!(text.contains(c.name), "{}", c.name);
        }
    }
}

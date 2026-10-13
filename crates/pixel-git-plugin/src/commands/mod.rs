// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The nineteen commands of the `git` plugin and how a name finds its code.

use std::ffi::OsString;
use std::process::ExitCode;

mod edit_env;
mod history;
mod inspect;
mod mutate;
mod plan_rollback;
mod who_wrote;

/// One command: the name `pixel <name>` is typed with, a line for `--help`,
/// and the entry point, which receives the arguments after the name.
pub struct Command {
    pub name: &'static str,
    pub about: &'static str,
    run: fn(Vec<OsString>) -> ExitCode,
}

impl Command {
    /// Run the command on `args` (the arguments after its name).
    pub fn run(&self, args: &[OsString]) -> ExitCode {
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push(OsString::from(format!("pixel {}", self.name)));
        argv.extend_from_slice(args);
        (self.run)(argv)
    }
}

const fn command(
    name: &'static str,
    about: &'static str,
    run: fn(Vec<OsString>) -> ExitCode,
) -> Command {
    Command { name, about, run }
}

/// Every command, in the order `--help` lists them. This list is also the
/// `commands` array of `pixel-plugin.toml` (`scripts/check-manifests.sh`
/// compares the two).
pub const COMMANDS: [Command; 19] = [
    command(
        "commit",
        "Stage files, commit, and optionally push (crash-safe, idempotent)",
        mutate::commit,
    ),
    command(
        "push",
        "Leased push to a remote (crash-safe, idempotent)",
        mutate::push,
    ),
    command(
        "commit-and-push",
        "Commit then leased push in one op",
        mutate::commit_and_push,
    ),
    command(
        "new-branch",
        "Create a new branch from HEAD (or --from <ref>)",
        mutate::new_branch,
    ),
    command(
        "fast-forward",
        "Fast-forward merge to a target OID (refuses non-ff + dirty intersection)",
        mutate::fast_forward,
    ),
    command(
        "squash-branch",
        "Squash a branch since its base into one crash-safe commit",
        mutate::squash_branch,
    ),
    command(
        "sync-branch",
        "One-call deterministic branch sync",
        history::sync_branch,
    ),
    command("fetch", "Fetch from a remote (idempotent)", mutate::fetch),
    command(
        "list-branches",
        "One-call read-only branch inventory: ahead/behind, merged, stale, unpushed",
        inspect::list_branches,
    ),
    command(
        "diff",
        "Structured diff between two refs (or ref to working tree)",
        inspect::diff,
    ),
    command(
        "commit-history",
        "Commit history with detail levels and byte caps",
        inspect::commit_history,
    ),
    command(
        "repo-state",
        "Show repo state: HEAD, branch, dirty files, fingerprints",
        inspect::repo_state,
    ),
    command(
        "review-changes",
        "Review working-tree changes (staged, unstaged, untracked, conflicted)",
        inspect::review_changes,
    ),
    command(
        "who-wrote",
        "Per-region blame attribution: who introduced/owns each region of a file",
        who_wrote::who_wrote,
    ),
    command(
        "edit-env",
        "Additive-only, key-level .env mutations with snapshots and restore",
        edit_env::edit_env,
    ),
    command(
        "plan-rollback",
        "Surgical revert planner; --apply <oid> restores files, never resets",
        plan_rollback::plan_rollback,
    ),
    command(
        "search-history",
        "History-wide fact + diff search",
        history::search_history,
    ),
    command(
        "file-history",
        "Lifecycle of a path or token",
        history::file_history,
    ),
    command(
        "dig-history",
        "History-wide discovery; --show <oid> prints a file as it was",
        history::dig_history,
    ),
];

/// The command named `name`, if this plugin provides it.
pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_found() {
        for (i, c) in COMMANDS.iter().enumerate() {
            assert!(find(c.name).is_some_and(|f| f.name == c.name));
            assert!(
                COMMANDS[i + 1..].iter().all(|o| o.name != c.name),
                "{}",
                c.name
            );
        }
        assert!(find("git").is_none());
        assert!(find("").is_none());
    }
}

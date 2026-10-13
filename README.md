# pixel-git

The `git` plugin for [Pixel](https://github.com/Pixel-CLI/pixel): every git
command Pixel offers, in one `pixel-git` binary. These nineteen commands used
to be built into `pixel`; they live here so the core stays small and only the
people who use them install them.

| Command | What it does |
| --- | --- |
| `pixel commit` | stage files, commit, optionally push (crash-safe, idempotent, `--request-id`) |
| `pixel push` | leased push to a remote |
| `pixel commit-and-push` | commit, then leased push, in one operation |
| `pixel new-branch` | create a branch from HEAD or `--from <ref>` |
| `pixel fast-forward` | fast-forward to a target OID; refuses non-ff and a dirty intersection |
| `pixel squash-branch` | squash a branch since its base into one crash-safe commit, backup ref kept |
| `pixel sync-branch` | one-call deterministic branch sync; `--into <target>` rebases and fast-forwards |
| `pixel fetch` | fetch from a remote |
| `pixel list-branches` | ahead/behind, merged, stale and unpushed branches in one answer |
| `pixel diff` | structured diff between two refs, or a ref and the working tree |
| `pixel commit-history` | commit log with detail levels and byte caps |
| `pixel repo-state` | HEAD, branch, dirty files, fingerprints |
| `pixel review-changes` | staged, unstaged, untracked and conflicted changes |
| `pixel who-wrote` | per-region blame attribution, with an optional did-this-author-touch-it verdict |
| `pixel edit-env` | additive-only key-level `.env` edits with snapshots; values are never printed |
| `pixel plan-rollback` | surgical revert planner; `--apply <oid>` restores files, never resets |
| `pixel search-history` | history-wide fact and diff search |
| `pixel file-history` | lifecycle of a path or token |
| `pixel dig-history` | history-wide discovery; `--show <oid> --file <path>` prints a file as it was |

Each command reproduces the built-in's arguments, output and exit status.
`pixel-git --help` lists them; `pixel-git <command> --help` shows one's options.

## Install

With a Pixel that has the plugin host (`pixel install <source>`):

```bash
pixel install github:Pixel-CLI/pixel-git
pixel plugin enable git        # the plugin declares network access (push, fetch)
```

The release assets are `pixel-git-<target-triple>` executables, each with a
`.sha256` file: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`.

From source, without Pixel's plugin management:

```bash
cargo install --git https://github.com/Pixel-CLI/pixel-git pixel-git-plugin
pixel-git install-links        # makes `pixel commit`, `pixel diff`, ... find it
```

## How `pixel <command>` finds this plugin

Plugin API 1 resolves `pixel commit` to a plugin *named* `commit`, or to an
executable `pixel-commit` on `PATH`. This repository is **one** plugin named
`git` that answers to nineteen names, so:

- `pixel git <command> …` always works once the plugin is installed (the host
  runs `bin/pixel-git`, which is, or finds, the `pixel-git` binary).
- `pixel <command> …` works through `PATH` links. `pixel-git install-links`
  creates `pixel-<command>` symlinks to `pixel-git` next to it (or in
  `--dir <dir>`); the binary runs the command named by the name it was started
  under. `pixel-git install-links --remove` deletes them. It never replaces a
  file it did not make.
- [`pixel-plugin.toml`](pixel-plugin.toml) lists the nineteen names under
  `commands`. API 1 ignores that key. A host that reads it could route every
  listed name to `pixel-git <name>` with no links at all; that is the clean fix,
  and `tests/manifest.rs` keeps the list equal to what the binary has.

## Environment

The host passes `PIXEL_API`, `PIXEL_REPO_ROOT`, `PIXEL_GRAPH_DB` and
`PIXEL_BIN`; all are optional. The `[PATH]` argument of every command defaults
to `.`, and the default (or an explicit `.`) is replaced by `PIXEL_REPO_ROOT`.
Run bare, a command acts on the repository around the current directory, as the
built-in did.

## Differences from the built-ins

- **No metrics line, no action-log entry.** A plugin is a separate process and
  the invocation accounting belongs to the `pixel` binary. `--metrics on|off`
  is still accepted (hidden), so scripts that pass it keep parsing; usage text
  omits `[OPTIONS]` where `--metrics` was the only option.
- **The history engines run in this process.** `search-history`,
  `file-history`, `dig-history` and `sync-branch` ask core's `Service`, opened
  in process: the answer core gives when no daemon serves the request. They
  never start or use a repository daemon.
- **No old aliases.** `ship`, `publish`, `branch`, `branches`, `history`,
  `inspect`, `review`, `sync`, `reconcile`, `excavate`, `lifecycle`,
  `history-search`, `rescue`, `rewrite`, `update`, `provenance` and `env` were
  clap aliases of the built-ins; the plugin answers to canonical names only.
- **`who-wrote` and `edit-env`** were deleted from core (#898, `87e17b41`) and
  are restored from `87e17b41^`, together with the `provenance` and `envfile`
  engines that went with them.

## Where the code comes from

Library crates of Pixel core (`pixel-ops`, `pixel-git`, `pixel-daemon`,
`pixel-facts`, `pixel-index`, `pixel-proto`, `pixel-rank`) come through a git
dependency pinned to one commit of
[Pixel-CLI/pixel](https://github.com/Pixel-CLI/pixel), recorded once in
`[workspace.dependencies]` of [`Cargo.toml`](Cargo.toml)
(`scripts/check-pin.sh` enforces a single value). Logic that lived only in the
`pixel` binary crate is ported here, MIT, with its origin in each module's doc:

| Here | From |
| --- | --- |
| `commands/mutate.rs`, `inspect.rs`, `history.rs` | the command lines and handlers of `crates/pixel/src/main.rs` |
| `commands/plan_rollback.rs`, `rescue.rs` | `rescue_cmd.rs` and `main.rs` |
| `commands/who_wrote.rs`, `provenance.rs` | `main.rs` and `crates/pixel-ops/src/provenance.rs` at `87e17b41^` |
| `commands/edit_env.rs`, `envfile.rs` | `main.rs` and `crates/pixel-ops/src/envfile.rs` at `87e17b41^` |
| `engine.rs` | `execute` and `unwrap_response` of `main.rs` |
| `env.rs`, `root.rs`, `output.rs`, `run.rs` | `discover_root`, `print_data` with its byte cap (`PIXEL_OUTPUT_CAP_BYTES`), the `--json` failure envelope and the `--request-id` tip |

## Versioning

One version (`[workspace.package]`), equal to the `version` in
`pixel-plugin.toml`; a release is a `vX.Y.Z` tag on `main`, which builds the
assets above. `api = 1` changes only when Pixel changes the plugin API. A bump
of the Pixel pin is a minor version when output changes, otherwise a patch.

## License

MIT, see [LICENSE](LICENSE).

// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! `who-wrote`: per-region blame attribution.
//!
//! Restored from `87e17b41^` of Pixel-CLI/pixel (MIT), where core deleted it
//! in #898: the command line and handler from `crates/pixel/src/main.rs`, the
//! engine as [`crate::provenance`].

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::env::resolve_path;
use crate::output::print_data;
use crate::provenance::{ProvenanceOptions, provenance};
use crate::root::discover_root;
use crate::run::{HostFlags, run};

/// Per-region blame attribution: who introduced/owns each region of a file.
#[derive(Parser)]
#[command(name = "who-wrote", bin_name = "pixel who-wrote")]
struct WhoWroteCli {
    #[command(flatten)]
    _host: HostFlags,
    /// Repo-relative file to attribute.
    file: String,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Restrict to lines a,b (1-based inclusive), e.g. --lines 10,40.
    #[arg(long, value_parser = parse_line_range)]
    lines: Option<(u32, u32)>,
    /// Author query (case-insensitive substring on name or email) —
    /// adds a did-they-touch-this verdict.
    #[arg(long)]
    author: Option<String>,
    /// Max regions emitted (default 200); truncation sets lower_bound.
    #[arg(long, default_value_t = 200)]
    limit_regions: usize,
    #[arg(long)]
    json: bool,
}

fn parse_line_range(s: &str) -> Result<(u32, u32), String> {
    let (a, b) = s
        .split_once(',')
        .ok_or_else(|| format!("expected 'start,end', got '{s}'"))?;
    let a: u32 = a
        .trim()
        .parse()
        .map_err(|e| format!("bad start line: {e}"))?;
    let b: u32 = b.trim().parse().map_err(|e| format!("bad end line: {e}"))?;
    if a == 0 || b < a {
        return Err(format!("invalid range {a},{b}: need 1 <= start <= end"));
    }
    Ok((a, b))
}

pub fn who_wrote(args: Vec<OsString>) -> ExitCode {
    run::<WhoWroteCli>("who-wrote", args, |cli| {
        let root = discover_root(&resolve_path(&cli.path))?;
        let opts = ProvenanceOptions {
            file: cli.file,
            lines: cli.lines,
            author: cli.author,
            limit_regions: cli.limit_regions,
        };
        let data = provenance(&root, &opts)?;
        print_data(&data, cli.json)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_range_is_two_ordered_one_based_numbers() {
        assert_eq!(parse_line_range("10,40"), Ok((10, 40)));
        assert_eq!(parse_line_range(" 3 , 3 "), Ok((3, 3)));
        assert!(parse_line_range("10").unwrap_err().contains("start,end"));
        assert!(parse_line_range("a,2").unwrap_err().contains("bad start"));
        assert!(parse_line_range("1,b").unwrap_err().contains("bad end"));
        assert!(
            parse_line_range("0,5")
                .unwrap_err()
                .contains("invalid range")
        );
        assert!(
            parse_line_range("5,4")
                .unwrap_err()
                .contains("invalid range")
        );
    }
}

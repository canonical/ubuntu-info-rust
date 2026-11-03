//! Code generator for `ubuntu-info-rust`'s series data.
//!
//! Reads `distro-info-data`'s `ubuntu.csv` and writes the crate's
//! `src/series.rs`. The output is a pure function of the input, so regenerating
//! an up-to-date file leaves it unchanged.
//!
//! ```sh
//! cargo run --package codegen -- ubuntu.csv src/series.rs
//! ```

mod emit;
mod model;

use std::collections::HashSet;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: codegen [--allow-removals] <ubuntu.csv> <output.rs>";

fn main() -> ExitCode {
    match run() {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let mut allow_removals = false;
    let mut positional = Vec::new();
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--allow-removals" => allow_removals = true,
            "-h" | "--help" => return Ok(USAGE.to_owned()),
            other if other.starts_with('-') => {
                return Err(format!("unknown option {other}\n{USAGE}"));
            }
            _ => positional.push(argument),
        }
    }
    let [csv_path, output_path] = positional.as_slice() else {
        return Err(USAGE.to_owned());
    };
    let csv_path = Path::new(csv_path);
    let output_path = Path::new(output_path);

    let csv_text = std::fs::read_to_string(csv_path)
        .map_err(|error| format!("cannot read {}: {error}", csv_path.display()))?;
    let dataset = model::derive(&csv_text)?;
    let rendered = emit::render(&dataset);

    let devel = &dataset.series[dataset.devel_index];
    let summary = format!(
        "{} series, reference date {}, development series {} ({})",
        dataset.series.len(),
        dataset.reference_date,
        devel.code_name_short,
        devel.version
    );

    let existing = std::fs::read_to_string(output_path).ok();
    if let Some(existing) = &existing {
        guard_against_removals(existing, &dataset, output_path, allow_removals)?;
    }

    if existing.as_deref() == Some(rendered.as_str()) {
        return Ok(format!(
            "{} is up to date: {}",
            output_path.display(),
            summary
        ));
    }

    write_atomically(output_path, &rendered)?;
    Ok(format!("wrote {}: {}", output_path.display(), summary))
}

/// Returns the short series names recorded in an existing generated file.
fn series_names(existing: &str) -> Vec<&str> {
    existing
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("code_name_short: \"")
                .and_then(|rest| rest.strip_suffix("\","))
        })
        .collect()
}

/// Refuses to drop series that are present in the existing generated file.
///
/// A truncated or stale `ubuntu.csv` can still be structurally valid, so
/// without this guard a regeneration would silently delete series from the
/// crate. Removing a series is possible with `--allow-removals`.
fn guard_against_removals(
    existing: &str,
    dataset: &model::Dataset,
    output_path: &Path,
    allow_removals: bool,
) -> Result<(), String> {
    if allow_removals {
        return Ok(());
    }

    let current: HashSet<&str> = dataset
        .series
        .iter()
        .map(|series| series.code_name_short.as_str())
        .collect();
    let removed: Vec<&str> = series_names(existing)
        .into_iter()
        .filter(|name| !current.contains(name))
        .collect();

    if removed.is_empty() {
        return Ok(());
    }

    Err(format!(
        "refusing to remove series from {}: {}\n\
         if the removal is intentional, pass --allow-removals",
        output_path.display(),
        removed.join(", ")
    ))
}

/// Writes `contents` to `path` through a temporary file and a rename, so an
/// interrupted run cannot leave a partially written file behind.
fn write_atomically(path: &Path, contents: &str) -> Result<(), String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("invalid output path: {}", path.display()))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id()
    ));

    std::fs::write(&temporary, contents)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        format!("cannot replace {}: {error}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::derive;

    const FIXTURE: &str = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30
22.04 LTS,Jammy Jellyfish,jammy,2021-10-14,2022-04-21,2027-06-01,2027-06-01,2032-04-21,2037-04-30
27.04,Tenacious T-Rex,tenacious,2026-10-16,2027-04-22,2028-01-20
";

    /// A structurally valid file that has lost a series, as a truncated
    /// download could.
    const TRUNCATED: &str = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30
27.04,Tenacious T-Rex,tenacious,2026-10-16,2027-04-22,2028-01-20
";

    #[test]
    fn extracts_series_names_from_generated_output() {
        let rendered = emit::render(&derive(FIXTURE).unwrap());
        assert_eq!(series_names(&rendered), ["warty", "jammy", "tenacious"]);
    }

    #[test]
    fn refuses_to_remove_series() {
        let existing = emit::render(&derive(FIXTURE).unwrap());
        let truncated = derive(TRUNCATED).unwrap();
        let error =
            guard_against_removals(&existing, &truncated, Path::new("src/series.rs"), false)
                .unwrap_err();
        assert!(error.contains("jammy"), "error: {error}");
        assert!(error.contains("--allow-removals"), "error: {error}");
    }

    #[test]
    fn allows_removals_when_requested() {
        let existing = emit::render(&derive(FIXTURE).unwrap());
        let truncated = derive(TRUNCATED).unwrap();
        assert!(
            guard_against_removals(&existing, &truncated, Path::new("src/series.rs"), true).is_ok()
        );
    }

    #[test]
    fn accepts_an_unchanged_series_set() {
        let existing = emit::render(&derive(FIXTURE).unwrap());
        let dataset = derive(FIXTURE).unwrap();
        assert!(
            guard_against_removals(&existing, &dataset, Path::new("src/series.rs"), false).is_ok()
        );
    }
}

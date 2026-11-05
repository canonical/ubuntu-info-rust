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

use clap::Parser;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Generates series data from distro-info-data's ubuntu.csv.
#[derive(Debug, Parser)]
#[command(name = "codegen", version, about)]
struct Args {
    /// Allow removal of series present in the existing generated file.
    #[arg(long)]
    allow_removals: bool,

    /// Input ubuntu.csv file.
    #[arg(value_name = "ubuntu.csv")]
    csv_path: PathBuf,

    /// Output Rust source file.
    #[arg(value_name = "output.rs")]
    output_path: PathBuf,
}

fn main() -> ExitCode {
    match run(Args::parse()) {
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

fn run(args: Args) -> Result<String, String> {
    let Args {
        allow_removals,
        csv_path,
        output_path,
    } = args;
    let csv_path = csv_path.as_path();
    let output_path = output_path.as_path();

    let csv_text = std::fs::read_to_string(csv_path)
        .map_err(|error| format!("cannot read {}: {error}", csv_path.display()))?;
    let dataset = model::derive(&csv_text)?;
    let rendered = emit::render(&dataset)
        .map_err(|error| format!("cannot render {}: {error}", output_path.display()))?;

    let devel = &dataset.series[dataset.devel_index];
    let summary = format!(
        "{} series, reference date {}, development series {} ({})",
        dataset.series.len(),
        dataset.reference_date,
        devel.code_name_short,
        devel.version
    );

    let existing = read_existing_output(output_path)?;
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

/// Reads an existing generated file. Only a missing file means there is no
/// baseline; every other read failure halts generation before replacement.
fn read_existing_output(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "cannot read existing generated file {}: {error}",
            path.display()
        )),
    }
}

/// Returns the short series names recorded in an existing generated file.
///
/// The existing file must contain one non-empty `pub static SERIES` slice whose
/// entries are `SeriesInfo` structs with a string `code_name_short` field.
/// Parsing the Rust syntax avoids silently ignoring a malformed baseline.
fn series_names(existing: &str) -> Result<Vec<String>, String> {
    let file =
        syn::parse_file(existing).map_err(|error| format!("invalid Rust syntax: {error}"))?;
    let mut series = None;
    for item in file.items {
        let syn::Item::Static(item) = item else {
            continue;
        };
        if item.ident == "SERIES" && series.replace(item).is_some() {
            return Err("multiple SERIES definitions".to_owned());
        }
    }
    let series = series.ok_or("missing SERIES definition")?;
    let syn::Expr::Reference(reference) = *series.expr else {
        return Err("SERIES is not a slice reference".to_owned());
    };
    let syn::Expr::Array(array) = *reference.expr else {
        return Err("SERIES is not an array literal".to_owned());
    };
    if array.elems.is_empty() {
        return Err("SERIES contains no entries".to_owned());
    }

    let mut names = Vec::with_capacity(array.elems.len());
    let mut seen = HashSet::with_capacity(array.elems.len());
    for (index, element) in array.elems.iter().enumerate() {
        let syn::Expr::Struct(entry) = element else {
            return Err(format!(
                "SERIES entry {} is not a SeriesInfo struct",
                index + 1
            ));
        };
        if !entry.path.is_ident("SeriesInfo") {
            return Err(format!(
                "SERIES entry {} is not a SeriesInfo struct",
                index + 1
            ));
        }
        let expected_fields = [
            "code_name",
            "code_name_short",
            "version",
            "release_date",
            "end_of_standard_support",
            "end_of_life",
            "esm_start",
            "legacy_start",
            "is_lts",
        ];
        let fields: HashSet<String> = entry
            .fields
            .iter()
            .filter_map(|field| {
                if let syn::Member::Named(name) = &field.member {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();
        if entry.rest.is_some()
            || entry.fields.len() != expected_fields.len()
            || fields.len() != expected_fields.len()
            || expected_fields.iter().any(|field| !fields.contains(*field))
        {
            return Err(format!(
                "SERIES entry {} has missing, duplicate, or unexpected fields",
                index + 1
            ));
        }
        let mut matches = entry.fields.iter().filter(
            |field| matches!(&field.member, syn::Member::Named(name) if name == "code_name_short"),
        );
        let field = matches
            .next()
            .ok_or_else(|| format!("SERIES entry {} has no code_name_short", index + 1))?;
        if matches.next().is_some() {
            return Err(format!(
                "SERIES entry {} repeats code_name_short",
                index + 1
            ));
        }
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(value),
            ..
        }) = &field.expr
        else {
            return Err(format!(
                "SERIES entry {} has a non-string code_name_short",
                index + 1
            ));
        };
        let name = value.value();
        if name.is_empty() {
            return Err(format!(
                "SERIES entry {} has an empty code_name_short",
                index + 1
            ));
        }
        if !seen.insert(name.clone()) {
            return Err(format!("duplicate code_name_short {name}"));
        }
        names.push(name);
    }
    Ok(names)
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
    let existing_names = series_names(existing).map_err(|error| {
        format!(
            "cannot validate existing generated file {}: {error}",
            output_path.display()
        )
    })?;
    if allow_removals {
        return Ok(());
    }

    let current: HashSet<&str> = dataset
        .series
        .iter()
        .map(|series| series.code_name_short.as_str())
        .collect();
    let removed: Vec<&str> = existing_names
        .iter()
        .map(String::as_str)
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
    fn parses_paths_and_removal_flag() {
        for (arguments, allow_removals) in [
            (vec!["codegen", "ubuntu.csv", "output.rs"], false),
            (
                vec!["codegen", "--allow-removals", "ubuntu.csv", "output.rs"],
                true,
            ),
            (
                vec!["codegen", "ubuntu.csv", "output.rs", "--allow-removals"],
                true,
            ),
        ] {
            let args = Args::try_parse_from(arguments).unwrap();
            assert_eq!(args.csv_path, Path::new("ubuntu.csv"));
            assert_eq!(args.output_path, Path::new("output.rs"));
            assert_eq!(args.allow_removals, allow_removals);
        }
    }

    #[test]
    fn rejects_invalid_arguments() {
        for arguments in [
            vec!["codegen"],
            vec!["codegen", "ubuntu.csv"],
            vec!["codegen", "ubuntu.csv", "output.rs", "extra"],
            vec!["codegen", "--unknown", "ubuntu.csv", "output.rs"],
        ] {
            assert!(Args::try_parse_from(arguments).is_err());
        }
    }

    #[test]
    fn displays_help_and_version_without_required_paths() {
        use clap::error::ErrorKind;

        for (flag, kind) in [
            ("-h", ErrorKind::DisplayHelp),
            ("--help", ErrorKind::DisplayHelp),
            ("-V", ErrorKind::DisplayVersion),
            ("--version", ErrorKind::DisplayVersion),
        ] {
            let error = Args::try_parse_from(["codegen", flag]).unwrap_err();
            assert_eq!(error.kind(), kind);
        }
    }

    #[test]
    fn extracts_series_names_from_generated_output() {
        let rendered = emit::render(&derive(FIXTURE).unwrap()).unwrap();
        assert_eq!(
            series_names(&rendered).unwrap(),
            ["warty", "jammy", "tenacious"]
        );
    }

    #[test]
    fn rejects_malformed_existing_outputs() {
        for existing in [
            "not Rust",
            "pub static OTHER: &[u8] = &[];",
            "pub static SERIES: &[SeriesInfo] = &[];",
            "pub static SERIES: &[SeriesInfo] = &[SeriesInfo { version: \"1.0\" }];",
            "pub static SERIES: &[SeriesInfo] = &[SeriesInfo { code_name_short: 1 }];",
        ] {
            assert!(series_names(existing).is_err(), "existing: {existing}");
        }
    }

    #[test]
    fn refuses_to_remove_series() {
        let existing = emit::render(&derive(FIXTURE).unwrap()).unwrap();
        let truncated = derive(TRUNCATED).unwrap();
        let error =
            guard_against_removals(&existing, &truncated, Path::new("src/series.rs"), false)
                .unwrap_err();
        assert!(error.contains("jammy"), "error: {error}");
        assert!(error.contains("--allow-removals"), "error: {error}");
    }

    #[test]
    fn allows_removals_when_requested() {
        let existing = emit::render(&derive(FIXTURE).unwrap()).unwrap();
        let truncated = derive(TRUNCATED).unwrap();
        assert!(
            guard_against_removals(&existing, &truncated, Path::new("src/series.rs"), true).is_ok()
        );
    }

    #[test]
    fn accepts_an_unchanged_series_set() {
        let existing = emit::render(&derive(FIXTURE).unwrap()).unwrap();
        let dataset = derive(FIXTURE).unwrap();
        assert!(
            guard_against_removals(&existing, &dataset, Path::new("src/series.rs"), false).is_ok()
        );
    }
}

//! Generates the checked-in country-code include from iso3166-static.
//!
//! This generator-only dependency never enters the library's dependency graph.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use iso3166_static::{Alpha2, Error};

#[derive(Parser)]
#[command(about, version)]
struct Args {
    /// Checked-in Rust include to generate.
    output: PathBuf,
}

fn render() -> Result<String, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    for first in b'A'..=b'Z' {
        for second in b'A'..=b'Z' {
            let bytes = [first, second];
            let input = std::str::from_utf8(&bytes)?;
            match Alpha2::from_str_slice(input) {
                Ok(country) => {
                    if country.as_str() != input {
                        return Err(format!("unexpected ISO code normalization: {input}").into());
                    }
                    let variants = [
                        [first, second],
                        [first, second.to_ascii_lowercase()],
                        [first.to_ascii_lowercase(), second],
                        [first.to_ascii_lowercase(), second.to_ascii_lowercase()],
                    ];
                    let mut row = String::from("    [");
                    for (index, bytes) in variants.iter().enumerate() {
                        if index > 0 {
                            row.push_str(", ");
                        }
                        write!(row, "{:?}", std::str::from_utf8(bytes)?)?;
                    }
                    row.push_str("],\n");
                    rows.push(row);
                }
                Err(Error::UnknownCode) => {}
                Err(error) => {
                    return Err(format!("cannot resolve ISO code {input}: {error}").into());
                }
            }
        }
    }
    if rows.is_empty() {
        return Err("iso3166-static returned no alpha-2 codes".into());
    }
    let mut output = String::from(
        "// Generated from iso3166-static 0.4.3 (Apache-2.0); do not edit by hand.\n\
// Source: https://crates.io/crates/iso3166-static\n\
// Regenerate: cargo run --locked -p codegen --bin country_codes -- src/country_codes.rs\n\
// Includes user-assigned Alpha2 values; each row preserves all four case variants.\n\
#[rustfmt::skip]\n",
    );
    writeln!(
        output,
        "static COUNTRY_CODES: [[&str; 4]; {}] = [",
        rows.len()
    )?;
    for row in rows {
        output.push_str(&row);
    }
    output.push_str("];\n");
    Ok(output)
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let rendered = render()?;
    match std::fs::read_to_string(&args.output) {
        Ok(existing) if existing == rendered => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("cannot read {}: {error}", args.output.display()).into()),
    }
    let file_name = args.output.file_name().ok_or("output must name a file")?;
    let temporary = args.output.with_file_name(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let written = (|| -> std::io::Result<()> {
        use std::io::Write as _;
        file.write_all(rendered.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, &args.output)
    })();
    if let Err(error) = written {
        let cleanup = std::fs::remove_file(&temporary);
        return Err(format!(
            "cannot replace {}: {error}; temporary-file cleanup: {cleanup:?}",
            args.output.display()
        )
        .into());
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    let path = args.output.clone();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "error: cannot generate country-code include {}: {error}",
                path.display()
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_country_codes_match_the_dependency() {
        let expected = include_str!("../../../src/country_codes.rs");
        assert_eq!(render().unwrap(), expected);
    }
}

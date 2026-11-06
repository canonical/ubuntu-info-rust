//! Subprocess regression tests for fail-closed data updates.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const FIXTURE: &str = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30
22.04 LTS,Jammy Jellyfish,jammy,2021-10-14,2022-04-21,2027-06-01,2027-06-01,2032-04-21,2037-04-30
27.04,Tenacious T-Rex,tenacious,2026-10-16,2027-04-22,2028-01-20
";

fn generate(input: &Path, output: &Path, allow_removals: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codegen"));
    if allow_removals {
        command.arg("--allow-removals");
    }
    command.arg(input).arg(output).output().unwrap()
}

fn successful_baseline(directory: &TempDir) -> (std::path::PathBuf, std::path::PathBuf, Vec<u8>) {
    let input = directory.path().join("ubuntu.csv");
    let output = directory.path().join("series.rs");
    fs::write(&input, FIXTURE).unwrap();
    let result = generate(&input, &output, false);
    assert!(result.status.success(), "{:?}", result);
    let original = fs::read(&output).unwrap();
    (input, output, original)
}

fn assert_rejected(result: Output, output: &Path, original: &[u8], message: &str) {
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    assert!(
        result.stdout.is_empty(),
        "unexpected success output: {:?}",
        result
    );
    let error = String::from_utf8(result.stderr).unwrap();
    assert!(error.starts_with("error: "), "{error}");
    assert!(
        error.contains(message),
        "expected {message:?}, got {error:?}"
    );
    assert_eq!(fs::read(output).unwrap(), original);
    let files: Vec<_> = fs::read_dir(output.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(files.len(), 2, "unexpected temporary output: {files:?}");
}

#[test]
fn rejects_invalid_updates_without_changing_existing_output() {
    let cases = [
        ("".to_owned(), "unexpected ubuntu.csv header"),
        (
            FIXTURE.replace("4.10,Warty", "oops,Warty"),
            "invalid version",
        ),
        (
            FIXTURE.replace("Warty Warthog", "jammy jellyfish"),
            "duplicate code name",
        ),
        (FIXTURE.replace("2037-04-30", "2032-04-15"), "eol-legacy"),
        (FIXTURE.replace("2006-04-30", "2004-10-19"), "release date"),
        (FIXTURE.replace("2004-03-05", "2004-10-21"), "created date"),
        (FIXTURE.replace("2004-10-20", "2004-13-20"), "invalid date"),
        (
            FIXTURE.replace(
                "4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30\n",
                "",
            ),
            "refusing to remove series",
        ),
        (
            FIXTURE.replace(",2006-04-30\n", ",2006-04-30,,,,extra\n"),
            "expected at most 9",
        ),
    ];
    for (text, message) in cases {
        for allow_removals in [false, true] {
            if allow_removals && message == "refusing to remove series" {
                continue;
            }
            let directory = tempfile::tempdir().unwrap();
            let (input, output, original) = successful_baseline(&directory);
            fs::write(&input, text.as_bytes()).unwrap();
            assert_rejected(
                generate(&input, &output, allow_removals),
                &output,
                &original,
                message,
            );
        }
    }
}

#[test]
fn rejects_a_valid_dataset_without_a_legacy_phase_fixture() {
    let directory = tempfile::tempdir().unwrap();
    let (input, output, original) = successful_baseline(&directory);
    let without_legacy = FIXTURE.replace(",2032-04-21,2037-04-30", ",,");
    fs::write(&input, without_legacy).unwrap();
    assert_rejected(
        generate(&input, &output, false),
        &output,
        &original,
        "no LTS series with legacy support",
    );
}

#[test]
fn rejects_a_phase_end_of_life_without_a_following_day() {
    let directory = tempfile::tempdir().unwrap();
    let (input, output, original) = successful_baseline(&directory);
    let at_date_max = FIXTURE.replace("2037-04-30", "9999-12-31");
    fs::write(&input, at_date_max).unwrap();
    assert_rejected(
        generate(&input, &output, false),
        &output,
        &original,
        "22.04 (jammy): cannot compute the next day after end_of_life 9999-12-31",
    );
}

#[test]
fn rejects_unreadable_or_malformed_baselines_even_when_removals_are_allowed() {
    for malformed in [
        vec![0xff, 0xfe],
        b"not Rust".to_vec(),
        b"pub static SERIES: &[SeriesInfo] = &[];".to_vec(),
        b"pub static SERIES: &[SeriesInfo] = &[SeriesInfo { version: \"4.10\" }];".to_vec(),
    ] {
        for allow_removals in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let (input, output, _) = successful_baseline(&directory);
            fs::write(&output, &malformed).unwrap();
            assert_rejected(
                generate(&input, &output, allow_removals),
                &output,
                &malformed,
                if malformed[0] == 0xff {
                    "cannot read existing generated file"
                } else {
                    "cannot validate existing generated file"
                },
            );
        }
    }
}

#[test]
fn a_missing_input_preserves_the_existing_output() {
    let directory = tempfile::tempdir().unwrap();
    let (input, output, original) = successful_baseline(&directory);
    fs::remove_file(&input).unwrap();
    let result = generate(&input, &output, false);
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("cannot read")
    );
    assert_eq!(fs::read(output).unwrap(), original);
}

#[test]
fn initial_generation_is_idempotent_and_allows_explicit_removal() {
    let directory = tempfile::tempdir().unwrap();
    let (input, output, original) = successful_baseline(&directory);
    let result = generate(&input, &output, false);
    assert!(result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("is up to date")
    );
    assert_eq!(fs::read(&output).unwrap(), original);
    let truncated = FIXTURE.replace(
        "4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30\n",
        "",
    );
    fs::write(&input, truncated).unwrap();
    assert!(generate(&input, &output, true).status.success());
    let updated = fs::read_to_string(&output).unwrap();
    assert!(!updated.contains("code_name_short: \"warty\""));
    assert!(generate(&input, &output, false).status.success());
}

#[test]
fn a_directory_output_is_a_read_error_not_an_absent_baseline() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("ubuntu.csv");
    let output = directory.path().join("series.rs");
    fs::write(&input, FIXTURE).unwrap();
    fs::create_dir(&output).unwrap();
    let sentinel = output.join("unchanged");
    fs::write(&sentinel, b"original").unwrap();
    let result = generate(&input, &output, true);
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("cannot read existing generated file")
    );
    assert_eq!(fs::read(sentinel).unwrap(), b"original");
}

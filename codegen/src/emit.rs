//! Renders `src/series.rs` from a derived [`Dataset`].
//!
//! The output is a pure function of the dataset, so regenerating an
//! up-to-date file is a no-op. The emitted code is formatted the way rustfmt
//! would format it, so `cargo fmt --check` passes without a formatting step.

use std::fmt::Write as _;

use crate::date::Date;
use crate::model::{Dataset, Series};

const TEMPLATE: &str = include_str!("series.rs.template");

/// The maximum line width rustfmt enforces.
const MAX_WIDTH: usize = 100;
/// rustfmt keeps small collections inline up to this width.
const ARRAY_WIDTH: usize = 60;
/// Indentation of the test function bodies in the template.
const TEST_INDENT: usize = 8;

/// Renders the complete contents of `src/series.rs`.
pub fn render(dataset: &Dataset) -> String {
    let devel = &dataset.series[dataset.devel_index];
    let phase = phase_series(dataset);

    let rendered = TEMPLATE
        .to_owned()
        .replace_once("@@REFERENCE_DATE@@", &dataset.reference_date.rust_args())
        .replace_once("@@SERIES_ENTRIES@@", &series_entries(&dataset.series))
        .replace_once("@@DEVEL_INDEX@@", &dataset.devel_index.to_string())
        .replace_once(
            "@@VERSION_INDEX@@",
            &index_entries(&dataset.series, Index::Version),
        )
        .replace_once(
            "@@SHORT_NAME_INDEX@@",
            &index_entries(&dataset.series, Index::ShortName),
        )
        .replace_once(
            "@@FULL_NAME_INDEX@@",
            &index_entries(&dataset.series, Index::FullName),
        )
        .replace_once("@@DEVEL_SHORT@@", &devel.code_name_short)
        .replace_once("@@DEVEL_CODE_NAME@@", &devel.code_name)
        .replace_once("@@DEVEL_VERSION@@", &devel.version)
        .replace_once(
            "@@ASSERT_ACTIVE@@",
            &assert_group("active", &group(dataset, Group::Active)),
        )
        .replace_once(
            "@@ASSERT_ESM@@",
            &assert_group("esm", &group(dataset, Group::EnteredEsm)),
        )
        .replace_once(
            "@@ASSERT_CURRENT_ESM@@",
            &assert_group("current_esm", &group(dataset, Group::CurrentEsm)),
        )
        .replace_once(
            "@@ASSERT_LEGACY@@",
            &assert_group("legacy", &group(dataset, Group::EnteredLegacy)),
        )
        .replace_once(
            "@@ASSERT_CURRENT_LEGACY@@",
            &assert_group("current_legacy", &group(dataset, Group::CurrentLegacy)),
        )
        .replace_once(
            "@@ASSERT_EOL@@",
            &assert_group("eol", &group(dataset, Group::EndOfLife)),
        )
        .replace_once("@@PHASE_VERSION@@", &phase.version)
        .replace_once(
            "@@PHASE_ESM@@",
            &phase.end_of_standard_support.next_day().rust_args(),
        )
        .replace_once("@@PHASE_EOL@@", &phase.end_of_life.next_day().rust_args());

    assert!(
        !rendered.contains("@@"),
        "the template has placeholders that the renderer does not fill"
    );
    rendered
}

/// Replaces the single occurrence of `placeholder` with `value`.
trait ReplaceOnce {
    fn replace_once(self, placeholder: &str, value: &str) -> String;
}

impl ReplaceOnce for String {
    fn replace_once(self, placeholder: &str, value: &str) -> String {
        let count = self.matches(placeholder).count();
        assert_eq!(
            count, 1,
            "placeholder {placeholder} appears {count} times in the template"
        );
        self.replacen(placeholder, value, 1)
    }
}

/// Which identifier an index map is keyed by.
enum Index {
    Version,
    ShortName,
    FullName,
}

fn index_entries(series: &[Series], index: Index) -> String {
    let mut out = String::new();
    for (position, entry) in series.iter().enumerate() {
        let key = match index {
            Index::Version => entry.version.clone(),
            Index::ShortName => entry.code_name_short.clone(),
            Index::FullName => entry.code_name.to_ascii_lowercase(),
        };
        let _ = writeln!(out, "    {key:?} => {position},");
    }
    out
}

fn series_entries(series: &[Series]) -> String {
    let mut out = String::new();
    for entry in series {
        let _ = writeln!(out, "    SeriesInfo {{");
        let _ = writeln!(out, "        code_name: {:?},", entry.code_name);
        let _ = writeln!(out, "        code_name_short: {:?},", entry.code_name_short);
        let _ = writeln!(out, "        version: {:?},", entry.version);
        let _ = writeln!(
            out,
            "        release_date: {},",
            entry.release_date.rust_expr()
        );
        let _ = writeln!(
            out,
            "        end_of_standard_support: {},",
            entry.end_of_standard_support.rust_expr()
        );
        let _ = writeln!(
            out,
            "        end_of_life: {},",
            entry.end_of_life.rust_expr()
        );
        let _ = writeln!(out, "        esm_start: {},", optional(&entry.esm_start));
        let _ = writeln!(
            out,
            "        legacy_start: {},",
            optional(&entry.legacy_start)
        );
        let _ = writeln!(out, "        is_lts: {},", entry.is_lts);
        let _ = writeln!(out, "    }},");
    }
    out
}

fn optional(date: &Option<Date>) -> String {
    match date {
        Some(date) => format!("Some({})", date.rust_expr()),
        None => "None".to_owned(),
    }
}

/// The lifecycle phases, mirroring the crate's `Lifecycle` enum.
///
/// The generator computes the expected lifecycle groups independently of the
/// crate so that the generated tests cross-check the crate's logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Development,
    StandardSupport,
    ExpandedSecurityMaintenance,
    LegacySupport,
    EndOfLife,
}

/// The lifecycle phase of `series` on `date`, mirroring
/// `SeriesInfo::lifecycle_on`.
fn lifecycle_on(series: &Series, date: Date) -> Phase {
    if date < series.release_date {
        Phase::Development
    } else if date <= series.end_of_standard_support {
        Phase::StandardSupport
    } else if date > series.end_of_life {
        Phase::EndOfLife
    } else if series.legacy_start.is_some_and(|start| date >= start) {
        Phase::LegacySupport
    } else if series.esm_start.is_some_and(|start| date >= start) {
        Phase::ExpandedSecurityMaintenance
    } else {
        Phase::EndOfLife
    }
}

/// The lifecycle groups asserted by the generated tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Active,
    EnteredEsm,
    CurrentEsm,
    EnteredLegacy,
    CurrentLegacy,
    EndOfLife,
}

fn group(dataset: &Dataset, group: Group) -> Vec<String> {
    let reference_date = dataset.reference_date;
    dataset
        .series
        .iter()
        .filter(|series| match group {
            Group::Active => {
                series.release_date <= reference_date && reference_date <= series.end_of_life
            }
            Group::EnteredEsm => series
                .esm_start
                .is_some_and(|start| start <= reference_date),
            Group::CurrentEsm => {
                lifecycle_on(series, reference_date) == Phase::ExpandedSecurityMaintenance
            }
            Group::EnteredLegacy => series
                .legacy_start
                .is_some_and(|start| start <= reference_date),
            Group::CurrentLegacy => lifecycle_on(series, reference_date) == Phase::LegacySupport,
            Group::EndOfLife => lifecycle_on(series, reference_date) == Phase::EndOfLife,
        })
        .map(|series| series.code_name_short.clone())
        .collect()
}

/// Renders an `assert_eq!` statement comparing a group with its expected
/// values, formatted the way rustfmt would.
fn assert_group(name: &str, elements: &[String]) -> String {
    let quoted: Vec<String> = elements
        .iter()
        .map(|element| format!("{element:?}"))
        .collect();
    let indent = " ".repeat(TEST_INDENT);

    let single_line = format!("assert_eq!({name}, [{}]);", quoted.join(", "));
    let array_width = quoted.join(", ").len() + 2;
    if TEST_INDENT + single_line.len() <= MAX_WIDTH && array_width <= ARRAY_WIDTH {
        // The template's own newline terminates the line.
        return format!("{indent}{single_line}");
    }

    let element_indent = TEST_INDENT + 8;
    let mut out = format!("{indent}assert_eq!(\n");
    let _ = writeln!(out, "{indent}    {name},");
    let _ = writeln!(out, "{indent}    [");
    out.push_str(&fill_lines(&quoted, element_indent));
    let _ = writeln!(out, "{indent}    ]");
    let _ = write!(out, "{indent});");
    out
}

/// Packs `elements` into lines of at most [`MAX_WIDTH`] columns, the way
/// rustfmt lays out array literals that do not fit on one line.
fn fill_lines(elements: &[String], indent: usize) -> String {
    let padding = " ".repeat(indent);
    let mut out = String::new();
    let mut line = String::new();

    for (position, element) in elements.iter().enumerate() {
        let last = position + 1 == elements.len();
        let token = if last {
            element.clone()
        } else {
            format!("{element},")
        };
        let separating_space = usize::from(!line.is_empty());
        if !line.is_empty() && indent + line.len() + separating_space + token.len() > MAX_WIDTH {
            let _ = writeln!(out, "{padding}{line}");
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&token);
    }

    if !line.is_empty() {
        let _ = writeln!(out, "{padding}{line}");
    }
    out
}

/// Returns the newest released LTS series that has legacy support, falling back
/// to the newest LTS series with legacy support. The generated phase test
/// probes every lifecycle phase on this series.
fn phase_series(dataset: &Dataset) -> &Series {
    lts_with_legacy(dataset)
        .rev()
        .find(|series| series.release_date <= dataset.reference_date)
        .or_else(|| lts_with_legacy(dataset).next_back())
        .expect("the data has an LTS series with legacy support")
}

fn lts_with_legacy(dataset: &Dataset) -> impl DoubleEndedIterator<Item = &Series> + '_ {
    dataset
        .series
        .iter()
        .filter(|series| series.is_lts && series.legacy_start.is_some())
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

    #[test]
    fn rendering_is_deterministic() {
        let first = render(&derive(FIXTURE).unwrap());
        let second = render(&derive(FIXTURE).unwrap());
        assert_eq!(first, second);
    }

    #[test]
    fn rendering_replaces_every_placeholder() {
        let rendered = render(&derive(FIXTURE).unwrap());
        assert!(!rendered.contains("@@"));
    }

    #[test]
    fn lines_fit_the_rustfmt_width() {
        let rendered = render(&derive(FIXTURE).unwrap());
        for line in rendered.lines() {
            assert!(
                line.chars().count() <= MAX_WIDTH,
                "line is {} columns: {line}",
                line.chars().count()
            );
        }
    }

    #[test]
    fn short_groups_are_asserted_on_one_line() {
        let statement = assert_group("current_esm", &["bionic".to_owned(), "focal".to_owned()]);
        assert_eq!(
            statement,
            "        assert_eq!(current_esm, [\"bionic\", \"focal\"]);"
        );
    }

    #[test]
    fn long_groups_are_asserted_across_lines() {
        let elements: Vec<String> = (0..12).map(|index| format!("series{index:02}")).collect();
        let statement = assert_group("eol", &elements);
        assert!(statement.starts_with("        assert_eq!(\n"));
        assert!(statement.ends_with("        );"));
        for line in statement.lines() {
            assert!(line.chars().count() <= MAX_WIDTH, "line: {line}");
        }
    }

    #[test]
    fn fill_lines_packs_elements_greedily() {
        let elements: Vec<String> = (0..12)
            .map(|index| format!("\"series{index:02}\""))
            .collect();
        let filled = fill_lines(&elements, 16);
        let lines: Vec<&str> = filled.lines().collect();
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.chars().count() <= MAX_WIDTH, "line: {line}");
        }
        // Every element appears exactly once, in order; only the last element
        // lacks a trailing comma.
        let joined = lines.join(" ");
        for index in 0..12 {
            let token = if index == 11 {
                format!("\"series{index:02}\"")
            } else {
                format!("\"series{index:02}\",")
            };
            assert!(joined.contains(&token));
        }
    }
}

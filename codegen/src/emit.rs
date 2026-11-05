//! Renders `src/series.rs` from a derived [`Dataset`].
//!
//! The output is a pure function of the dataset, so regenerating an
//! up-to-date file is a no-op. The emitted code is formatted the way rustfmt
//! would format it, so `cargo fmt --check` passes without a formatting step.

use std::fmt::{self, Write as _};

use crate::model::{Dataset, Series};
use time::Date;

const TEMPLATE: &str = include_str!("series.rs.template");

/// The maximum line width rustfmt enforces.
const MAX_WIDTH: usize = 100;
/// rustfmt keeps small collections inline up to this width.
const ARRAY_WIDTH: usize = 60;
/// Indentation of the test function bodies in the template.
const TEST_INDENT: usize = 8;

#[derive(Debug)]
pub(crate) enum RenderError {
    Format(fmt::Error),
    InvalidDevelIndex {
        index: usize,
        series_count: usize,
    },
    MissingPhaseSeries,
    NextDayOverflow {
        version: String,
        code_name_short: String,
        field: &'static str,
        date: Date,
    },
    PlaceholderCount {
        placeholder: String,
        count: usize,
    },
    UnfilledPlaceholders,
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(error) => write!(formatter, "formatting generated source failed: {error}"),
            Self::InvalidDevelIndex {
                index,
                series_count,
            } => write!(
                formatter,
                "devel_index {index} is out of bounds for a dataset with {series_count} series"
            ),
            Self::MissingPhaseSeries => write!(
                formatter,
                "the data has no LTS series with legacy support for the lifecycle phase fixture"
            ),
            Self::NextDayOverflow {
                version,
                code_name_short,
                field,
                date,
            } => write!(
                formatter,
                "lifecycle phase series {version} ({code_name_short}): cannot compute the next day after {field} {date}"
            ),
            Self::PlaceholderCount { placeholder, count } => write!(
                formatter,
                "placeholder {placeholder} appears {count} times in the template; expected exactly one occurrence"
            ),
            Self::UnfilledPlaceholders => write!(
                formatter,
                "the template has placeholders that the renderer does not fill"
            ),
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Format(error) => Some(error),
            _ => None,
        }
    }
}

impl From<fmt::Error> for RenderError {
    fn from(error: fmt::Error) -> Self {
        Self::Format(error)
    }
}

/// Renders the complete contents of `src/series.rs`.
///
/// # Errors
///
/// Returns an error if the dataset or template cannot be rendered.
pub(crate) fn render(dataset: &Dataset) -> Result<String, RenderError> {
    render_with_template(dataset, TEMPLATE)
}

fn render_with_template(dataset: &Dataset, template: &str) -> Result<String, RenderError> {
    let devel = dataset
        .series
        .get(dataset.devel_index)
        .ok_or(RenderError::InvalidDevelIndex {
            index: dataset.devel_index,
            series_count: dataset.series.len(),
        })?;
    let phase = phase_series(dataset)?;

    let rendered = template
        .to_owned()
        .replace_unique("@@REFERENCE_DATE@@", &rust_args(dataset.reference_date))?
        .replace_unique("@@SERIES_ENTRIES@@", &series_entries(&dataset.series)?)?
        .replace_unique("@@DEVEL_INDEX@@", &dataset.devel_index.to_string())?
        .replace_unique(
            "@@VERSION_INDEX@@",
            &index_entries(&dataset.series, Index::Version)?,
        )?
        .replace_unique(
            "@@SHORT_NAME_INDEX@@",
            &index_entries(&dataset.series, Index::ShortName)?,
        )?
        .replace_unique(
            "@@FULL_NAME_INDEX@@",
            &index_entries(&dataset.series, Index::FullName)?,
        )?
        .replace_unique("@@DEVEL_SHORT@@", &devel.code_name_short)?
        .replace_unique("@@DEVEL_CODE_NAME@@", &devel.code_name)?
        .replace_unique("@@DEVEL_VERSION@@", &devel.version)?
        .replace_unique(
            "@@ASSERT_ACTIVE@@",
            &assert_group("active", &group(dataset, Group::Active))?,
        )?
        .replace_unique(
            "@@ASSERT_ESM@@",
            &assert_group("esm", &group(dataset, Group::EnteredEsm))?,
        )?
        .replace_unique(
            "@@ASSERT_CURRENT_ESM@@",
            &assert_group("current_esm", &group(dataset, Group::CurrentEsm))?,
        )?
        .replace_unique(
            "@@ASSERT_LEGACY@@",
            &assert_group("legacy", &group(dataset, Group::EnteredLegacy))?,
        )?
        .replace_unique(
            "@@ASSERT_CURRENT_LEGACY@@",
            &assert_group("current_legacy", &group(dataset, Group::CurrentLegacy))?,
        )?
        .replace_unique(
            "@@ASSERT_EOL@@",
            &assert_group("eol", &group(dataset, Group::EndOfLife))?,
        )?
        .replace_unique("@@PHASE_VERSION@@", &phase.version)?
        .replace_unique(
            "@@PHASE_ESM@@",
            &rust_args(next_day(
                phase.end_of_standard_support,
                phase,
                "end_of_standard_support",
            )?),
        )?
        .replace_unique(
            "@@PHASE_EOL@@",
            &rust_args(next_day(phase.end_of_life, phase, "end_of_life")?),
        )?;

    if rendered.contains("@@") {
        return Err(RenderError::UnfilledPlaceholders);
    }
    Ok(rendered)
}

/// Replaces the single occurrence of `placeholder` with `value`.
trait ReplaceUnique {
    fn replace_unique(self, placeholder: &str, value: &str) -> Result<String, RenderError>;
}

impl ReplaceUnique for String {
    fn replace_unique(self, placeholder: &str, value: &str) -> Result<String, RenderError> {
        let count = self.matches(placeholder).count();
        if count != 1 {
            return Err(RenderError::PlaceholderCount {
                placeholder: placeholder.to_owned(),
                count,
            });
        }
        Ok(self.replacen(placeholder, value, 1))
    }
}

/// Which identifier an index map is keyed by.
enum Index {
    Version,
    ShortName,
    FullName,
}

fn index_entries(series: &[Series], index: Index) -> Result<String, fmt::Error> {
    let mut out = String::new();
    for (position, entry) in series.iter().enumerate() {
        let key = match index {
            Index::Version => entry.version.clone(),
            Index::ShortName => entry.code_name_short.clone(),
            Index::FullName => entry.code_name.to_ascii_lowercase(),
        };
        writeln!(out, "    {key:?} => {position},")?;
    }
    Ok(out)
}

fn series_entries(series: &[Series]) -> Result<String, fmt::Error> {
    let mut out = String::new();
    for entry in series {
        writeln!(out, "    SeriesInfo {{")?;
        writeln!(out, "        code_name: {:?},", entry.code_name)?;
        writeln!(out, "        code_name_short: {:?},", entry.code_name_short)?;
        writeln!(out, "        version: {:?},", entry.version)?;
        writeln!(
            out,
            "        release_date: {},",
            rust_expr(entry.release_date)
        )?;
        writeln!(
            out,
            "        end_of_standard_support: {},",
            rust_expr(entry.end_of_standard_support)
        )?;
        writeln!(
            out,
            "        end_of_life: {},",
            rust_expr(entry.end_of_life)
        )?;
        writeln!(out, "        esm_start: {},", optional(&entry.esm_start))?;
        writeln!(
            out,
            "        legacy_start: {},",
            optional(&entry.legacy_start)
        )?;
        writeln!(out, "        is_lts: {},", entry.is_lts)?;
        writeln!(out, "    }},")?;
    }
    Ok(out)
}

fn rust_args(date: Date) -> String {
    format!("{}, {}, {}", date.year(), date.month() as u8, date.day())
}

fn rust_expr(date: Date) -> String {
    format!("Date::new({})", rust_args(date))
}

fn next_day(date: Date, series: &Series, field: &'static str) -> Result<Date, RenderError> {
    date.next_day().ok_or_else(|| RenderError::NextDayOverflow {
        version: series.version.clone(),
        code_name_short: series.code_name_short.clone(),
        field,
        date,
    })
}

fn optional(date: &Option<Date>) -> String {
    match date {
        Some(date) => format!("Some({})", rust_expr(*date)),
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
fn assert_group(name: &str, elements: &[String]) -> Result<String, fmt::Error> {
    let quoted: Vec<String> = elements
        .iter()
        .map(|element| format!("{element:?}"))
        .collect();
    let indent = " ".repeat(TEST_INDENT);

    let single_line = format!("assert_eq!({name}, [{}]);", quoted.join(", "));
    let array_width = quoted.join(", ").len() + 2;
    if TEST_INDENT + single_line.len() <= MAX_WIDTH && array_width <= ARRAY_WIDTH {
        // The template's own newline terminates the line.
        return Ok(format!("{indent}{single_line}"));
    }

    let element_indent = TEST_INDENT + 8;
    let mut out = format!("{indent}assert_eq!(\n");
    writeln!(out, "{indent}    {name},")?;
    writeln!(out, "{indent}    [")?;
    out.push_str(&fill_lines(&quoted, element_indent)?);
    writeln!(out, "{indent}    ]")?;
    write!(out, "{indent});")?;
    Ok(out)
}

/// Packs `elements` into lines of at most [`MAX_WIDTH`] columns, the way
/// rustfmt lays out array literals that do not fit on one line.
fn fill_lines(elements: &[String], indent: usize) -> Result<String, fmt::Error> {
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
            writeln!(out, "{padding}{line}")?;
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&token);
    }

    if !line.is_empty() {
        writeln!(out, "{padding}{line}")?;
    }
    Ok(out)
}

/// Returns the newest released LTS series that has legacy support, falling back
/// to the newest LTS series with legacy support. The generated phase test
/// probes every lifecycle phase on this series.
fn phase_series(dataset: &Dataset) -> Result<&Series, RenderError> {
    lts_with_legacy(dataset)
        .rev()
        .find(|series| series.release_date <= dataset.reference_date)
        .or_else(|| lts_with_legacy(dataset).next_back())
        .ok_or(RenderError::MissingPhaseSeries)
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
        let first = render(&derive(FIXTURE).unwrap()).unwrap();
        let second = render(&derive(FIXTURE).unwrap()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn rendering_replaces_every_placeholder() {
        let rendered = render(&derive(FIXTURE).unwrap()).unwrap();
        assert!(!rendered.contains("@@"));
    }

    #[test]
    fn default_template_rendering_is_byte_exact() {
        let dataset = derive(FIXTURE).unwrap();
        assert_eq!(
            render(&dataset).unwrap(),
            render_with_template(&dataset, TEMPLATE).unwrap()
        );
    }

    #[test]
    fn invalid_devel_index_is_an_error() {
        let mut dataset = derive(FIXTURE).unwrap();
        dataset.devel_index = dataset.series.len();
        let error = render(&dataset).unwrap_err();
        assert!(
            error.to_string().contains("devel_index 3 is out of bounds"),
            "error: {error}"
        );
    }

    #[test]
    fn an_empty_dataset_is_an_error() {
        let mut dataset = derive(FIXTURE).unwrap();
        dataset.series.clear();
        dataset.devel_index = 0;
        let error = render(&dataset).unwrap_err();
        assert!(
            error.to_string().contains("dataset with 0 series"),
            "error: {error}"
        );
    }

    #[test]
    fn a_missing_legacy_phase_fixture_is_an_error() {
        let dataset = derive(&FIXTURE.replace(",2032-04-21,2037-04-30", ",,")).unwrap();
        let error = render(&dataset).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("no LTS series with legacy support"),
            "error: {error}"
        );
    }

    #[test]
    fn a_phase_end_of_life_at_date_max_is_an_error() {
        let mut dataset = derive(FIXTURE).unwrap();
        dataset
            .series
            .iter_mut()
            .find(|series| series.version == "22.04")
            .unwrap()
            .end_of_life = Date::MAX;
        let error = render(&dataset).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("22.04 (jammy)"), "error: {message}");
        assert!(
            message.contains("end_of_life 9999-12-31"),
            "error: {message}"
        );
    }

    #[test]
    fn a_missing_template_placeholder_is_an_error() {
        let dataset = derive(FIXTURE).unwrap();
        let template = TEMPLATE.replacen("@@REFERENCE_DATE@@", "", 1);
        let error = render_with_template(&dataset, &template).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("placeholder @@REFERENCE_DATE@@ appears 0 times"),
            "error: {error}"
        );
    }

    #[test]
    fn a_duplicate_template_placeholder_is_an_error() {
        let dataset = derive(FIXTURE).unwrap();
        let template = TEMPLATE.replacen(
            "@@REFERENCE_DATE@@",
            "@@REFERENCE_DATE@@@@REFERENCE_DATE@@",
            1,
        );
        let error = render_with_template(&dataset, &template).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("placeholder @@REFERENCE_DATE@@ appears 2 times"),
            "error: {error}"
        );
    }

    #[test]
    fn an_unfilled_template_placeholder_is_an_error() {
        let dataset = derive(FIXTURE).unwrap();
        let template = format!("{TEMPLATE}\n@@UNFILLED@@");
        let error = render_with_template(&dataset, &template).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("placeholders that the renderer does not fill"),
            "error: {error}"
        );
    }

    #[test]
    fn lines_fit_the_rustfmt_width() {
        let rendered = render(&derive(FIXTURE).unwrap()).unwrap();
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
        let statement =
            assert_group("current_esm", &["bionic".to_owned(), "focal".to_owned()]).unwrap();
        assert_eq!(
            statement,
            "        assert_eq!(current_esm, [\"bionic\", \"focal\"]);"
        );
    }

    #[test]
    fn long_groups_are_asserted_across_lines() {
        let elements: Vec<String> = (0..12).map(|index| format!("series{index:02}")).collect();
        let statement = assert_group("eol", &elements).unwrap();
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
        let filled = fill_lines(&elements, 16).unwrap();
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

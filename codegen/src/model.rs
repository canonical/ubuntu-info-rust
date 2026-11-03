//! Derives the crate's series data from `distro-info-data`'s `ubuntu.csv`.
//!
//! The upstream file has the columns `version`, `codename`, `series`,
//! `created`, `release`, `eol`, `eol-server`, `eol-esm`, and `eol-legacy`. The
//! columns after `eol` are optional and give the later end dates used by LTS
//! series. They are *end* dates: `eol-esm` is the last day of Expanded Security
//! Maintenance and `eol-legacy` is the last day of Ubuntu Pro legacy support.
//!
//! The derivation rules are:
//!
//! - `version` is the CSV version with the ` LTS` suffix removed, and `is_lts`
//!   is whether the suffix was present.
//! - `release_date` is the `release` column.
//! - `end_of_standard_support` is `eol-server` when present, otherwise `eol`.
//!   For LTS series before 12.04 the server lifecycle is the standard support
//!   period; from 12.04 on the two columns agree.
//! - `end_of_life` is the last end date present: `eol-legacy`, then `eol-esm`,
//!   then `eol-server`, then `eol`.
//! - `esm_start` is the end of standard support, for series that have an
//!   `eol-esm` date: ESM begins when standard support ends.
//! - `legacy_start` is the first day of the month in which ESM ends, for series
//!   that have an `eol-legacy` date: legacy support begins in the month ESM
//!   ends.
//! - `REFERENCE_DATE` is the newest `created` date in the file: the date the
//!   most recent series was added upstream.

use crate::csv;
use crate::date::Date;

/// The columns of `ubuntu.csv`, in order.
const COLUMNS: [&str; 9] = [
    "version",
    "codename",
    "series",
    "created",
    "release",
    "eol",
    "eol-server",
    "eol-esm",
    "eol-legacy",
];

/// One series, derived from a row of `ubuntu.csv`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    /// The full code name, for example `"Warty Warthog"`.
    pub code_name: String,
    /// The short code name used in suites, for example `"warty"`.
    pub code_name_short: String,
    /// The version without the LTS suffix, for example `"4.10"`.
    pub version: String,
    /// The date of the initial release.
    pub release_date: Date,
    /// The last day of standard support.
    pub end_of_standard_support: Date,
    /// The last day of support, including ESM and legacy support.
    pub end_of_life: Date,
    /// The first day of Expanded Security Maintenance, if the series has ESM.
    pub esm_start: Option<Date>,
    /// The first day of Ubuntu Pro legacy support, if the series has it.
    pub legacy_start: Option<Date>,
    /// Whether the series is a Long Term Support (LTS) series.
    pub is_lts: bool,
}

/// The complete dataset rendered into `src/series.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset {
    /// The date the most recent series was created upstream.
    pub reference_date: Date,
    /// Every series, ordered by release date.
    pub series: Vec<Series>,
    /// Index into `series` of the earliest future release.
    pub devel_index: usize,
}

/// A raw `ubuntu.csv` row: the derived series plus the `created` date, which is
/// only used to compute `REFERENCE_DATE`.
#[derive(Debug, Clone)]
struct Row {
    created: Date,
    series: Series,
}

/// Derives the dataset from the contents of `ubuntu.csv`.
pub fn derive(csv_text: &str) -> Result<Dataset, String> {
    let records = csv::parse(csv_text)?;
    let header = records.first().ok_or("ubuntu.csv is empty")?;
    if header.len() != COLUMNS.len() || header.iter().zip(COLUMNS).any(|(got, want)| got != want) {
        return Err(format!(
            "unexpected ubuntu.csv header: expected {}",
            COLUMNS.join(",")
        ));
    }

    let mut rows = Vec::new();
    for record in &records[1..] {
        rows.push(derive_row(record)?);
    }

    rows.sort_by(|a, b| a.series.release_date.cmp(&b.series.release_date));
    for pair in rows.windows(2) {
        if pair[0].series.release_date == pair[1].series.release_date {
            return Err(format!(
                "{} and {} share the release date {}; series must have unique release dates",
                pair[0].series.code_name_short,
                pair[1].series.code_name_short,
                pair[0].series.release_date
            ));
        }
    }

    validate_identifiers(&rows)?;

    let reference_date = rows
        .iter()
        .map(|row| row.created)
        .max()
        .ok_or("ubuntu.csv contains no data rows")?;

    let series: Vec<Series> = rows.into_iter().map(|row| row.series).collect();
    let devel_index = series
        .iter()
        .position(|series| series.release_date > reference_date)
        .ok_or(
            "no series has a release date after the reference date; \
             cannot determine the development series",
        )?;

    Ok(Dataset {
        reference_date,
        series,
        devel_index,
    })
}

fn derive_row(record: &[String]) -> Result<Row, String> {
    let field = |index: usize| record.get(index).map(String::as_str).unwrap_or("");
    let version_raw = field(0);
    let code_name = field(1);
    let code_name_short = field(2);
    let created = field(3);
    let release = field(4);
    let eol = field(5);
    let eol_server = field(6);
    let eol_esm = field(7);
    let eol_legacy = field(8);

    let context = format!(
        "series {} ({})",
        if version_raw.is_empty() {
            code_name
        } else {
            version_raw
        },
        code_name_short
    );

    let (version, is_lts) = match version_raw.strip_suffix(" LTS") {
        Some(version) => (version, true),
        None => (version_raw, false),
    };
    if version.is_empty() {
        return Err(format!("{context}: missing version column"));
    }
    require(code_name, "codename", &context)?;
    require(code_name_short, "series", &context)?;
    if !code_name_short
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(format!(
            "{context}: the series name must be lowercase letters and digits; \
             suite parsing splits on hyphens"
        ));
    }

    let created = Date::parse(require(created, "created", &context)?)
        .map_err(|error| format!("{context}: {error}"))?;
    let release_date = Date::parse(require(release, "release", &context)?)
        .map_err(|error| format!("{context}: {error}"))?;
    let eol = Date::parse(require(eol, "eol", &context)?)
        .map_err(|error| format!("{context}: {error}"))?;
    let eol_server = optional_date(eol_server, &context, "eol-server")?;
    let eol_esm = optional_date(eol_esm, &context, "eol-esm")?;
    let eol_legacy = optional_date(eol_legacy, &context, "eol-legacy")?;

    if eol_legacy.is_some() && eol_esm.is_none() {
        return Err(format!(
            "{context}: eol-legacy is set without eol-esm; legacy support follows ESM"
        ));
    }

    let end_of_standard_support = eol_server.unwrap_or(eol);
    let end_of_life = eol_legacy.or(eol_esm).or(eol_server).unwrap_or(eol);
    let esm_start = eol_esm.map(|_| end_of_standard_support);
    let legacy_start = eol_legacy.is_some().then(|| {
        eol_esm
            .expect("eol-legacy requires eol-esm")
            .first_of_month()
    });

    if release_date > end_of_standard_support {
        return Err(format!(
            "{context}: release date {release_date} is after the end of standard support"
        ));
    }
    if end_of_standard_support > end_of_life {
        return Err(format!(
            "{context}: end of standard support is after the end of life"
        ));
    }
    if let Some(legacy_start) = legacy_start
        && legacy_start > end_of_life
    {
        return Err(format!(
            "{context}: legacy support starts after the end of life"
        ));
    }
    if let (Some(esm_start), Some(legacy_start)) = (esm_start, legacy_start)
        && esm_start >= legacy_start
    {
        return Err(format!(
            "{context}: ESM would end before it starts; check the eol-server and eol-esm dates"
        ));
    }

    Ok(Row {
        created,
        series: Series {
            code_name: code_name.to_owned(),
            code_name_short: code_name_short.to_owned(),
            version: version.to_owned(),
            release_date,
            end_of_standard_support,
            end_of_life,
            esm_start,
            legacy_start,
            is_lts,
        },
    })
}

fn require<'a>(value: &'a str, column: &str, context: &str) -> Result<&'a str, String> {
    if value.is_empty() {
        Err(format!("{context}: missing {column} column"))
    } else {
        Ok(value)
    }
}

fn optional_date(value: &str, context: &str, column: &str) -> Result<Option<Date>, String> {
    if value.is_empty() {
        return Ok(None);
    }
    Date::parse(value)
        .map(Some)
        .map_err(|error| format!("{context}: {column}: {error}"))
}

fn validate_identifiers(rows: &[Row]) -> Result<(), String> {
    let mut versions = std::collections::HashSet::new();
    let mut short_names = std::collections::HashSet::new();
    let mut full_names = std::collections::HashSet::new();

    for row in rows {
        let series = &row.series;
        if !versions.insert(series.version.as_str()) {
            return Err(format!("duplicate version {}", series.version));
        }
        if !short_names.insert(series.code_name_short.as_str()) {
            return Err(format!("duplicate series name {}", series.code_name_short));
        }
        if !full_names.insert(series.code_name.as_str()) {
            return Err(format!("duplicate code name {}", series.code_name));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30
22.04 LTS,Jammy Jellyfish,jammy,2021-10-14,2022-04-21,2027-06-01,2027-06-01,2032-04-21,2037-04-30
27.04,Tenacious T-Rex,tenacious,2026-10-16,2027-04-22,2028-01-20
";

    #[test]
    fn derives_a_non_lts_series() {
        let dataset = derive(FIXTURE).unwrap();
        let warty = &dataset.series[0];
        assert_eq!(warty.code_name, "Warty Warthog");
        assert_eq!(warty.code_name_short, "warty");
        assert_eq!(warty.version, "4.10");
        assert_eq!(warty.release_date.to_string(), "2004-10-20");
        assert_eq!(warty.end_of_standard_support.to_string(), "2006-04-30");
        assert_eq!(warty.end_of_life.to_string(), "2006-04-30");
        assert_eq!(warty.esm_start, None);
        assert_eq!(warty.legacy_start, None);
        assert!(!warty.is_lts);
    }

    #[test]
    fn derives_an_lts_series_with_all_phases() {
        let dataset = derive(FIXTURE).unwrap();
        let jammy = &dataset.series[1];
        assert_eq!(jammy.version, "22.04");
        assert!(jammy.is_lts);
        assert_eq!(jammy.end_of_standard_support.to_string(), "2027-06-01");
        assert_eq!(jammy.end_of_life.to_string(), "2037-04-30");
        assert_eq!(
            jammy.esm_start.map(|date| date.to_string()),
            Some("2027-06-01".to_owned())
        );
        assert_eq!(
            jammy.legacy_start.map(|date| date.to_string()),
            Some("2032-04-01".to_owned())
        );
    }

    #[test]
    fn orders_series_by_release_date() {
        let dataset = derive(FIXTURE).unwrap();
        let versions: Vec<&str> = dataset.series.iter().map(|s| s.version.as_str()).collect();
        assert_eq!(versions, ["4.10", "22.04", "27.04"]);
    }

    #[test]
    fn reference_date_is_the_newest_created_date() {
        let dataset = derive(FIXTURE).unwrap();
        assert_eq!(dataset.reference_date.to_string(), "2026-10-16");
    }

    #[test]
    fn devel_is_the_earliest_future_release() {
        let dataset = derive(FIXTURE).unwrap();
        assert_eq!(dataset.devel_index, 2);
        assert_eq!(dataset.series[2].code_name_short, "tenacious");
    }

    #[test]
    fn lts_without_esm_uses_the_server_lifecycle() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
6.06 LTS,Dapper Drake,dapper,2005-10-12,2006-06-01,2009-07-14,2011-06-01
";
        let dataset = derive(csv).unwrap();
        let dapper = &dataset.series[0];
        assert!(dapper.is_lts);
        assert_eq!(dapper.end_of_standard_support.to_string(), "2011-06-01");
        assert_eq!(dapper.end_of_life.to_string(), "2011-06-01");
        assert_eq!(dapper.esm_start, None);
        assert_eq!(dapper.legacy_start, None);
    }

    #[test]
    fn rejects_a_header_mismatch() {
        let csv = "version,codename\n4.10,Warty Warthog\n";
        assert!(derive(csv).is_err());
    }

    #[test]
    fn rejects_a_file_without_data_rows() {
        let csv = "version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy\n";
        let error = derive(csv).unwrap_err();
        assert!(error.contains("no data rows"), "error: {error}");
    }

    #[test]
    fn rejects_missing_required_columns() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2004-10-20
";
        assert!(derive(csv).is_err());
    }

    #[test]
    fn rejects_duplicate_series() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30
4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30
";
        assert!(derive(csv).is_err());
    }

    #[test]
    fn rejects_shared_release_dates() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2004-10-20,2006-04-30
5.04,Hoary Hedgehog,hoary,2004-10-20,2004-10-20,2006-10-31
";
        assert!(derive(csv).is_err());
    }

    #[test]
    fn rejects_legacy_without_esm() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
22.04 LTS,Jammy Jellyfish,jammy,2021-10-14,2022-04-21,2027-06-01,2027-06-01,,2037-04-30
";
        assert!(derive(csv).is_err());
    }

    #[test]
    fn rejects_an_invalid_date() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2004-13-20,2006-04-30
";
        assert!(derive(csv).is_err());
    }

    #[test]
    fn rejects_an_uppercase_series_name() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,Warty,2004-03-05,2004-10-20,2006-04-30
";
        assert!(derive(csv).is_err());
    }

    #[test]
    fn rejects_a_release_after_standard_support() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2004-03-05,2007-10-20,2006-04-30
";
        assert!(derive(csv).is_err());
    }

    #[test]
    fn rejects_a_dataset_without_a_future_release() {
        let csv = "\
version,codename,series,created,release,eol,eol-server,eol-esm,eol-legacy
4.10,Warty Warthog,warty,2006-05-05,2004-10-20,2006-04-30
";
        assert!(derive(csv).is_err());
    }
}

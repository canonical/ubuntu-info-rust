use std::cmp::Ordering;
use std::fmt;

use phf::phf_map;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::date::Date;

/// The date against which the "current" iterators are evaluated.
///
/// The crate contains static data and performs no I/O, so it cannot discover the
/// current date at runtime. This constant is the date on which the series data
/// was last verified. Use the `*_on` methods to evaluate a lifecycle phase on a
/// different date.
pub const REFERENCE_DATE: Date = Date::new(2026, 10, 20);

/// Information about an Ubuntu series.
///
/// A series is the set of packages targeting one Ubuntu version. It is not the
/// same as a release: an LTS series has an initial release plus point releases,
/// all of which belong to the same series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SeriesInfo {
    /// The full code name, for example `"Stonking Stingray"`.
    pub code_name: &'static str,
    /// The short code name used in suites, for example `"stonking"`.
    pub code_name_short: &'static str,
    /// The version, for example `"26.10"`.
    pub version: &'static str,
    /// The date of the initial release.
    pub release_date: Date,
    /// The last day of standard support.
    pub end_of_standard_support: Date,
    /// The last day of support, including ESM and legacy support where applicable.
    pub end_of_life: Date,
    /// The first day of Expanded Security Maintenance, if the series has ESM.
    pub esm_start: Option<Date>,
    /// The first day of Ubuntu Pro legacy support, if the series has it.
    pub legacy_start: Option<Date>,
    /// Whether the series is a Long Term Support (LTS) series.
    pub is_lts: bool,
}

/// The lifecycle phase of a series on a given date.
///
/// Variants are ordered from development through end of life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Lifecycle {
    /// The series has not been released yet and is in development.
    Development,
    /// The series is within its standard support period.
    StandardSupport,
    /// The series is receiving Expanded Security Maintenance.
    ExpandedSecurityMaintenance,
    /// The series is receiving Ubuntu Pro legacy support.
    LegacySupport,
    /// The series is no longer supported.
    EndOfLife,
}

/// The owned representation used to deserialize [`SeriesInfo`].
///
/// [`SeriesInfo`] stores its identifiers as `&'static str`, which serde cannot
/// deserialize directly. Deserialization resolves the series against the
/// built-in [`SERIES`] data instead of allocating new strings, so the input must
/// describe a known series exactly.
#[cfg(feature = "serde")]
#[derive(Deserialize)]
struct SeriesInfoOwned {
    code_name: String,
    code_name_short: String,
    version: String,
    release_date: Date,
    end_of_standard_support: Date,
    end_of_life: Date,
    esm_start: Option<Date>,
    legacy_start: Option<Date>,
    is_lts: bool,
}

#[cfg(feature = "serde")]
impl TryFrom<SeriesInfoOwned> for SeriesInfo {
    type Error = String;

    fn try_from(owned: SeriesInfoOwned) -> Result<Self, Self::Error> {
        let series = SeriesInfo::from_short_name(&owned.code_name_short)
            .ok_or_else(|| format!("unknown Ubuntu series: {}", owned.code_name_short))?;

        let matches = owned.code_name == series.code_name
            && owned.version == series.version
            && owned.release_date == series.release_date
            && owned.end_of_standard_support == series.end_of_standard_support
            && owned.end_of_life == series.end_of_life
            && owned.esm_start == series.esm_start
            && owned.legacy_start == series.legacy_start
            && owned.is_lts == series.is_lts;

        if matches {
            Ok(*series)
        } else {
            Err(format!(
                "series data for {} does not match the built-in data",
                owned.code_name_short
            ))
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SeriesInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = SeriesInfoOwned::deserialize(deserializer)?;
        SeriesInfo::try_from(owned).map_err(serde::de::Error::custom)
    }
}

impl PartialOrd for SeriesInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SeriesInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare release dates first. The remaining fields are tie-breakers so
        // that the ordering is consistent with the derived `Eq` implementation.
        (
            self.release_date,
            self.version,
            self.code_name_short,
            self.code_name,
            self.end_of_standard_support,
            self.end_of_life,
            self.esm_start,
            self.legacy_start,
            self.is_lts,
        )
            .cmp(&(
                other.release_date,
                other.version,
                other.code_name_short,
                other.code_name,
                other.end_of_standard_support,
                other.end_of_life,
                other.esm_start,
                other.legacy_start,
                other.is_lts,
            ))
    }
}

impl SeriesInfo {
    /// Returns every known series, ordered by release date.
    pub fn all() -> impl Iterator<Item = &'static SeriesInfo> {
        SERIES.iter()
    }

    /// Looks up a series by version.
    ///
    /// Accepts a plain version (`"26.04"`), an LTS suffix (`"26.04 LTS"`), an
    /// optional `Ubuntu` prefix (`"Ubuntu 26.04 LTS"`), and point releases
    /// (`"24.04.1 LTS"`), which resolve to their series. An LTS suffix or point
    /// release is rejected for a series that is not an LTS.
    pub fn from_version(input: &str) -> Option<&'static SeriesInfo> {
        let input = input.trim();
        let input = strip_prefix_ignore_ascii_case(input, "Ubuntu ").unwrap_or(input);
        let (input, had_lts_suffix) = match strip_suffix_ignore_ascii_case(input, " LTS") {
            Some(value) => (value, true),
            None => (input, false),
        };

        let mut parts = input.split('.');
        let major = parts.next()?;
        let minor = parts.next()?;
        if !is_numeric(major) || !is_numeric(minor) {
            return None;
        }

        let mut had_point_release = false;
        if let Some(point_release) = parts.next() {
            if !is_numeric(point_release) || parts.next().is_some() {
                return None;
            }
            had_point_release = true;
        }

        let version = format!("{major}.{minor}");
        let index = *VERSION_INDEX.get(version.as_str())?;
        let series = &SERIES[index];

        if (had_lts_suffix || had_point_release) && !series.is_lts {
            return None;
        }

        Some(series)
    }

    /// Looks up a series by code name.
    ///
    /// Accepts both the full code name (`"Stonking Stingray"`) and the short code
    /// name (`"stonking"`). Matching is ASCII case-insensitive. The special name
    /// `"devel"` resolves to the series currently in development.
    pub fn from_code_name(input: &str) -> Option<&'static SeriesInfo> {
        let input = input.trim();
        if input.eq_ignore_ascii_case("devel") {
            return Some(Self::devel());
        }
        let lower = input.to_ascii_lowercase();
        let index = SHORT_NAME_INDEX
            .get(lower.as_str())
            .or_else(|| FULL_NAME_INDEX.get(lower.as_str()))?;
        Some(&SERIES[*index])
    }

    /// Looks up a series by its short code name.
    ///
    /// Matching is ASCII case-insensitive. The special name `"devel"` resolves to
    /// the series currently in development.
    pub fn from_short_name(input: &str) -> Option<&'static SeriesInfo> {
        let input = input.trim();
        if input.eq_ignore_ascii_case("devel") {
            return Some(Self::devel());
        }
        let lower = input.to_ascii_lowercase();
        let index = *SHORT_NAME_INDEX.get(lower.as_str())?;
        Some(&SERIES[index])
    }

    /// Returns the series currently in development, also called the future
    /// release.
    ///
    /// This is the unreleased series with the earliest release date relative to
    /// [`REFERENCE_DATE`].
    pub fn devel() -> &'static SeriesInfo {
        &SERIES[DEVEL_INDEX]
    }

    /// Returns the series that are currently supported, including standard
    /// support, ESM, and legacy support.
    ///
    /// The future release is not included.
    pub fn active() -> impl Iterator<Item = &'static SeriesInfo> {
        SERIES
            .iter()
            .filter(|series| series.is_active_on(REFERENCE_DATE))
    }

    /// Returns the series that have entered Expanded Security Maintenance.
    ///
    /// This includes series that have since moved on to legacy support or end of
    /// life, matching the upstream ESM table. Use [`SeriesInfo::current_esm`] for
    /// the series currently in the ESM phase.
    pub fn esm() -> impl Iterator<Item = &'static SeriesInfo> {
        SERIES.iter().filter(|series| {
            series
                .esm_start
                .is_some_and(|start| start <= REFERENCE_DATE)
        })
    }

    /// Returns the series currently receiving Expanded Security Maintenance.
    pub fn current_esm() -> impl Iterator<Item = &'static SeriesInfo> {
        SERIES.iter().filter(|series| {
            series.lifecycle_on(REFERENCE_DATE) == Lifecycle::ExpandedSecurityMaintenance
        })
    }

    /// Returns the series that have entered Ubuntu Pro legacy support.
    ///
    /// Use [`SeriesInfo::current_legacy`] for the series currently in the legacy
    /// support phase.
    pub fn legacy() -> impl Iterator<Item = &'static SeriesInfo> {
        SERIES.iter().filter(|series| {
            series
                .legacy_start
                .is_some_and(|start| start <= REFERENCE_DATE)
        })
    }

    /// Returns the series currently receiving Ubuntu Pro legacy support.
    pub fn current_legacy() -> impl Iterator<Item = &'static SeriesInfo> {
        SERIES
            .iter()
            .filter(|series| series.lifecycle_on(REFERENCE_DATE) == Lifecycle::LegacySupport)
    }

    /// Returns the series that have reached end of life.
    pub fn end_of_life() -> impl Iterator<Item = &'static SeriesInfo> {
        SERIES
            .iter()
            .filter(|series| series.lifecycle_on(REFERENCE_DATE) == Lifecycle::EndOfLife)
    }

    /// Returns the lifecycle phase of this series on `date`.
    ///
    /// Dates are inclusive: a series remains in a phase through the last day of
    /// that phase.
    pub fn lifecycle_on(&self, date: Date) -> Lifecycle {
        if date < self.release_date {
            Lifecycle::Development
        } else if date <= self.end_of_standard_support {
            Lifecycle::StandardSupport
        } else if date > self.end_of_life {
            Lifecycle::EndOfLife
        } else if self.legacy_start.is_some_and(|start| date >= start) {
            Lifecycle::LegacySupport
        } else if self.esm_start.is_some_and(|start| date >= start) {
            Lifecycle::ExpandedSecurityMaintenance
        } else {
            Lifecycle::EndOfLife
        }
    }

    /// Returns `true` if the series is supported on `date`, including standard
    /// support, ESM, and legacy support.
    pub fn is_active_on(&self, date: Date) -> bool {
        self.release_date <= date && date <= self.end_of_life
    }

    /// Returns `true` if the series is receiving ESM on `date`.
    pub fn is_esm_on(&self, date: Date) -> bool {
        self.lifecycle_on(date) == Lifecycle::ExpandedSecurityMaintenance
    }

    /// Returns `true` if the series is receiving legacy support on `date`.
    pub fn is_legacy_on(&self, date: Date) -> bool {
        self.lifecycle_on(date) == Lifecycle::LegacySupport
    }

    /// Returns `true` if the series has reached end of life on `date`.
    pub fn is_end_of_life_on(&self, date: Date) -> bool {
        self.lifecycle_on(date) == Lifecycle::EndOfLife
    }
}

impl fmt::Display for SeriesInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code_name_short)
    }
}

fn is_numeric(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn strip_prefix_ignore_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    if value.len() >= prefix.len() && value[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&value[prefix.len()..])
    } else {
        None
    }
}

fn strip_suffix_ignore_ascii_case<'a>(value: &'a str, suffix: &str) -> Option<&'a str> {
    if value.len() >= suffix.len()
        && value[value.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
    {
        Some(&value[..value.len() - suffix.len()])
    } else {
        None
    }
}

/// Every known Ubuntu series, ordered by release date.
///
/// The data is current as of [`REFERENCE_DATE`]. Where an upstream source only
/// specifies a month, start dates use the first day of the month and end dates
/// use the last day of the month.
pub static SERIES: &[SeriesInfo] = &[
    SeriesInfo {
        code_name: "Warty Warthog",
        code_name_short: "warty",
        version: "4.10",
        release_date: Date::new(2004, 10, 26),
        end_of_standard_support: Date::new(2006, 4, 30),
        end_of_life: Date::new(2006, 4, 30),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Hoary Hedgehog",
        code_name_short: "hoary",
        version: "5.04",
        release_date: Date::new(2005, 4, 8),
        end_of_standard_support: Date::new(2006, 10, 31),
        end_of_life: Date::new(2006, 10, 31),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Breezy Badger",
        code_name_short: "breezy",
        version: "5.10",
        release_date: Date::new(2005, 10, 12),
        end_of_standard_support: Date::new(2007, 4, 13),
        end_of_life: Date::new(2007, 4, 13),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Dapper Drake",
        code_name_short: "dapper",
        version: "6.06",
        release_date: Date::new(2006, 6, 1),
        end_of_standard_support: Date::new(2011, 6, 1),
        end_of_life: Date::new(2011, 6, 1),
        esm_start: None,
        legacy_start: None,
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Edgy Eft",
        code_name_short: "edgy",
        version: "6.10",
        release_date: Date::new(2006, 10, 26),
        end_of_standard_support: Date::new(2008, 4, 26),
        end_of_life: Date::new(2008, 4, 26),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Feisty Fawn",
        code_name_short: "feisty",
        version: "7.04",
        release_date: Date::new(2007, 4, 19),
        end_of_standard_support: Date::new(2008, 10, 19),
        end_of_life: Date::new(2008, 10, 19),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Gutsy Gibbon",
        code_name_short: "gutsy",
        version: "7.10",
        release_date: Date::new(2007, 10, 18),
        end_of_standard_support: Date::new(2009, 4, 18),
        end_of_life: Date::new(2009, 4, 18),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Hardy Heron",
        code_name_short: "hardy",
        version: "8.04",
        release_date: Date::new(2008, 4, 24),
        end_of_standard_support: Date::new(2013, 5, 9),
        end_of_life: Date::new(2013, 5, 9),
        esm_start: None,
        legacy_start: None,
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Intrepid Ibex",
        code_name_short: "intrepid",
        version: "8.10",
        release_date: Date::new(2008, 10, 30),
        end_of_standard_support: Date::new(2010, 4, 30),
        end_of_life: Date::new(2010, 4, 30),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Jaunty Jackalope",
        code_name_short: "jaunty",
        version: "9.04",
        release_date: Date::new(2009, 4, 23),
        end_of_standard_support: Date::new(2010, 10, 23),
        end_of_life: Date::new(2010, 10, 23),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Karmic Koala",
        code_name_short: "karmic",
        version: "9.10",
        release_date: Date::new(2009, 10, 29),
        end_of_standard_support: Date::new(2011, 4, 30),
        end_of_life: Date::new(2011, 4, 30),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Lucid Lynx",
        code_name_short: "lucid",
        version: "10.04",
        release_date: Date::new(2010, 4, 29),
        end_of_standard_support: Date::new(2015, 4, 30),
        end_of_life: Date::new(2015, 4, 30),
        esm_start: None,
        legacy_start: None,
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Maverick Meerkat",
        code_name_short: "maverick",
        version: "10.10",
        release_date: Date::new(2010, 10, 10),
        end_of_standard_support: Date::new(2012, 4, 10),
        end_of_life: Date::new(2012, 4, 10),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Natty Narwhal",
        code_name_short: "natty",
        version: "11.04",
        release_date: Date::new(2011, 4, 28),
        end_of_standard_support: Date::new(2012, 10, 28),
        end_of_life: Date::new(2012, 10, 28),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Oneiric Ocelot",
        code_name_short: "oneiric",
        version: "11.10",
        release_date: Date::new(2011, 10, 13),
        end_of_standard_support: Date::new(2013, 5, 9),
        end_of_life: Date::new(2013, 5, 9),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Precise Pangolin",
        code_name_short: "precise",
        version: "12.04",
        release_date: Date::new(2012, 4, 26),
        end_of_standard_support: Date::end_of_month(2017, 4),
        end_of_life: Date::end_of_month(2019, 4),
        esm_start: Some(Date::new(2017, 4, 28)),
        legacy_start: None,
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Quantal Quetzal",
        code_name_short: "quantal",
        version: "12.10",
        release_date: Date::new(2012, 10, 18),
        end_of_standard_support: Date::new(2014, 5, 16),
        end_of_life: Date::new(2014, 5, 16),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Raring Ringtail",
        code_name_short: "raring",
        version: "13.04",
        release_date: Date::new(2013, 4, 25),
        end_of_standard_support: Date::new(2014, 1, 27),
        end_of_life: Date::new(2014, 1, 27),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Saucy Salamander",
        code_name_short: "saucy",
        version: "13.10",
        release_date: Date::new(2013, 10, 17),
        end_of_standard_support: Date::new(2014, 7, 17),
        end_of_life: Date::new(2014, 7, 17),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Trusty Tahr",
        code_name_short: "trusty",
        version: "14.04",
        release_date: Date::new(2014, 4, 17),
        end_of_standard_support: Date::end_of_month(2019, 4),
        end_of_life: Date::end_of_month(2029, 4),
        esm_start: Some(Date::new(2019, 4, 25)),
        legacy_start: Some(Date::new(2024, 4, 1)),
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Utopic Unicorn",
        code_name_short: "utopic",
        version: "14.10",
        release_date: Date::new(2014, 10, 23),
        end_of_standard_support: Date::new(2015, 7, 23),
        end_of_life: Date::new(2015, 7, 23),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Vivid Vervet",
        code_name_short: "vivid",
        version: "15.04",
        release_date: Date::new(2015, 4, 23),
        end_of_standard_support: Date::new(2016, 2, 4),
        end_of_life: Date::new(2016, 2, 4),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Wily Werewolf",
        code_name_short: "wily",
        version: "15.10",
        release_date: Date::new(2015, 10, 22),
        end_of_standard_support: Date::new(2016, 7, 28),
        end_of_life: Date::new(2016, 7, 28),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Xenial Xerus",
        code_name_short: "xenial",
        version: "16.04",
        release_date: Date::new(2016, 4, 21),
        end_of_standard_support: Date::end_of_month(2021, 4),
        end_of_life: Date::end_of_month(2031, 4),
        esm_start: Some(Date::new(2021, 4, 1)),
        legacy_start: Some(Date::new(2026, 4, 1)),
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Yakkety Yak",
        code_name_short: "yakkety",
        version: "16.10",
        release_date: Date::new(2016, 10, 13),
        end_of_standard_support: Date::new(2017, 7, 20),
        end_of_life: Date::new(2017, 7, 20),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Zesty Zapus",
        code_name_short: "zesty",
        version: "17.04",
        release_date: Date::new(2017, 4, 13),
        end_of_standard_support: Date::new(2018, 1, 13),
        end_of_life: Date::new(2018, 1, 13),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Artful Aardvark",
        code_name_short: "artful",
        version: "17.10",
        release_date: Date::new(2017, 10, 19),
        end_of_standard_support: Date::new(2018, 7, 19),
        end_of_life: Date::new(2018, 7, 19),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Bionic Beaver",
        code_name_short: "bionic",
        version: "18.04",
        release_date: Date::new(2018, 4, 26),
        end_of_standard_support: Date::end_of_month(2023, 6),
        end_of_life: Date::end_of_month(2033, 4),
        esm_start: Some(Date::new(2023, 6, 1)),
        legacy_start: Some(Date::new(2028, 4, 1)),
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Cosmic Cuttlefish",
        code_name_short: "cosmic",
        version: "18.10",
        release_date: Date::new(2018, 10, 18),
        end_of_standard_support: Date::new(2019, 7, 18),
        end_of_life: Date::new(2019, 7, 18),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Disco Dingo",
        code_name_short: "disco",
        version: "19.04",
        release_date: Date::new(2019, 4, 18),
        end_of_standard_support: Date::new(2020, 1, 23),
        end_of_life: Date::new(2020, 1, 23),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Eoan Ermine",
        code_name_short: "eoan",
        version: "19.10",
        release_date: Date::new(2019, 10, 17),
        end_of_standard_support: Date::new(2020, 7, 17),
        end_of_life: Date::new(2020, 7, 17),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Focal Fossa",
        code_name_short: "focal",
        version: "20.04",
        release_date: Date::new(2020, 4, 23),
        end_of_standard_support: Date::end_of_month(2025, 5),
        end_of_life: Date::end_of_month(2035, 4),
        esm_start: Some(Date::new(2025, 6, 1)),
        legacy_start: Some(Date::new(2030, 4, 1)),
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Groovy Gorilla",
        code_name_short: "groovy",
        version: "20.10",
        release_date: Date::new(2020, 10, 22),
        end_of_standard_support: Date::new(2021, 7, 22),
        end_of_life: Date::new(2021, 7, 22),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Hirsute Hippo",
        code_name_short: "hirsute",
        version: "21.04",
        release_date: Date::new(2021, 4, 22),
        end_of_standard_support: Date::new(2022, 1, 20),
        end_of_life: Date::new(2022, 1, 20),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Impish Indri",
        code_name_short: "impish",
        version: "21.10",
        release_date: Date::new(2021, 10, 14),
        end_of_standard_support: Date::new(2022, 7, 14),
        end_of_life: Date::new(2022, 7, 14),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Jammy Jellyfish",
        code_name_short: "jammy",
        version: "22.04",
        release_date: Date::new(2022, 4, 21),
        end_of_standard_support: Date::end_of_month(2027, 6),
        end_of_life: Date::end_of_month(2037, 4),
        esm_start: Some(Date::new(2027, 6, 1)),
        legacy_start: Some(Date::new(2032, 4, 1)),
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Kinetic Kudu",
        code_name_short: "kinetic",
        version: "22.10",
        release_date: Date::new(2022, 10, 20),
        end_of_standard_support: Date::new(2023, 7, 20),
        end_of_life: Date::new(2023, 7, 20),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Lunar Lobster",
        code_name_short: "lunar",
        version: "23.04",
        release_date: Date::new(2023, 4, 20),
        end_of_standard_support: Date::new(2024, 1, 25),
        end_of_life: Date::new(2024, 1, 25),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Mantic Minotaur",
        code_name_short: "mantic",
        version: "23.10",
        release_date: Date::new(2023, 10, 12),
        end_of_standard_support: Date::new(2024, 7, 11),
        end_of_life: Date::new(2024, 7, 11),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Noble Numbat",
        code_name_short: "noble",
        version: "24.04",
        release_date: Date::new(2024, 4, 25),
        end_of_standard_support: Date::end_of_month(2029, 6),
        end_of_life: Date::end_of_month(2039, 4),
        esm_start: Some(Date::new(2029, 6, 1)),
        legacy_start: Some(Date::new(2034, 4, 1)),
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Oracular Oriole",
        code_name_short: "oracular",
        version: "24.10",
        release_date: Date::new(2024, 10, 10),
        end_of_standard_support: Date::new(2025, 7, 10),
        end_of_life: Date::new(2025, 7, 10),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Plucky Puffin",
        code_name_short: "plucky",
        version: "25.04",
        release_date: Date::new(2025, 4, 17),
        end_of_standard_support: Date::new(2026, 1, 15),
        end_of_life: Date::new(2026, 1, 15),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Questing Quokka",
        code_name_short: "questing",
        version: "25.10",
        release_date: Date::new(2025, 10, 9),
        end_of_standard_support: Date::end_of_month(2026, 7),
        end_of_life: Date::end_of_month(2026, 7),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Resolute Raccoon",
        code_name_short: "resolute",
        version: "26.04",
        release_date: Date::new(2026, 4, 23),
        end_of_standard_support: Date::end_of_month(2031, 5),
        end_of_life: Date::end_of_month(2041, 4),
        esm_start: Some(Date::new(2031, 5, 1)),
        legacy_start: Some(Date::new(2036, 4, 1)),
        is_lts: true,
    },
    SeriesInfo {
        code_name: "Stonking Stingray",
        code_name_short: "stonking",
        version: "26.10",
        release_date: Date::new(2026, 10, 15),
        end_of_standard_support: Date::end_of_month(2027, 7),
        end_of_life: Date::end_of_month(2027, 7),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
    SeriesInfo {
        code_name: "Tenacious T-Rex",
        code_name_short: "tenacious",
        version: "27.04",
        release_date: Date::year_month(2027, 4),
        end_of_standard_support: Date::end_of_month(2028, 1),
        end_of_life: Date::end_of_month(2028, 1),
        esm_start: None,
        legacy_start: None,
        is_lts: false,
    },
];

/// Index of the series currently in development.
///
/// Kept as a constant so that [`SeriesInfo::devel`] is O(1). The
/// `devel_index_is_correct` test verifies that it points at the earliest future
/// release.
const DEVEL_INDEX: usize = 45;

/// Version to index in [`SERIES`].
///
/// A compile-time perfect hash map keeps lookups O(1) in the number of series.
static VERSION_INDEX: phf::Map<&'static str, usize> = phf_map! {
    "4.10" => 0,
    "5.04" => 1,
    "5.10" => 2,
    "6.06" => 3,
    "6.10" => 4,
    "7.04" => 5,
    "7.10" => 6,
    "8.04" => 7,
    "8.10" => 8,
    "9.04" => 9,
    "9.10" => 10,
    "10.04" => 11,
    "10.10" => 12,
    "11.04" => 13,
    "11.10" => 14,
    "12.04" => 15,
    "12.10" => 16,
    "13.04" => 17,
    "13.10" => 18,
    "14.04" => 19,
    "14.10" => 20,
    "15.04" => 21,
    "15.10" => 22,
    "16.04" => 23,
    "16.10" => 24,
    "17.04" => 25,
    "17.10" => 26,
    "18.04" => 27,
    "18.10" => 28,
    "19.04" => 29,
    "19.10" => 30,
    "20.04" => 31,
    "20.10" => 32,
    "21.04" => 33,
    "21.10" => 34,
    "22.04" => 35,
    "22.10" => 36,
    "23.04" => 37,
    "23.10" => 38,
    "24.04" => 39,
    "24.10" => 40,
    "25.04" => 41,
    "25.10" => 42,
    "26.04" => 43,
    "26.10" => 44,
    "27.04" => 45,
};

/// Lowercase short code name to index in [`SERIES`].
static SHORT_NAME_INDEX: phf::Map<&'static str, usize> = phf_map! {
    "warty" => 0,
    "hoary" => 1,
    "breezy" => 2,
    "dapper" => 3,
    "edgy" => 4,
    "feisty" => 5,
    "gutsy" => 6,
    "hardy" => 7,
    "intrepid" => 8,
    "jaunty" => 9,
    "karmic" => 10,
    "lucid" => 11,
    "maverick" => 12,
    "natty" => 13,
    "oneiric" => 14,
    "precise" => 15,
    "quantal" => 16,
    "raring" => 17,
    "saucy" => 18,
    "trusty" => 19,
    "utopic" => 20,
    "vivid" => 21,
    "wily" => 22,
    "xenial" => 23,
    "yakkety" => 24,
    "zesty" => 25,
    "artful" => 26,
    "bionic" => 27,
    "cosmic" => 28,
    "disco" => 29,
    "eoan" => 30,
    "focal" => 31,
    "groovy" => 32,
    "hirsute" => 33,
    "impish" => 34,
    "jammy" => 35,
    "kinetic" => 36,
    "lunar" => 37,
    "mantic" => 38,
    "noble" => 39,
    "oracular" => 40,
    "plucky" => 41,
    "questing" => 42,
    "resolute" => 43,
    "stonking" => 44,
    "tenacious" => 45,
};

/// Lowercase full code name to index in [`SERIES`].
static FULL_NAME_INDEX: phf::Map<&'static str, usize> = phf_map! {
    "warty warthog" => 0,
    "hoary hedgehog" => 1,
    "breezy badger" => 2,
    "dapper drake" => 3,
    "edgy eft" => 4,
    "feisty fawn" => 5,
    "gutsy gibbon" => 6,
    "hardy heron" => 7,
    "intrepid ibex" => 8,
    "jaunty jackalope" => 9,
    "karmic koala" => 10,
    "lucid lynx" => 11,
    "maverick meerkat" => 12,
    "natty narwhal" => 13,
    "oneiric ocelot" => 14,
    "precise pangolin" => 15,
    "quantal quetzal" => 16,
    "raring ringtail" => 17,
    "saucy salamander" => 18,
    "trusty tahr" => 19,
    "utopic unicorn" => 20,
    "vivid vervet" => 21,
    "wily werewolf" => 22,
    "xenial xerus" => 23,
    "yakkety yak" => 24,
    "zesty zapus" => 25,
    "artful aardvark" => 26,
    "bionic beaver" => 27,
    "cosmic cuttlefish" => 28,
    "disco dingo" => 29,
    "eoan ermine" => 30,
    "focal fossa" => 31,
    "groovy gorilla" => 32,
    "hirsute hippo" => 33,
    "impish indri" => 34,
    "jammy jellyfish" => 35,
    "kinetic kudu" => 36,
    "lunar lobster" => 37,
    "mantic minotaur" => 38,
    "noble numbat" => 39,
    "oracular oriole" => 40,
    "plucky puffin" => 41,
    "questing quokka" => 42,
    "resolute raccoon" => 43,
    "stonking stingray" => 44,
    "tenacious t-rex" => 45,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_identifiers_are_unique() {
        let mut versions = std::collections::HashSet::new();
        let mut short_names = std::collections::HashSet::new();
        let mut full_names = std::collections::HashSet::new();

        for series in SERIES {
            assert!(
                versions.insert(series.version),
                "duplicate version {}",
                series.version
            );
            assert!(
                short_names.insert(series.code_name_short),
                "duplicate short name {}",
                series.code_name_short
            );
            assert!(
                full_names.insert(series.code_name),
                "duplicate full name {}",
                series.code_name
            );
        }
    }

    #[test]
    fn lookup_maps_match_series_data() {
        assert_eq!(VERSION_INDEX.len(), SERIES.len());
        assert_eq!(SHORT_NAME_INDEX.len(), SERIES.len());
        assert_eq!(FULL_NAME_INDEX.len(), SERIES.len());

        for (index, series) in SERIES.iter().enumerate() {
            assert_eq!(VERSION_INDEX.get(series.version), Some(&index));
            assert_eq!(SHORT_NAME_INDEX.get(series.code_name_short), Some(&index));

            let full_name = series.code_name.to_ascii_lowercase();
            assert_eq!(FULL_NAME_INDEX.get(full_name.as_str()), Some(&index));
        }
    }

    #[test]
    fn devel_index_is_correct() {
        let expected = SERIES
            .iter()
            .enumerate()
            .filter(|(_, series)| series.release_date > REFERENCE_DATE)
            .min_by_key(|(_, series)| series.release_date)
            .map(|(index, _)| index)
            .expect("series data must contain a future release");

        assert_eq!(DEVEL_INDEX, expected);
    }

    #[test]
    fn series_are_ordered_by_release_date() {
        for pair in SERIES.windows(2) {
            assert!(
                pair[0].release_date < pair[1].release_date,
                "{} is not before {}",
                pair[0].code_name_short,
                pair[1].code_name_short
            );
            assert!(
                pair[0] < pair[1],
                "{} does not sort before {}",
                pair[0].code_name_short,
                pair[1].code_name_short
            );
        }
    }

    #[test]
    fn lifecycle_phases_are_ordered() {
        assert!(Lifecycle::Development < Lifecycle::StandardSupport);
        assert!(Lifecycle::StandardSupport < Lifecycle::ExpandedSecurityMaintenance);
        assert!(Lifecycle::ExpandedSecurityMaintenance < Lifecycle::LegacySupport);
        assert!(Lifecycle::LegacySupport < Lifecycle::EndOfLife);
    }

    #[test]
    fn lifecycle_dates_are_consistent() {
        for series in SERIES {
            assert!(
                series.release_date <= series.end_of_standard_support,
                "{}: release after end of standard support",
                series.code_name_short
            );
            assert!(
                series.end_of_standard_support <= series.end_of_life,
                "{}: end of standard support after end of life",
                series.code_name_short
            );
            if let Some(esm_start) = series.esm_start {
                assert!(
                    esm_start <= series.end_of_life,
                    "{}: ESM starts after end of life",
                    series.code_name_short
                );
            }
            if let Some(legacy_start) = series.legacy_start {
                assert!(
                    legacy_start <= series.end_of_life,
                    "{}: legacy support starts after end of life",
                    series.code_name_short
                );
            }
        }
    }

    #[test]
    fn version_lookup_accepts_common_forms() {
        let cases = [
            ("26.04", "resolute"),
            ("26.04 LTS", "resolute"),
            ("26.04.1 LTS", "resolute"),
            ("Ubuntu 26.04 LTS", "resolute"),
            ("ubuntu 26.04 lts", "resolute"),
            ("24.04.1 LTS", "noble"),
            ("26.10", "stonking"),
            ("27.04", "tenacious"),
        ];

        for (input, expected) in cases {
            let series = SeriesInfo::from_version(input).unwrap();
            assert_eq!(series.code_name_short, expected, "input: {input}");
        }
    }

    #[test]
    fn version_lookup_rejects_invalid_forms() {
        for input in [
            "",
            "26",
            "26.4",
            "26.04.1.2",
            "26.04 LTS extra",
            "26.10 LTS",
            "26.10.1",
            "99.99",
        ] {
            assert!(SeriesInfo::from_version(input).is_none(), "input: {input}");
        }
    }

    #[test]
    fn code_name_lookup_accepts_full_and_short_forms() {
        let cases = [
            ("Stonking Stingray", "stonking"),
            ("stonking", "stonking"),
            ("Resolute Raccoon", "resolute"),
            ("resolute", "resolute"),
            ("devel", "tenacious"),
        ];

        for (input, expected) in cases {
            let series = SeriesInfo::from_code_name(input).unwrap();
            assert_eq!(series.code_name_short, expected, "input: {input}");
        }
    }

    #[test]
    fn code_name_lookup_is_case_insensitive() {
        assert_eq!(
            SeriesInfo::from_code_name("STONKING STINGRAY")
                .unwrap()
                .code_name_short,
            "stonking"
        );
        assert_eq!(
            SeriesInfo::from_code_name("StOnKiNg")
                .unwrap()
                .code_name_short,
            "stonking"
        );
    }

    #[test]
    fn devel_is_the_future_release() {
        let devel = SeriesInfo::devel();
        assert_eq!(devel.code_name_short, "tenacious");
        assert_eq!(devel.code_name, "Tenacious T-Rex");
        assert_eq!(devel.version, "27.04");
        assert!(devel.release_date > REFERENCE_DATE);
        assert_eq!(devel.lifecycle_on(REFERENCE_DATE), Lifecycle::Development);
    }

    #[test]
    fn current_lifecycle_groups_are_correct() {
        let active: Vec<_> = SeriesInfo::active()
            .map(|series| series.code_name_short)
            .collect();
        for expected in [
            "trusty", "xenial", "bionic", "focal", "jammy", "noble", "resolute", "stonking",
        ] {
            assert!(
                active.contains(&expected),
                "missing active series {expected}"
            );
        }
        for excluded in ["questing", "plucky", "tenacious"] {
            assert!(
                !active.contains(&excluded),
                "unexpected active series {excluded}"
            );
        }

        let esm: Vec<_> = SeriesInfo::esm()
            .map(|series| series.code_name_short)
            .collect();
        assert_eq!(esm, ["precise", "trusty", "xenial", "bionic", "focal"]);

        let current_esm: Vec<_> = SeriesInfo::current_esm()
            .map(|series| series.code_name_short)
            .collect();
        assert_eq!(current_esm, ["bionic", "focal"]);

        let legacy: Vec<_> = SeriesInfo::legacy()
            .map(|series| series.code_name_short)
            .collect();
        assert_eq!(legacy, ["trusty", "xenial"]);

        let current_legacy: Vec<_> = SeriesInfo::current_legacy()
            .map(|series| series.code_name_short)
            .collect();
        assert_eq!(current_legacy, ["trusty", "xenial"]);

        let eol: Vec<_> = SeriesInfo::end_of_life()
            .map(|series| series.code_name_short)
            .collect();
        for expected in ["precise", "plucky", "questing"] {
            assert!(eol.contains(&expected), "missing EOL series {expected}");
        }
        for excluded in ["trusty", "xenial", "bionic", "focal"] {
            assert!(!eol.contains(&excluded), "unexpected EOL series {excluded}");
        }
    }

    #[test]
    fn lifecycle_phases_are_reported() {
        let resolute = SeriesInfo::from_version("26.04").unwrap();
        assert_eq!(
            resolute.lifecycle_on(REFERENCE_DATE),
            Lifecycle::StandardSupport
        );
        assert_eq!(
            resolute.lifecycle_on(Date::new(2032, 1, 1)),
            Lifecycle::ExpandedSecurityMaintenance
        );
        assert_eq!(
            resolute.lifecycle_on(Date::new(2037, 1, 1)),
            Lifecycle::LegacySupport
        );
        assert_eq!(
            resolute.lifecycle_on(Date::new(2042, 1, 1)),
            Lifecycle::EndOfLife
        );

        let xenial = SeriesInfo::from_version("16.04").unwrap();
        assert_eq!(
            xenial.lifecycle_on(Date::new(2018, 1, 1)),
            Lifecycle::StandardSupport
        );
        assert_eq!(
            xenial.lifecycle_on(Date::new(2022, 1, 1)),
            Lifecycle::ExpandedSecurityMaintenance
        );
        assert_eq!(
            xenial.lifecycle_on(REFERENCE_DATE),
            Lifecycle::LegacySupport
        );
        assert_eq!(
            xenial.lifecycle_on(Date::new(2032, 1, 1)),
            Lifecycle::EndOfLife
        );
    }
}

use std::fmt;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::ParseError;
use crate::pocket::Pocket;
use crate::series::SeriesInfo;

/// A combination of an Ubuntu series and a pocket.
///
/// The `release` pocket has no suffix. All other pockets append `-<pocket>` to
/// the series name, for example `jammy-security`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Suite {
    /// The series this suite belongs to.
    pub series: &'static SeriesInfo,
    /// The pocket within the series.
    pub pocket: Pocket,
}

/// The owned representation used to deserialize [`Suite`].
///
/// [`Suite`] stores a `&'static SeriesInfo`, which serde cannot deserialize
/// directly. Deserialization resolves the series against the built-in series
/// data instead of allocating a new one.
#[cfg(feature = "serde")]
#[derive(Deserialize)]
struct SuiteOwned {
    series: SeriesInfo,
    pocket: Pocket,
}

#[cfg(feature = "serde")]
impl TryFrom<SuiteOwned> for Suite {
    type Error = String;

    fn try_from(owned: SuiteOwned) -> Result<Self, Self::Error> {
        let series = SeriesInfo::from_short_name(owned.series.code_name_short)
            .ok_or_else(|| format!("unknown Ubuntu series: {}", owned.series.code_name_short))?;
        Ok(Self::new(series, owned.pocket))
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Suite {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = SuiteOwned::deserialize(deserializer)?;
        Suite::try_from(owned).map_err(serde::de::Error::custom)
    }
}

impl Suite {
    /// Creates a suite from a series and a pocket.
    pub const fn new(series: &'static SeriesInfo, pocket: Pocket) -> Self {
        Self { series, pocket }
    }

    /// Creates a suite for the series currently in development.
    pub fn devel(pocket: Pocket) -> Self {
        Self::new(SeriesInfo::devel(), pocket)
    }
}

impl fmt::Display for Suite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.series.code_name_short, self.pocket.suffix())
    }
}

impl FromStr for Suite {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (series_name, pocket) = match s.split_once('-') {
            Some((series_name, pocket_name)) => (series_name, Pocket::from_str(pocket_name)?),
            None => (s, Pocket::Release),
        };

        let series = SeriesInfo::from_short_name(series_name)
            .ok_or_else(|| ParseError::UnknownSeries(series_name.to_owned()))?;

        Ok(Self::new(series, pocket))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jammy_suite_table_round_trips() {
        let cases = [
            ("jammy", Pocket::Release),
            ("jammy-security", Pocket::Security),
            ("jammy-updates", Pocket::Updates),
            ("jammy-proposed", Pocket::Proposed),
            ("jammy-backports", Pocket::Backports),
        ];

        for (text, pocket) in cases {
            let suite: Suite = text.parse().unwrap();
            assert_eq!(suite.series.code_name_short, "jammy");
            assert_eq!(suite.pocket, pocket);
            assert_eq!(suite.to_string(), text);
        }
    }

    #[test]
    fn devel_suites_resolve_to_the_future_release() {
        let suite: Suite = "devel-security".parse().unwrap();
        assert_eq!(suite.series.code_name_short, "tenacious");
        assert_eq!(suite.pocket, Pocket::Security);
        assert_eq!(suite.to_string(), "tenacious-security");
    }

    #[test]
    fn unknown_suites_are_rejected() {
        assert!(matches!(
            "unknown".parse::<Suite>(),
            Err(ParseError::UnknownSeries(_))
        ));
        assert!(matches!(
            "jammy-unknown".parse::<Suite>(),
            Err(ParseError::UnknownPocket(_))
        ));
    }
}

use std::fmt;
use std::str::FromStr;

use crate::error::ParseError;
use crate::pocket::Pocket;
use crate::series::SeriesInfo;

/// A combination of an Ubuntu series and a pocket.
///
/// The `release` pocket has no suffix. All other pockets append `-<pocket>` to
/// the series name, for example `jammy-security`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Suite {
    /// The series this suite belongs to.
    pub series: &'static SeriesInfo,
    /// The pocket within the series.
    pub pocket: Pocket,
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

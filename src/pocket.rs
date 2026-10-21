use std::fmt;
use std::str::FromStr;

use crate::error::ParseError;

/// A pocket within an Ubuntu series.
///
/// Every series has five pockets. The `release` pocket has no suite suffix; the
/// other pockets append `-<pocket>` to the series name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pocket {
    /// Packages the series shipped with. Frozen after the initial release.
    Release,
    /// Security-related updates to packages in the release pocket.
    Security,
    /// Non-security updates to packages in the release pocket.
    Updates,
    /// Staging area for updates before they reach `release` or `updates`.
    Proposed,
    /// Packages the series did not ship with. No security support guarantee.
    Backports,
}

impl Pocket {
    /// All pockets, in the order used by the Ubuntu archive documentation.
    pub const ALL: [Pocket; 5] = [
        Pocket::Release,
        Pocket::Security,
        Pocket::Updates,
        Pocket::Proposed,
        Pocket::Backports,
    ];

    /// Returns the pocket's canonical identifier, without a leading hyphen.
    pub const fn as_str(self) -> &'static str {
        match self {
            Pocket::Release => "release",
            Pocket::Security => "security",
            Pocket::Updates => "updates",
            Pocket::Proposed => "proposed",
            Pocket::Backports => "backports",
        }
    }

    /// Returns the suffix appended to a series name to form a suite.
    ///
    /// The release pocket has an empty suffix.
    pub const fn suffix(self) -> &'static str {
        match self {
            Pocket::Release => "",
            Pocket::Security => "-security",
            Pocket::Updates => "-updates",
            Pocket::Proposed => "-proposed",
            Pocket::Backports => "-backports",
        }
    }
}

impl fmt::Display for Pocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Pocket {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "release" => Ok(Pocket::Release),
            "security" => Ok(Pocket::Security),
            "updates" => Ok(Pocket::Updates),
            "proposed" => Ok(Pocket::Proposed),
            "backports" => Ok(Pocket::Backports),
            _ => Err(ParseError::UnknownPocket(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pocket_round_trips() {
        for pocket in Pocket::ALL {
            assert_eq!(pocket.to_string().parse::<Pocket>().unwrap(), pocket);
        }
    }

    #[test]
    fn only_release_has_an_empty_suffix() {
        assert_eq!(Pocket::Release.suffix(), "");
        for pocket in Pocket::ALL.into_iter().filter(|p| *p != Pocket::Release) {
            assert_eq!(pocket.suffix(), format!("-{}", pocket.as_str()));
        }
    }
}

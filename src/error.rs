use std::fmt;

/// An error returned when parsing an Ubuntu identifier or date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The input is not a known archive pocket.
    UnknownPocket(String),
    /// The input is not a known archive component.
    UnknownComponent(String),
    /// The input is not a known machine architecture.
    UnknownArchitecture(String),
    /// The input is not a known Ubuntu series.
    UnknownSeries(String),
    /// The input is not a known Ubuntu suite.
    UnknownSuite(String),
    /// The input is not a valid calendar date.
    InvalidDate(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPocket(value) => write!(f, "unknown Ubuntu archive pocket: {value}"),
            Self::UnknownComponent(value) => write!(f, "unknown Ubuntu archive component: {value}"),
            Self::UnknownArchitecture(value) => {
                write!(f, "unknown Ubuntu machine architecture: {value}")
            }
            Self::UnknownSeries(value) => write!(f, "unknown Ubuntu series: {value}"),
            Self::UnknownSuite(value) => write!(f, "unknown Ubuntu suite: {value}"),
            Self::InvalidDate(value) => write!(f, "invalid date: {value}"),
        }
    }
}

impl std::error::Error for ParseError {}

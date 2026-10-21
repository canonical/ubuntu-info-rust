use std::fmt;
use std::str::FromStr;

use crate::error::ParseError;

/// A calendar date.
///
/// Dates are stored as a year, month, and day. Where an upstream source only
/// specifies a month, use [`Date::year_month`] for the first day of the month or
/// [`Date::end_of_month`] for the last day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

impl Date {
    /// Creates a date.
    ///
    /// # Panics
    ///
    /// Panics if the month or day is out of range. Because this is a `const fn`,
    /// invalid dates in the crate's static data fail at compile time.
    pub const fn new(year: u16, month: u8, day: u8) -> Self {
        assert!(month >= 1 && month <= 12, "month must be in 1..=12");
        assert!(
            day >= 1 && day <= days_in_month(year, month),
            "day is out of range for the month"
        );
        Self { year, month, day }
    }

    /// Creates a date for the first day of the given month.
    pub const fn year_month(year: u16, month: u8) -> Self {
        Self::new(year, month, 1)
    }

    /// Creates a date for the last day of the given month.
    pub const fn end_of_month(year: u16, month: u8) -> Self {
        Self::new(year, month, days_in_month(year, month))
    }

    /// Returns the year.
    pub const fn year(self) -> u16 {
        self.year
    }

    /// Returns the month, from 1 to 12.
    pub const fn month(self) -> u8 {
        self.month
    }

    /// Returns the day of the month, from 1 to 31.
    pub const fn day(self) -> u8 {
        self.day
    }
}

const fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for Date {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseError::InvalidDate(s.to_owned());

        let mut parts = s.split('-');
        let year = parts.next().ok_or_else(invalid)?;
        let month = parts.next().ok_or_else(invalid)?;
        let day = parts.next().ok_or_else(invalid)?;
        if parts.next().is_some() {
            return Err(invalid());
        }

        let year: u16 = year.parse().map_err(|_| invalid())?;
        let month: u8 = month.parse().map_err(|_| invalid())?;
        let day: u8 = day.parse().map_err(|_| invalid())?;

        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return Err(invalid());
        }

        Ok(Self { year, month, day })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_of_month_handles_leap_years() {
        assert_eq!(Date::end_of_month(2024, 2), Date::new(2024, 2, 29));
        assert_eq!(Date::end_of_month(2025, 2), Date::new(2025, 2, 28));
        assert_eq!(Date::end_of_month(2000, 2), Date::new(2000, 2, 29));
        assert_eq!(Date::end_of_month(1900, 2), Date::new(1900, 2, 28));
    }

    #[test]
    fn dates_are_ordered_chronologically() {
        assert!(Date::new(2026, 4, 23) < Date::new(2026, 10, 15));
        assert!(Date::new(2026, 10, 15) < Date::new(2027, 4, 1));
    }

    #[test]
    fn parsing_and_display_round_trip() {
        let date: Date = "2026-04-23".parse().unwrap();
        assert_eq!(date.to_string(), "2026-04-23");
    }

    #[test]
    fn invalid_dates_are_rejected() {
        for input in ["2026-13-01", "2026-02-30", "2026-04", "2026-04-23-01", "x"] {
            assert!(input.parse::<Date>().is_err(), "input: {input}");
        }
    }
}

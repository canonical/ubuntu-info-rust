//! Calendar date parsing and arithmetic for the code generator.
//!
//! The generator only needs a small slice of calendar math: parsing the
//! `YYYY-MM-DD` dates used by `distro-info-data`, stepping to the next day, and
//! taking the first day of a month.

use std::fmt;

/// A calendar date in the proleptic Gregorian calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

impl Date {
    /// Parses a `YYYY-MM-DD` date.
    pub fn parse(input: &str) -> Result<Self, String> {
        let invalid = || format!("invalid date: {input}");

        let mut parts = input.split('-');
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

    /// Returns the date after this one.
    pub fn next_day(self) -> Self {
        let mut day = self.day + 1;
        let mut month = self.month;
        let mut year = self.year;
        if day > days_in_month(year, month) {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
        Self { year, month, day }
    }

    /// Returns the first day of this date's month.
    pub const fn first_of_month(self) -> Self {
        Self {
            year: self.year,
            month: self.month,
            day: 1,
        }
    }

    /// Renders the date as the argument list of a `Date::new` call.
    pub fn rust_args(self) -> String {
        format!("{}, {}, {}", self.year, self.month, self.day)
    }

    /// Renders the date as a `Date::new` expression.
    pub fn rust_expr(self) -> String {
        format!("Date::new({})", self.rust_args())
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_dates() {
        let date = Date::parse("2026-10-16").unwrap();
        assert_eq!(date.year, 2026);
        assert_eq!(date.month, 10);
        assert_eq!(date.day, 16);
    }

    #[test]
    fn rejects_invalid_dates() {
        for input in [
            "",
            "2026",
            "2026-10",
            "2026-10-16-01",
            "2026-13-01",
            "2026-00-10",
            "2026-10-00",
            "2026-02-30",
            "2026-02-29",
            "2024-02-30",
            "x",
        ] {
            assert!(Date::parse(input).is_err(), "input: {input}");
        }
        assert!(Date::parse("2024-02-29").is_ok());
    }

    #[test]
    fn next_day_steps_across_boundaries() {
        assert_eq!(
            Date::parse("2026-10-16").unwrap().next_day(),
            Date::parse("2026-10-17").unwrap()
        );
        assert_eq!(
            Date::parse("2026-04-30").unwrap().next_day(),
            Date::parse("2026-05-01").unwrap()
        );
        assert_eq!(
            Date::parse("2026-12-31").unwrap().next_day(),
            Date::parse("2027-01-01").unwrap()
        );
        assert_eq!(
            Date::parse("2024-02-28").unwrap().next_day(),
            Date::parse("2024-02-29").unwrap()
        );
        assert_eq!(
            Date::parse("2024-02-29").unwrap().next_day(),
            Date::parse("2024-03-01").unwrap()
        );
        assert_eq!(
            Date::parse("2025-02-28").unwrap().next_day(),
            Date::parse("2025-03-01").unwrap()
        );
    }

    #[test]
    fn first_of_month_resets_the_day() {
        assert_eq!(
            Date::parse("2026-10-16").unwrap().first_of_month(),
            Date::parse("2026-10-01").unwrap()
        );
    }

    #[test]
    fn dates_are_ordered_chronologically() {
        assert!(Date::parse("2026-04-23").unwrap() < Date::parse("2026-10-15").unwrap());
        assert!(Date::parse("2026-10-15").unwrap() < Date::parse("2027-04-01").unwrap());
    }

    #[test]
    fn rust_args_render_the_date() {
        assert_eq!(
            Date::parse("2026-10-16").unwrap().rust_args(),
            "2026, 10, 16"
        );
    }

    #[test]
    fn rust_expr_renders_a_constructor_call() {
        assert_eq!(
            Date::parse("2004-10-20").unwrap().rust_expr(),
            "Date::new(2004, 10, 20)"
        );
    }

    #[test]
    fn display_pads_the_fields() {
        assert_eq!(Date::parse("2004-10-20").unwrap().to_string(), "2004-10-20");
    }
}

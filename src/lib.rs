//! Ubuntu distribution information modeled as static data and enums.
//!
//! This crate provides a dependency-light, compile-time source of truth for
//! Ubuntu series, archive pockets, suites, components, machine architectures,
//! and archive locations. It performs no network requests and reads no system
//! files. Series lookups use compile-time perfect hash maps and are O(1) in the
//! number of series.
//!
//! With the `serde` feature enabled, all public types implement serde's
//! `Serialize` and `Deserialize` traits. Enums serialize as their canonical
//! identifiers, and [`Date`] serializes as `YYYY-MM-DD`. [`SeriesInfo`] and
//! [`Suite`] deserialize by resolving against the built-in series data, so the
//! input must describe a known series exactly.
//!
//! # Example
//!
//! ```
//! use ubuntu_info_rust::{Pocket, SeriesInfo, Suite};
//!
//! let resolute = SeriesInfo::from_version("26.04 LTS").unwrap();
//! assert_eq!(resolute.code_name, "Resolute Raccoon");
//! assert_eq!(resolute.code_name_short, "resolute");
//!
//! let stonking = SeriesInfo::from_code_name("Stonking Stingray").unwrap();
//! assert_eq!(stonking.version, "26.10");
//!
//! let suite: Suite = "jammy-security".parse().unwrap();
//! assert_eq!(suite.pocket, Pocket::Security);
//! assert_eq!(suite.series.code_name_short, "jammy");
//!
//! for series in SeriesInfo::active() {
//!     println!("{} {}", series.version, series.code_name);
//! }
//! ```
//!
//! # Static data
//!
//! Series data is current as of [`REFERENCE_DATE`]. The [`SeriesInfo::active`],
//! [`SeriesInfo::esm`], [`SeriesInfo::legacy`], and
//! [`SeriesInfo::end_of_life`] iterators use that date. Use
//! [`SeriesInfo::current_esm`] and [`SeriesInfo::current_legacy`] for the series
//! currently in those phases, or the `*_on` methods, such as
//! [`SeriesInfo::lifecycle_on`], to evaluate a lifecycle phase on a different
//! date.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod architecture;
mod archive;
mod component;
mod date;
mod error;
mod pocket;
mod series;
mod suite;

pub use architecture::{Architecture, Endianness};
pub use archive::Archive;
pub use component::Component;
pub use date::Date;
pub use error::ParseError;
pub use pocket::Pocket;
pub use series::{Lifecycle, REFERENCE_DATE, SERIES, SeriesInfo};
pub use suite::Suite;

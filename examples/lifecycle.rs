//! Show the lifecycle phases of Ubuntu series.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example lifecycle
//! ```

use ubuntu_info_rust::{Lifecycle, REFERENCE_DATE, SeriesInfo};

fn phase_name(phase: Lifecycle) -> &'static str {
    match phase {
        Lifecycle::Development => "development",
        Lifecycle::StandardSupport => "standard support",
        Lifecycle::ExpandedSecurityMaintenance => "ESM",
        Lifecycle::LegacySupport => "legacy support",
        Lifecycle::EndOfLife => "end of life",
    }
}

fn main() {
    println!("Active series on {REFERENCE_DATE}:");
    for series in SeriesInfo::active() {
        println!(
            "  {:<6} {:<20} standard support ends {:<10} EOL {}",
            series.version, series.code_name, series.end_of_standard_support, series.end_of_life
        );
    }

    println!("\nCurrently in ESM:");
    for series in SeriesInfo::current_esm() {
        println!("  {} {}", series.version, series.code_name);
    }

    println!("\nCurrently in legacy support:");
    for series in SeriesInfo::current_legacy() {
        println!("  {} {}", series.version, series.code_name);
    }

    println!("\nEnd of life:");
    for series in SeriesInfo::end_of_life() {
        println!("  {} {}", series.version, series.code_name);
    }

    let devel = SeriesInfo::devel();
    println!(
        "\nDevelopment series: {} {} ({})",
        devel.version,
        devel.code_name,
        phase_name(devel.lifecycle_on(REFERENCE_DATE))
    );
}

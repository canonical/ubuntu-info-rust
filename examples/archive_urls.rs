//! Build Ubuntu archive URLs for suites, components, and architectures.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example archive_urls
//! ```

use ubuntu_info_rust::{Architecture, Archive, Component, Pocket, SeriesInfo, Suite};

fn main() {
    let resolute = SeriesInfo::from_version("26.04 LTS").expect("known series");
    let security = Suite::new(resolute, Pocket::Security);

    println!("Security archive, amd64:");
    for component in Component::ALL {
        println!(
            "  {}",
            Archive::Security.dists_url(&security, component, Architecture::Amd64)
        );
    }

    println!("\nPrimary archive, arm64:");
    println!(
        "  {}",
        Archive::Primary.dists_url(&security, Component::Main, Architecture::Arm64)
    );

    println!("\nCountry mirror (Finland), riscv64:");
    println!(
        "  {}",
        Archive::country("fi").dists_url(&security, Component::Universe, Architecture::Riscv64)
    );
}

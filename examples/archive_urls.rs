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

    println!("Archive used by each architecture:");
    for architecture in Architecture::ALL {
        println!(
            "  {:<8} primary={} security={}",
            architecture,
            architecture.archive_url(),
            architecture.security_archive_url()
        );
    }

    println!("\nSecurity archive, amd64:");
    for component in Component::ALL {
        println!(
            "  {}",
            Archive::Security.dists_url(&security, component, Architecture::Amd64)
        );
    }

    println!("\nPrimary archive, arm64 (ports):");
    println!(
        "  {}",
        Architecture::Arm64.dists_url(&security, Component::Main)
    );

    println!("\nCountry mirror (Finland), riscv64:");
    println!(
        "  {}",
        Archive::country("fi").dists_url(&security, Component::Universe, Architecture::Riscv64)
    );
}

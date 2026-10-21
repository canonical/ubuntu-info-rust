//! Inspect Ubuntu archive components and machine architectures.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example components_and_architectures
//! ```

use ubuntu_info_rust::{Architecture, Component};

fn main() {
    println!("Components:");
    for component in Component::ALL {
        println!(
            "  {:<12} canonical={:<5} community={:<5} open_source={}",
            component,
            component.is_canonical(),
            component.is_community(),
            component.is_open_source()
        );
    }

    println!("\nArchitectures:");
    for architecture in Architecture::ALL {
        println!(
            "  {:<8} endianness={:?} officially_supported={}",
            architecture,
            architecture.endianness(),
            architecture.is_officially_supported()
        );
    }
}

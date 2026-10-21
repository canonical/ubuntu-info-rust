//! Parse and format Ubuntu suites.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example suites
//! ```

use ubuntu_info_rust::{Pocket, Suite};

fn main() {
    let inputs = [
        "jammy",
        "jammy-security",
        "jammy-updates",
        "jammy-proposed",
        "jammy-backports",
        "devel-security",
    ];

    for input in inputs {
        let suite: Suite = input.parse().expect("valid suite");
        println!(
            "{input:<18} -> series={:<10} pocket={:<10} canonical={}",
            suite.series.code_name_short, suite.pocket, suite
        );
    }

    let devel_proposed = Suite::devel(Pocket::Proposed);
    println!("\nDevel proposed suite: {devel_proposed}");
}

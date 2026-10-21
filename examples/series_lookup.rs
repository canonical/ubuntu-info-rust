//! Look up Ubuntu series by version or code name.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example series_lookup
//! ```

use ubuntu_info_rust::SeriesInfo;

fn main() {
    let inputs = [
        "26.04",
        "26.04 LTS",
        "24.04.1 LTS",
        "Ubuntu 22.04 LTS",
        "Stonking Stingray",
        "stonking",
        "devel",
    ];

    for input in inputs {
        let series = SeriesInfo::from_version(input).or_else(|| SeriesInfo::from_code_name(input));

        match series {
            Some(series) => println!(
                "{input:>20} -> {} {} ({})",
                series.version, series.code_name, series.code_name_short
            ),
            None => println!("{input:>20} -> not found"),
        }
    }
}

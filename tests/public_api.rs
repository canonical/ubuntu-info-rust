use std::str::FromStr;

use ubuntu_info_rust::{
    Architecture, Archive, Component, Date, Lifecycle, ParseError, Pocket, REFERENCE_DATE,
    SeriesInfo, Suite,
};

#[test]
fn version_lookup_returns_series_info() {
    let resolute = SeriesInfo::from_version("26.04").unwrap();
    assert_eq!(resolute.code_name, "Resolute Raccoon");
    assert_eq!(resolute.code_name_short, "resolute");
    assert_eq!(resolute.version, "26.04");
    assert!(resolute.is_lts);

    assert_eq!(
        SeriesInfo::from_version("26.04 LTS").unwrap(),
        resolute,
        "the LTS suffix should resolve to the same series"
    );
    assert_eq!(
        SeriesInfo::from_version("26.04.1 LTS").unwrap(),
        resolute,
        "point releases should resolve to their series"
    );
}

#[test]
fn code_name_lookup_returns_series_info() {
    let stonking = SeriesInfo::from_code_name("Stonking Stingray").unwrap();
    assert_eq!(stonking.code_name_short, "stonking");
    assert_eq!(stonking.version, "26.10");

    assert_eq!(
        SeriesInfo::from_code_name("stonking").unwrap(),
        stonking,
        "the short code name should resolve to the same series"
    );
}

#[test]
fn devel_resolves_to_the_future_release() {
    let devel = SeriesInfo::devel();
    assert_eq!(devel.code_name, "Tenacious T-Rex");
    assert_eq!(devel.code_name_short, "tenacious");
    assert_eq!(devel.version, "27.04");
    assert_eq!(devel.lifecycle_on(REFERENCE_DATE), Lifecycle::Development);
    assert_eq!(SeriesInfo::from_code_name("devel").unwrap(), devel);
}

#[test]
fn lifecycle_iterators_partition_current_series() {
    let active: Vec<_> = SeriesInfo::active()
        .map(|series| series.code_name_short)
        .collect();
    assert!(active.contains(&"resolute"));
    assert!(active.contains(&"stonking"));
    assert!(!active.contains(&"questing"));
    assert!(!active.contains(&"tenacious"));

    let esm: Vec<_> = SeriesInfo::esm()
        .map(|series| series.code_name_short)
        .collect();
    assert_eq!(esm, ["precise", "trusty", "xenial", "bionic", "focal"]);

    let current_esm: Vec<_> = SeriesInfo::current_esm()
        .map(|series| series.code_name_short)
        .collect();
    assert_eq!(current_esm, ["bionic", "focal"]);

    let legacy: Vec<_> = SeriesInfo::legacy()
        .map(|series| series.code_name_short)
        .collect();
    assert_eq!(legacy, ["trusty", "xenial"]);

    let current_legacy: Vec<_> = SeriesInfo::current_legacy()
        .map(|series| series.code_name_short)
        .collect();
    assert_eq!(current_legacy, ["trusty", "xenial"]);

    let eol: Vec<_> = SeriesInfo::end_of_life()
        .map(|series| series.code_name_short)
        .collect();
    assert!(eol.contains(&"questing"));
    assert!(!eol.contains(&"trusty"));
}

#[test]
fn enums_round_trip_through_display_and_from_str() {
    for pocket in Pocket::ALL {
        assert_eq!(Pocket::from_str(&pocket.to_string()).unwrap(), pocket);
    }
    for component in Component::ALL {
        assert_eq!(
            Component::from_str(&component.to_string()).unwrap(),
            component
        );
    }
    for architecture in Architecture::ALL {
        assert_eq!(
            Architecture::from_str(&architecture.to_string()).unwrap(),
            architecture
        );
    }
}

#[test]
fn suites_round_trip_and_resolve_devel() {
    for text in [
        "jammy",
        "jammy-security",
        "jammy-updates",
        "jammy-proposed",
        "jammy-backports",
    ] {
        let suite: Suite = text.parse().unwrap();
        assert_eq!(suite.to_string(), text);
    }

    let devel: Suite = "devel-security".parse().unwrap();
    assert_eq!(devel.series.code_name_short, "tenacious");
    assert_eq!(devel.pocket, Pocket::Security);
}

#[test]
fn parse_errors_are_descriptive() {
    let error = "jammy-unknown".parse::<Suite>().unwrap_err();
    assert!(matches!(error, ParseError::UnknownPocket(_)));
    assert!(error.to_string().contains("unknown"));
}

#[test]
fn archive_urls_use_the_documented_layout() {
    let suite: Suite = "jammy-security".parse().unwrap();
    assert_eq!(
        Archive::Security.dists_url(&suite, Component::Main, Architecture::Amd64),
        "http://security.ubuntu.com/ubuntu/dists/jammy-security/main/binary-amd64/"
    );
}

#[test]
fn series_and_suites_are_ordered() {
    let jammy = SeriesInfo::from_version("22.04").unwrap();
    let noble = SeriesInfo::from_version("24.04").unwrap();
    assert!(jammy < noble);

    let jammy_release: Suite = "jammy".parse().unwrap();
    let jammy_security: Suite = "jammy-security".parse().unwrap();
    let noble_release: Suite = "noble".parse().unwrap();

    assert!(jammy_release < jammy_security);
    assert!(jammy_security < noble_release);
}

#[test]
fn dates_are_inclusive_at_phase_boundaries() {
    let xenial = SeriesInfo::from_version("16.04").unwrap();

    assert_eq!(
        xenial.lifecycle_on(xenial.end_of_standard_support),
        Lifecycle::StandardSupport
    );
    assert_eq!(
        xenial.lifecycle_on(Date::new(2021, 5, 1)),
        Lifecycle::ExpandedSecurityMaintenance
    );
    assert_eq!(
        xenial.lifecycle_on(xenial.end_of_life),
        Lifecycle::LegacySupport
    );
    assert_eq!(
        xenial.lifecycle_on(Date::new(2031, 5, 1)),
        Lifecycle::EndOfLife
    );
}

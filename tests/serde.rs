#![cfg(feature = "serde")]

use ubuntu_info_rust::{
    Architecture, Archive, Component, Date, Endianness, Lifecycle, ParseError, Pocket, SeriesInfo,
    Suite,
};

#[test]
fn enums_serialize_as_canonical_identifiers() {
    assert_eq!(
        serde_json::to_string(&Architecture::Amd64).unwrap(),
        "\"amd64\""
    );
    assert_eq!(
        serde_json::to_string(&Architecture::Amd64v3).unwrap(),
        "\"amd64v3\""
    );
    assert_eq!(
        serde_json::to_string(&Endianness::Little).unwrap(),
        "\"little\""
    );
    assert_eq!(
        serde_json::to_string(&Pocket::Security).unwrap(),
        "\"security\""
    );
    assert_eq!(
        serde_json::to_string(&Component::Universe).unwrap(),
        "\"universe\""
    );
    assert_eq!(
        serde_json::to_string(&Lifecycle::ExpandedSecurityMaintenance).unwrap(),
        "\"expanded_security_maintenance\""
    );
}

#[test]
fn enums_round_trip_through_serde() {
    for architecture in Architecture::ALL {
        let json = serde_json::to_string(&architecture).unwrap();
        assert_eq!(
            serde_json::from_str::<Architecture>(&json).unwrap(),
            architecture
        );
    }

    for endianness in [Endianness::Little, Endianness::Big] {
        let json = serde_json::to_string(&endianness).unwrap();
        assert_eq!(
            serde_json::from_str::<Endianness>(&json).unwrap(),
            endianness
        );
    }

    for pocket in Pocket::ALL {
        let json = serde_json::to_string(&pocket).unwrap();
        assert_eq!(serde_json::from_str::<Pocket>(&json).unwrap(), pocket);
    }

    for component in Component::ALL {
        let json = serde_json::to_string(&component).unwrap();
        assert_eq!(serde_json::from_str::<Component>(&json).unwrap(), component);
    }

    for lifecycle in [
        Lifecycle::Development,
        Lifecycle::StandardSupport,
        Lifecycle::ExpandedSecurityMaintenance,
        Lifecycle::LegacySupport,
        Lifecycle::EndOfLife,
    ] {
        let json = serde_json::to_string(&lifecycle).unwrap();
        assert_eq!(serde_json::from_str::<Lifecycle>(&json).unwrap(), lifecycle);
    }
}

#[test]
fn dates_serialize_as_iso_8601_strings() {
    let date = Date::new(2026, 4, 23);
    let json = serde_json::to_string(&date).unwrap();
    assert_eq!(json, "\"2026-04-23\"");
    assert_eq!(serde_json::from_str::<Date>(&json).unwrap(), date);

    assert!(serde_json::from_str::<Date>("\"2026-02-30\"").is_err());
}

#[test]
fn parse_errors_round_trip() {
    let error = ParseError::UnknownArchitecture("amd128".to_owned());
    let json = serde_json::to_string(&error).unwrap();
    assert_eq!(serde_json::from_str::<ParseError>(&json).unwrap(), error);
}

#[test]
fn series_info_round_trips_against_built_in_data() {
    let series = SeriesInfo::from_version("24.04 LTS").unwrap();
    let json = serde_json::to_string(series).unwrap();
    assert_eq!(serde_json::from_str::<SeriesInfo>(&json).unwrap(), *series);
}

#[test]
fn unknown_series_are_rejected_during_deserialization() {
    let json = r#"{
        "code_name": "Unknown Umbrellabird",
        "code_name_short": "unknown",
        "version": "99.99",
        "release_date": "2099-01-01",
        "end_of_standard_support": "2099-01-01",
        "end_of_life": "2099-01-01",
        "esm_start": null,
        "legacy_start": null,
        "is_lts": false
    }"#;

    assert!(serde_json::from_str::<SeriesInfo>(json).is_err());
}

#[test]
fn suites_round_trip_through_serde() {
    let suite: Suite = "jammy-security".parse().unwrap();
    let json = serde_json::to_string(&suite).unwrap();
    assert_eq!(serde_json::from_str::<Suite>(&json).unwrap(), suite);
}

#[test]
fn archives_round_trip_through_serde() {
    for archive in [
        Archive::Primary,
        Archive::Security,
        Archive::Ports,
        Archive::country("fi"),
    ] {
        let json = serde_json::to_string(&archive).unwrap();
        assert_eq!(serde_json::from_str::<Archive>(&json).unwrap(), archive);
    }
}

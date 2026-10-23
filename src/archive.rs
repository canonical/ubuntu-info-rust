use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::architecture::Architecture;
use crate::component::Component;
use crate::suite::Suite;

/// An Ubuntu package archive location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Archive {
    /// The primary archive at `http://archive.ubuntu.com/ubuntu`.
    Primary,
    /// The security archive at `http://security.ubuntu.com/ubuntu`.
    ///
    /// This uses the same servers as the primary archive but a separate hostname
    /// so the security pocket is not accidentally redirected to a mirror.
    Security,
    /// The ports archive at `http://ports.ubuntu.com/ubuntu-ports`.
    ///
    /// This hosts the architectures that are not part of the primary archive,
    /// such as `arm64`, `armhf`, `ppc64el`, `riscv64`, and `s390x`.
    Ports,
    /// A country mirror, identified by its two-letter country code.
    Country(&'static str),
}

/// The owned representation used to deserialize [`Archive`].
///
/// [`Archive::Country`] stores a `&'static str`, which serde cannot deserialize
/// directly. Deserializing a country mirror allocates the country code and
/// leaks it so it can be stored with a `'static` lifetime.
#[cfg(feature = "serde")]
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ArchiveOwned {
    Primary,
    Security,
    Ports,
    Country(String),
}

#[cfg(feature = "serde")]
impl From<ArchiveOwned> for Archive {
    fn from(owned: ArchiveOwned) -> Self {
        match owned {
            ArchiveOwned::Primary => Self::Primary,
            ArchiveOwned::Security => Self::Security,
            ArchiveOwned::Ports => Self::Ports,
            ArchiveOwned::Country(code) => Self::Country(Box::leak(code.into_boxed_str())),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Archive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = ArchiveOwned::deserialize(deserializer)?;
        Ok(Archive::from(owned))
    }
}

impl Archive {
    /// The base URL of the primary archive.
    pub const PRIMARY_URL: &'static str = "http://archive.ubuntu.com/ubuntu";
    /// The base URL of the security archive.
    pub const SECURITY_URL: &'static str = "http://security.ubuntu.com/ubuntu";
    /// The base URL of the ports archive.
    pub const PORTS_URL: &'static str = "http://ports.ubuntu.com/ubuntu-ports";

    /// Creates a country mirror archive from a country code.
    pub const fn country(code: &'static str) -> Self {
        Self::Country(code)
    }

    /// Returns the primary archive that hosts the given architecture.
    ///
    /// `amd64`, `amd64v3`, and `i386` are hosted by the primary archive. All
    /// other architectures are hosted by the ports archive.
    pub const fn for_architecture(architecture: Architecture) -> Self {
        match architecture {
            Architecture::Amd64 | Architecture::Amd64v3 | Architecture::I386 => Self::Primary,
            Architecture::Arm64
            | Architecture::Armhf
            | Architecture::Ppc64el
            | Architecture::Riscv64
            | Architecture::S390x => Self::Ports,
        }
    }

    /// Returns the security archive that hosts the given architecture.
    ///
    /// `amd64`, `amd64v3`, and `i386` use the dedicated security archive. All
    /// other architectures use the security pocket of the ports archive.
    pub const fn security_for_architecture(architecture: Architecture) -> Self {
        match architecture {
            Architecture::Amd64 | Architecture::Amd64v3 | Architecture::I386 => Self::Security,
            Architecture::Arm64
            | Architecture::Armhf
            | Architecture::Ppc64el
            | Architecture::Riscv64
            | Architecture::S390x => Self::Ports,
        }
    }

    /// Returns the archive's base URL.
    pub fn base_url(&self) -> String {
        match self {
            Self::Primary => Self::PRIMARY_URL.to_owned(),
            Self::Security => Self::SECURITY_URL.to_owned(),
            Self::Ports => Self::PORTS_URL.to_owned(),
            Self::Country(code) => format!("http://{code}.archive.ubuntu.com/ubuntu"),
        }
    }

    /// Returns the path to a suite's component for an architecture.
    ///
    /// The path has the form `dists/<suite>/<component>/binary-<arch>/`.
    pub fn dists_path(
        &self,
        suite: &Suite,
        component: Component,
        architecture: Architecture,
    ) -> String {
        format!(
            "dists/{}/{}/binary-{}/",
            suite,
            component.as_str(),
            architecture.as_str()
        )
    }

    /// Returns the full URL to a suite's component for an architecture.
    pub fn dists_url(
        &self,
        suite: &Suite,
        component: Component,
        architecture: Architecture,
    ) -> String {
        format!(
            "{}/{}",
            self.base_url(),
            self.dists_path(suite, component, architecture)
        )
    }
}

impl fmt::Display for Archive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.base_url())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_urls_are_built_from_data() {
        let suite: Suite = "jammy-security".parse().unwrap();

        assert_eq!(
            Archive::Primary.dists_url(&suite, Component::Main, Architecture::Amd64),
            "http://archive.ubuntu.com/ubuntu/dists/jammy-security/main/binary-amd64/"
        );
        assert_eq!(
            Archive::Security.dists_url(&suite, Component::Main, Architecture::Amd64),
            "http://security.ubuntu.com/ubuntu/dists/jammy-security/main/binary-amd64/"
        );
        assert_eq!(
            Archive::Ports.dists_url(&suite, Component::Main, Architecture::Arm64),
            "http://ports.ubuntu.com/ubuntu-ports/dists/jammy-security/main/binary-arm64/"
        );
        assert_eq!(
            Archive::country("fi").base_url(),
            "http://fi.archive.ubuntu.com/ubuntu"
        );
    }

    #[test]
    fn architectures_map_to_their_archives() {
        assert_eq!(
            Archive::for_architecture(Architecture::Amd64),
            Archive::Primary
        );
        assert_eq!(
            Archive::for_architecture(Architecture::Amd64v3),
            Archive::Primary
        );
        assert_eq!(
            Archive::for_architecture(Architecture::I386),
            Archive::Primary
        );
        assert_eq!(
            Archive::for_architecture(Architecture::Arm64),
            Archive::Ports
        );
        assert_eq!(
            Archive::for_architecture(Architecture::S390x),
            Archive::Ports
        );

        assert_eq!(
            Archive::security_for_architecture(Architecture::Amd64),
            Archive::Security
        );
        assert_eq!(
            Archive::security_for_architecture(Architecture::Amd64v3),
            Archive::Security
        );
        assert_eq!(
            Archive::security_for_architecture(Architecture::Arm64),
            Archive::Ports
        );
    }
}

use std::fmt;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::archive::Archive;
use crate::component::Component;
use crate::error::ParseError;
use crate::suite::Suite;

/// A machine architecture supported by Ubuntu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Architecture {
    /// 64-bit x86 (`amd64`).
    Amd64,
    /// 64-bit x86 with the `x86-64-v3` microarchitecture level (`amd64v3`).
    ///
    /// This is an architecture variant introduced in Ubuntu 25.10. It assumes
    /// AVX2, FMA, BMI2, and other instructions available on most AMD64 CPUs
    /// produced since about 2013.
    Amd64v3,
    /// 64-bit ARM (`arm64`).
    Arm64,
    /// 32-bit ARM hard float (`armhf`).
    Armhf,
    /// 32-bit x86 (`i386`).
    ///
    /// This is a partial port: there is no kernel, installer, or bootloader, and
    /// packages are intended for multi-arch use on an `amd64` host.
    I386,
    /// 64-bit PowerPC little-endian (`ppc64el`).
    Ppc64el,
    /// 64-bit RISC-V (`riscv64`).
    Riscv64,
    /// IBM System z (`s390x`).
    S390x,
}

/// The byte order of a machine architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Endianness {
    /// Least significant byte first.
    Little,
    /// Most significant byte first.
    Big,
}

impl Architecture {
    /// All architectures, in the order used by the Ubuntu documentation.
    pub const ALL: [Architecture; 8] = [
        Architecture::Amd64,
        Architecture::Amd64v3,
        Architecture::Arm64,
        Architecture::Armhf,
        Architecture::I386,
        Architecture::Ppc64el,
        Architecture::Riscv64,
        Architecture::S390x,
    ];

    /// Returns the architecture's canonical identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Architecture::Amd64 => "amd64",
            Architecture::Amd64v3 => "amd64v3",
            Architecture::Arm64 => "arm64",
            Architecture::Armhf => "armhf",
            Architecture::I386 => "i386",
            Architecture::Ppc64el => "ppc64el",
            Architecture::Riscv64 => "riscv64",
            Architecture::S390x => "s390x",
        }
    }

    /// Returns the architecture's byte order.
    pub const fn endianness(self) -> Endianness {
        match self {
            Architecture::S390x => Endianness::Big,
            Architecture::Amd64
            | Architecture::Amd64v3
            | Architecture::Arm64
            | Architecture::Armhf
            | Architecture::I386
            | Architecture::Ppc64el
            | Architecture::Riscv64 => Endianness::Little,
        }
    }

    /// Returns `true` for architectures with full official support.
    ///
    /// `i386` is a partial port and returns `false`.
    pub const fn is_officially_supported(self) -> bool {
        !matches!(self, Architecture::I386)
    }

    /// Returns the primary archive that hosts this architecture.
    ///
    /// `amd64`, `amd64v3`, and `i386` are hosted by the primary archive. All
    /// other architectures are hosted by the ports archive.
    pub const fn archive(self) -> Archive {
        Archive::for_architecture(self)
    }

    /// Returns the security archive that hosts this architecture.
    ///
    /// `amd64`, `amd64v3`, and `i386` use the dedicated security archive. All
    /// other architectures use the security pocket of the ports archive.
    pub const fn security_archive(self) -> Archive {
        Archive::security_for_architecture(self)
    }

    /// Returns the base URL of the primary archive for this architecture.
    pub fn archive_url(self) -> String {
        self.archive().base_url()
    }

    /// Returns the base URL of the security archive for this architecture.
    pub fn security_archive_url(self) -> String {
        self.security_archive().base_url()
    }

    /// Returns the full URL to a suite's component for this architecture.
    ///
    /// This uses the primary archive that hosts the architecture.
    pub fn dists_url(self, suite: &Suite, component: Component) -> String {
        self.archive().dists_url(suite, component, self)
    }

    /// Returns the full security URL to a suite's component for this
    /// architecture.
    ///
    /// This uses the security archive that hosts the architecture.
    pub fn security_dists_url(self, suite: &Suite, component: Component) -> String {
        self.security_archive().dists_url(suite, component, self)
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Architecture {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amd64" => Ok(Architecture::Amd64),
            "amd64v3" => Ok(Architecture::Amd64v3),
            "arm64" => Ok(Architecture::Arm64),
            "armhf" => Ok(Architecture::Armhf),
            "i386" => Ok(Architecture::I386),
            "ppc64el" => Ok(Architecture::Ppc64el),
            "riscv64" => Ok(Architecture::Riscv64),
            "s390x" => Ok(Architecture::S390x),
            _ => Err(ParseError::UnknownArchitecture(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_architecture_round_trips() {
        for architecture in Architecture::ALL {
            assert_eq!(
                architecture.to_string().parse::<Architecture>().unwrap(),
                architecture
            );
        }
    }

    #[test]
    fn endianness_is_reported() {
        assert_eq!(Architecture::S390x.endianness(), Endianness::Big);
        for architecture in Architecture::ALL
            .into_iter()
            .filter(|a| *a != Architecture::S390x)
        {
            assert_eq!(architecture.endianness(), Endianness::Little);
        }
    }

    #[test]
    fn i386_is_a_partial_port() {
        assert!(!Architecture::I386.is_officially_supported());
        assert!(Architecture::Amd64.is_officially_supported());
        assert!(Architecture::Amd64v3.is_officially_supported());
    }

    #[test]
    fn architectures_use_the_expected_archives() {
        for architecture in [
            Architecture::Amd64,
            Architecture::Amd64v3,
            Architecture::I386,
        ] {
            assert_eq!(architecture.archive(), Archive::Primary);
            assert_eq!(architecture.security_archive(), Archive::Security);
            assert_eq!(
                architecture.archive_url(),
                "http://archive.ubuntu.com/ubuntu"
            );
            assert_eq!(
                architecture.security_archive_url(),
                "http://security.ubuntu.com/ubuntu"
            );
        }

        for architecture in [
            Architecture::Arm64,
            Architecture::Armhf,
            Architecture::Ppc64el,
            Architecture::Riscv64,
            Architecture::S390x,
        ] {
            assert_eq!(architecture.archive(), Archive::Ports);
            assert_eq!(architecture.security_archive(), Archive::Ports);
            assert_eq!(
                architecture.archive_url(),
                "http://ports.ubuntu.com/ubuntu-ports"
            );
            assert_eq!(
                architecture.security_archive_url(),
                "http://ports.ubuntu.com/ubuntu-ports"
            );
        }
    }

    #[test]
    fn architecture_urls_include_the_architecture() {
        let suite: Suite = "jammy-security".parse().unwrap();

        assert_eq!(
            Architecture::Amd64.dists_url(&suite, Component::Main),
            "http://archive.ubuntu.com/ubuntu/dists/jammy-security/main/binary-amd64/"
        );
        assert_eq!(
            Architecture::Amd64v3.dists_url(&suite, Component::Main),
            "http://archive.ubuntu.com/ubuntu/dists/jammy-security/main/binary-amd64v3/"
        );
        assert_eq!(
            Architecture::Arm64.dists_url(&suite, Component::Universe),
            "http://ports.ubuntu.com/ubuntu-ports/dists/jammy-security/universe/binary-arm64/"
        );
        assert_eq!(
            Architecture::Arm64.security_dists_url(&suite, Component::Main),
            "http://ports.ubuntu.com/ubuntu-ports/dists/jammy-security/main/binary-arm64/"
        );
    }
}

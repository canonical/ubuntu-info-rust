use std::fmt;
use std::str::FromStr;

use crate::error::ParseError;

/// A machine architecture supported by Ubuntu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Architecture {
    /// 64-bit x86 (`amd64`).
    Amd64,
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
pub enum Endianness {
    /// Least significant byte first.
    Little,
    /// Most significant byte first.
    Big,
}

impl Architecture {
    /// All architectures, in the order used by the Ubuntu documentation.
    pub const ALL: [Architecture; 7] = [
        Architecture::Amd64,
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
    }
}

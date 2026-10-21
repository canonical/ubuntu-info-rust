use std::fmt;

use crate::architecture::Architecture;
use crate::component::Component;
use crate::suite::Suite;

/// An Ubuntu package archive location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Archive {
    /// The primary archive at `http://archive.ubuntu.com/ubuntu`.
    Primary,
    /// The security archive at `http://security.ubuntu.com/ubuntu`.
    ///
    /// This uses the same servers as the primary archive but a separate hostname
    /// so the security pocket is not accidentally redirected to a mirror.
    Security,
    /// A country mirror, identified by its two-letter country code.
    Country(&'static str),
}

impl Archive {
    /// The base URL of the primary archive.
    pub const PRIMARY_URL: &'static str = "http://archive.ubuntu.com/ubuntu";
    /// The base URL of the security archive.
    pub const SECURITY_URL: &'static str = "http://security.ubuntu.com/ubuntu";

    /// Creates a country mirror archive from a country code.
    pub const fn country(code: &'static str) -> Self {
        Self::Country(code)
    }

    /// Returns the archive's base URL.
    pub fn base_url(&self) -> String {
        match self {
            Self::Primary => Self::PRIMARY_URL.to_owned(),
            Self::Security => Self::SECURITY_URL.to_owned(),
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
            Archive::country("fi").base_url(),
            "http://fi.archive.ubuntu.com/ubuntu"
        );
    }
}

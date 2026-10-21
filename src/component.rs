use std::fmt;
use std::str::FromStr;

use crate::error::ParseError;

/// A component within an Ubuntu suite.
///
/// Components subdivide packages by whether they are open source and whether
/// they are part of the Ubuntu base or the community.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Component {
    /// Open source packages maintained by Canonical.
    Main,
    /// Non-open-source packages maintained by Canonical, mostly proprietary drivers.
    Restricted,
    /// Open source packages maintained by the Ubuntu community.
    Universe,
    /// Packages that do not meet the full open source definition, maintained by
    /// the Ubuntu community.
    Multiverse,
}

impl Component {
    /// All components, in the order used by the Ubuntu archive documentation.
    pub const ALL: [Component; 4] = [
        Component::Main,
        Component::Restricted,
        Component::Universe,
        Component::Multiverse,
    ];

    /// Returns the component's canonical identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Component::Main => "main",
            Component::Restricted => "restricted",
            Component::Universe => "universe",
            Component::Multiverse => "multiverse",
        }
    }

    /// Returns `true` for components maintained by Canonical (`main` and
    /// `restricted`).
    pub const fn is_canonical(self) -> bool {
        matches!(self, Component::Main | Component::Restricted)
    }

    /// Returns `true` for components maintained by the community (`universe` and
    /// `multiverse`).
    pub const fn is_community(self) -> bool {
        matches!(self, Component::Universe | Component::Multiverse)
    }

    /// Returns `true` for components containing open source software (`main` and
    /// `universe`).
    pub const fn is_open_source(self) -> bool {
        matches!(self, Component::Main | Component::Universe)
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Component {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "main" => Ok(Component::Main),
            "restricted" => Ok(Component::Restricted),
            "universe" => Ok(Component::Universe),
            "multiverse" => Ok(Component::Multiverse),
            _ => Err(ParseError::UnknownComponent(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_component_round_trips() {
        for component in Component::ALL {
            assert_eq!(
                component.to_string().parse::<Component>().unwrap(),
                component
            );
        }
    }

    #[test]
    fn components_are_classified() {
        assert!(Component::Main.is_canonical());
        assert!(Component::Restricted.is_canonical());
        assert!(Component::Universe.is_community());
        assert!(Component::Multiverse.is_community());
        assert!(Component::Main.is_open_source());
        assert!(Component::Universe.is_open_source());
        assert!(!Component::Restricted.is_open_source());
        assert!(!Component::Multiverse.is_open_source());
    }
}

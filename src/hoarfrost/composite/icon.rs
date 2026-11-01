//! A shared icon value used by composites that accept a glyph or identifier.

/// An icon identifier or character glyph string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Icon(pub String);

impl Icon {
    /// Create a new icon from a string representation.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl From<&str> for Icon {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Icon {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::ops::Deref for Icon {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
#[path = "icon_tests.rs"]
mod icon_tests;

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;

/// Maximum number of Unicode scalar values in discovery metadata text.
pub const MAX_METADATA_TEXT_CHARS: usize = 1024;

/// Validated human-readable discovery metadata.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MetadataText(String);

impl MetadataText {
    /// Returns the validated text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for MetadataText {
    type Err = MetadataTextError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.trim() != value
            || value.chars().count() > MAX_METADATA_TEXT_CHARS
            || value.chars().any(char::is_control)
        {
            return Err(MetadataTextError);
        }
        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for MetadataText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for MetadataText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MetadataText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

/// Discovery metadata text was empty, padded, controlled, or too long.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error(
    "metadata text must be trimmed, non-empty, contain no control characters, and contain at most 1024 characters"
)]
pub struct MetadataTextError;

#[cfg(test)]
mod tests {
    use super::{MAX_METADATA_TEXT_CHARS, MetadataText};

    #[test]
    fn rejects_unbounded_or_noncanonical_text() {
        for value in ["", " padded", "padded ", "line\nbreak"] {
            assert!(value.parse::<MetadataText>().is_err(), "accepted {value:?}");
        }
        assert!(
            "x".repeat(MAX_METADATA_TEXT_CHARS + 1)
                .parse::<MetadataText>()
                .is_err()
        );
    }

    #[test]
    fn serde_uses_a_bounded_string() {
        let text = "Window observer"
            .parse::<MetadataText>()
            .expect("valid text");
        let json = serde_json::to_string(&text).expect("serialize");
        assert_eq!(json, "\"Window observer\"");
        assert_eq!(
            serde_json::from_str::<MetadataText>(&json).expect("deserialize"),
            text
        );
    }
}

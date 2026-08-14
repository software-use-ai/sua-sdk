use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;

const MAX_IDENTIFIER_LENGTH: usize = 128;

macro_rules! identifier_type {
    ($name:ident, $label:literal, $allow_digit_start:literal) => {
        #[doc = concat!("A validated ", $label, " identifier.")]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Returns the canonical identifier.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = IdentifierError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                validate_identifier($label, value, $allow_digit_start)?;
                Ok(Self(value.to_owned()))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value.parse().map_err(de::Error::custom)
            }
        }
    };
}

identifier_type!(CapabilityId, "capability", false);
identifier_type!(ProviderId, "provider", false);
identifier_type!(InvocationId, "invocation", true);

/// Why an SDK identifier was rejected.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error(
    "invalid {kind} identifier: use 1-128 lowercase ASCII letters, digits, dots, slashes, and single hyphens; capability/provider IDs start with a letter, and all IDs end with a letter or digit"
)]
pub struct IdentifierError {
    kind: &'static str,
}

fn validate_identifier(
    kind: &'static str,
    value: &str,
    allow_digit_start: bool,
) -> Result<(), IdentifierError> {
    let bytes = value.as_bytes();
    let valid_length = !bytes.is_empty() && bytes.len() <= MAX_IDENTIFIER_LENGTH;
    let valid_start = bytes.first().is_some_and(|byte| {
        byte.is_ascii_lowercase() || (allow_digit_start && byte.is_ascii_digit())
    });
    let valid_end = bytes
        .last()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    let valid_characters = bytes.iter().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(*byte, b'-' | b'.' | b'/')
    });
    let valid_separators = !bytes
        .windows(2)
        .any(|pair| matches!(pair[0], b'-' | b'.' | b'/') && matches!(pair[1], b'-' | b'.' | b'/'));

    if valid_length && valid_start && valid_end && valid_characters && valid_separators {
        Ok(())
    } else {
        Err(IdentifierError { kind })
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityId, InvocationId, ProviderId};

    #[test]
    fn accepts_namespaced_identifiers() {
        assert!("software.window.observe".parse::<CapabilityId>().is_ok());
        assert!("dcc/cua-windows".parse::<ProviderId>().is_ok());
        assert!("invocation-2026-08-14".parse::<InvocationId>().is_ok());
        assert!(
            "019ffe5b-5c93-7091-9277-92d045da6ef0"
                .parse::<InvocationId>()
                .is_ok()
        );
        assert!("019ffe5b".parse::<CapabilityId>().is_err());
    }

    #[test]
    fn rejects_ambiguous_or_noncanonical_identifiers() {
        for value in ["", "Upper", "2fast", "ends-", "two..dots", "under_score"] {
            assert!(value.parse::<CapabilityId>().is_err(), "accepted {value}");
        }
    }

    #[test]
    fn identifier_json_round_trip_is_a_string() {
        let id = "software.read".parse::<CapabilityId>().expect("valid ID");
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"software.read\"");
        assert_eq!(
            serde_json::from_str::<CapabilityId>(&json).expect("deserialize"),
            id
        );
    }
}

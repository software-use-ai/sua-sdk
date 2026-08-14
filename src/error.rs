use serde::{Deserialize, Deserializer, Serialize, de};
use thiserror::Error;

/// Maximum number of Unicode scalar values in a transport diagnostic.
pub const MAX_ERROR_MESSAGE_CHARS: usize = 512;

/// Stable, transport-independent failure categories.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The caller supplied an invalid request or schema value.
    InvalidRequest,
    /// No registered provider can satisfy the requested capability.
    CapabilityUnavailable,
    /// An explicit provider or interaction constraint cannot be satisfied.
    ConstraintUnsatisfied,
    /// Policy denied the invocation before software interaction.
    PolicyDenied,
    /// The invocation identifier already exists.
    DuplicateInvocation,
    /// The invocation identifier is unknown.
    InvocationNotFound,
    /// A bounded runtime or catalog capacity has been reached.
    ResourceExhausted,
    /// Input failed the selected capability schema.
    InputSchemaViolation,
    /// Output failed the selected capability schema.
    OutputSchemaViolation,
    /// The selected provider failed.
    ProviderFailed,
    /// Cancellation was requested and observed.
    Cancelled,
    /// An internal invariant failed without exposing implementation details.
    Internal,
}

/// A length-bounded error safe to carry through SDK transports.
///
/// Producers remain responsible for excluding credentials and other secrets.
#[derive(Clone, Debug, Eq, Error, PartialEq, Serialize)]
#[error("{code:?}: {message}")]
pub struct SoftwareUseError {
    /// Stable machine-readable category.
    code: ErrorCode,
    /// Human-readable diagnostic; producers must redact sensitive values.
    message: String,
    /// Whether a caller may reasonably retry as a new invocation.
    retryable: bool,
}

impl SoftwareUseError {
    /// Constructs a non-retryable SDK error.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: bound_message(message.into()),
            retryable: false,
        }
    }

    /// Returns the stable machine-readable category.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// Returns the bounded human-readable diagnostic.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns whether a new explicit invocation may be retried.
    #[must_use]
    pub const fn retryable(&self) -> bool {
        self.retryable
    }

    /// Marks whether a new explicit invocation may be retried.
    #[must_use]
    pub const fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
}

impl<'de> Deserialize<'de> for SoftwareUseError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireError {
            code: ErrorCode,
            message: String,
            retryable: bool,
        }

        let wire = WireError::deserialize(deserializer)?;
        if wire.message.chars().count() > MAX_ERROR_MESSAGE_CHARS {
            return Err(de::Error::custom(format!(
                "error message exceeds {MAX_ERROR_MESSAGE_CHARS} characters"
            )));
        }
        Ok(Self {
            code: wire.code,
            message: wire.message,
            retryable: wire.retryable,
        })
    }
}

fn bound_message(message: String) -> String {
    if message.chars().count() <= MAX_ERROR_MESSAGE_CHARS {
        return message;
    }
    let mut bounded: String = message.chars().take(MAX_ERROR_MESSAGE_CHARS - 1).collect();
    bounded.push('…');
    bounded
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, MAX_ERROR_MESSAGE_CHARS, SoftwareUseError};

    #[test]
    fn constructor_bounds_transport_diagnostics() {
        let error = SoftwareUseError::new(ErrorCode::Internal, "x".repeat(600));
        assert_eq!(error.message().chars().count(), MAX_ERROR_MESSAGE_CHARS);
        assert!(error.message().ends_with('…'));
    }

    #[test]
    fn deserialization_rejects_oversized_diagnostics() {
        let json = serde_json::json!({
            "code": "internal",
            "message": "x".repeat(MAX_ERROR_MESSAGE_CHARS + 1),
            "retryable": false
        });
        assert!(serde_json::from_value::<SoftwareUseError>(json).is_err());
    }
}

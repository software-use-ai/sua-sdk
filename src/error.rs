use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// A bounded error safe to carry through SDK transports.
#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct SoftwareUseError {
    /// Stable machine-readable category.
    pub code: ErrorCode,
    /// Human-readable diagnostic without arbitrary provider secrets.
    pub message: String,
    /// Whether a caller may reasonably retry as a new invocation.
    pub retryable: bool,
}

impl SoftwareUseError {
    /// Constructs a non-retryable SDK error.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: false,
        }
    }

    /// Marks whether a new explicit invocation may be retried.
    #[must_use]
    pub const fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
}

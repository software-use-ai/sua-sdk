use async_trait::async_trait;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::num::NonZeroU64;

use crate::{
    CapabilityId, CapabilityOffer, InteractionKind, InvocationId, ProviderId, SoftwareUseError,
};

/// Optional constraints that must be honored without implicit relaxation.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationConstraints {
    /// Require one exact provider.
    pub provider_id: Option<ProviderId>,
    /// Require one exact interaction kind.
    pub interaction: Option<InteractionKind>,
}

/// A caller-owned, transport-neutral invocation request.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationRequest {
    /// Idempotency and audit identity supplied by the caller.
    pub invocation_id: InvocationId,
    /// Capability to invoke.
    pub capability_id: CapabilityId,
    /// Accepted capability contract versions.
    pub version_requirement: VersionReq,
    /// JSON value validated against the selected capability input schema.
    pub input: Value,
    /// Hard provider/interaction constraints.
    #[serde(default)]
    pub constraints: InvocationConstraints,
}

/// Evidence returned once a request has been accepted and pinned.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationReceipt {
    /// Invocation identity.
    pub invocation_id: InvocationId,
    /// Selected capability identity.
    pub capability_id: CapabilityId,
    /// Selected provider identity.
    pub provider_id: ProviderId,
    /// Selected provider implementation version.
    pub provider_version: Version,
    /// Selected capability contract version.
    pub capability_version: Version,
    /// Selected interaction kind.
    pub interaction: InteractionKind,
}

/// Successful provider output after runtime validation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationResult {
    /// JSON value validated against the selected capability output schema.
    pub output: Value,
}

/// Stable invocation lifecycle states.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationState {
    /// Accepted but provider execution has not begun.
    Accepted,
    /// The pinned provider is executing.
    Running,
    /// Output was produced and validated.
    Succeeded,
    /// Execution or validation failed.
    Failed,
    /// Cancellation was requested and observed.
    Cancelled,
}

/// Structured evidence for a terminal cancellation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationOutcome {
    /// Cancellation won before provider execution began.
    BeforeExecution,
    /// The running provider explicitly acknowledged the runtime signal.
    AcknowledgedByProvider,
}

/// Lifecycle status with state-specific evidence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvocationStatus {
    /// Accepted but provider execution has not begun.
    Accepted,
    /// The pinned provider is executing.
    Running,
    /// Output was produced and validated.
    Succeeded {
        /// Validated provider output.
        result: InvocationResult,
    },
    /// Execution or output validation failed.
    Failed {
        /// Bounded failure evidence.
        error: SoftwareUseError,
    },
    /// Cancellation reached a terminal state.
    Cancelled {
        /// Whether cancellation won before execution or was provider-acknowledged.
        outcome: CancellationOutcome,
    },
}

impl InvocationStatus {
    /// Returns the state discriminator without discarding evidence.
    #[must_use]
    pub const fn state(&self) -> InvocationState {
        match self {
            Self::Accepted => InvocationState::Accepted,
            Self::Running => InvocationState::Running,
            Self::Succeeded { .. } => InvocationState::Succeeded,
            Self::Failed { .. } => InvocationState::Failed,
            Self::Cancelled { .. } => InvocationState::Cancelled,
        }
    }

    /// Returns successful output when the invocation succeeded.
    #[must_use]
    pub const fn result(&self) -> Option<&InvocationResult> {
        match self {
            Self::Succeeded { result } => Some(result),
            _ => None,
        }
    }

    /// Returns failure evidence when the invocation failed.
    #[must_use]
    pub const fn error(&self) -> Option<&SoftwareUseError> {
        match self {
            Self::Failed { error } => Some(error),
            _ => None,
        }
    }
}

/// A point-in-time view of one invocation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationSnapshot {
    /// Acceptance and selection evidence.
    pub receipt: InvocationReceipt,
    /// Current lifecycle status and its state-specific evidence.
    pub status: InvocationStatus,
    /// Last emitted event sequence; accepted snapshots always have at least one event.
    pub last_event_sequence: NonZeroU64,
}

impl InvocationSnapshot {
    /// Returns the current state discriminator.
    #[must_use]
    pub const fn state(&self) -> InvocationState {
        self.status.state()
    }

    /// Returns successful output when the invocation succeeded.
    #[must_use]
    pub const fn result(&self) -> Option<&InvocationResult> {
        self.status.result()
    }

    /// Returns failure evidence when the invocation failed.
    #[must_use]
    pub const fn error(&self) -> Option<&SoftwareUseError> {
        self.status.error()
    }
}

/// Stable event categories for replay and audit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Runtime accepted and pinned the invocation.
    Accepted,
    /// Provider execution began.
    Started,
    /// A caller requested cancellation.
    CancellationRequested,
    /// Output completed and passed validation.
    Succeeded,
    /// Execution or validation failed.
    Failed,
    /// Cancellation reached a terminal state.
    Cancelled,
}

/// Event category with required category-specific evidence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeEventData {
    /// Runtime accepted and pinned the invocation.
    Accepted,
    /// Provider execution began.
    Started,
    /// A caller requested cancellation.
    CancellationRequested,
    /// Output completed and passed validation.
    Succeeded,
    /// Execution or validation failed.
    Failed {
        /// Bounded failure evidence.
        error: SoftwareUseError,
    },
    /// Cancellation reached a terminal state.
    Cancelled {
        /// Structured cancellation evidence.
        outcome: CancellationOutcome,
    },
}

impl RuntimeEventData {
    /// Returns the category discriminator without discarding evidence.
    #[must_use]
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::Accepted => EventKind::Accepted,
            Self::Started => EventKind::Started,
            Self::CancellationRequested => EventKind::CancellationRequested,
            Self::Succeeded => EventKind::Succeeded,
            Self::Failed { .. } => EventKind::Failed,
            Self::Cancelled { .. } => EventKind::Cancelled,
        }
    }

    /// Returns failure evidence for a failed event.
    #[must_use]
    pub const fn error(&self) -> Option<&SoftwareUseError> {
        match self {
            Self::Failed { error } => Some(error),
            _ => None,
        }
    }
}

/// One append-only runtime lifecycle event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEvent {
    /// Invocation identity.
    pub invocation_id: InvocationId,
    /// Gap-free, monotonically increasing sequence beginning at one.
    pub sequence: NonZeroU64,
    /// Pinned provider.
    pub provider_id: ProviderId,
    /// Lifecycle category and required category-specific evidence.
    pub data: RuntimeEventData,
}

impl RuntimeEvent {
    /// Returns the event category discriminator.
    #[must_use]
    pub const fn kind(&self) -> EventKind {
        self.data.kind()
    }

    /// Returns failure evidence for a failed event.
    #[must_use]
    pub const fn error(&self) -> Option<&SoftwareUseError> {
        self.data.error()
    }
}

/// Exclusive event replay cursor.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct EventCursor(pub u64);

/// One transport-neutral event replay page.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventPage {
    /// Events with a sequence greater than the requested cursor.
    pub events: Vec<RuntimeEvent>,
    /// Cursor that can be supplied to the next replay call.
    pub next_cursor: EventCursor,
    /// Whether the invocation was terminal when this page was produced.
    pub terminal: bool,
}

/// Agent/adapter-facing runtime port, independent of MCP, HTTP, CLI, or IPC.
#[async_trait]
pub trait InvocationPort: Send + Sync {
    /// Discovers deterministically ordered capability offers.
    async fn discover(
        &self,
        query: crate::CapabilityQuery,
    ) -> Result<Vec<CapabilityOffer>, SoftwareUseError>;

    /// Accepts, validates, selects, and starts one invocation.
    async fn start(
        &self,
        request: InvocationRequest,
    ) -> Result<InvocationReceipt, SoftwareUseError>;

    /// Reads the current state without changing it.
    async fn get(
        &self,
        invocation_id: &InvocationId,
    ) -> Result<InvocationSnapshot, SoftwareUseError>;

    /// Replays events after an optional exclusive cursor.
    async fn events(
        &self,
        invocation_id: &InvocationId,
        after: Option<EventCursor>,
    ) -> Result<EventPage, SoftwareUseError>;

    /// Requests cooperative cancellation and returns the resulting snapshot.
    async fn cancel(
        &self,
        invocation_id: &InvocationId,
    ) -> Result<InvocationSnapshot, SoftwareUseError>;
}

#[cfg(test)]
mod tests {
    use super::{InvocationSnapshot, RuntimeEvent};

    #[test]
    fn malformed_snapshot_state_evidence_is_rejected() {
        let json = r#"{
            "receipt": {
                "invocation_id": "invocation-1",
                "capability_id": "software.observe",
                "provider_id": "provider.one",
                "provider_version": "1.0.0",
                "capability_version": "1.0.0",
                "interaction": "typed"
            },
            "status": {"state": "succeeded"},
            "last_event_sequence": 1
        }"#;
        assert!(serde_json::from_str::<InvocationSnapshot>(json).is_err());
    }

    #[test]
    fn event_sequence_zero_and_failed_without_error_are_rejected() {
        let zero = r#"{
            "invocation_id": "invocation-1",
            "sequence": 0,
            "provider_id": "provider.one",
            "data": {"kind": "started"}
        }"#;
        assert!(serde_json::from_str::<RuntimeEvent>(zero).is_err());

        let missing_error = r#"{
            "invocation_id": "invocation-1",
            "sequence": 1,
            "provider_id": "provider.one",
            "data": {"kind": "failed"}
        }"#;
        assert!(serde_json::from_str::<RuntimeEvent>(missing_error).is_err());
    }
}

use async_trait::async_trait;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// A point-in-time view of one invocation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationSnapshot {
    /// Acceptance and selection evidence.
    pub receipt: InvocationReceipt,
    /// Current lifecycle state.
    pub state: InvocationState,
    /// Present only for a successful terminal state.
    pub result: Option<InvocationResult>,
    /// Present only for a failed terminal state.
    pub error: Option<SoftwareUseError>,
    /// Last emitted event sequence, or zero before any event.
    pub last_event_sequence: u64,
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

/// One append-only runtime lifecycle event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEvent {
    /// Invocation identity.
    pub invocation_id: InvocationId,
    /// Gap-free, monotonically increasing sequence beginning at one.
    pub sequence: u64,
    /// Lifecycle category.
    pub kind: EventKind,
    /// Pinned provider, if selection completed.
    pub provider_id: Option<ProviderId>,
    /// Bounded failure evidence, if this is a failed event.
    pub error: Option<SoftwareUseError>,
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

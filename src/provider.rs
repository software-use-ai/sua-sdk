use async_trait::async_trait;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{CapabilityId, InvocationRequest, InvocationResult, ProviderId, SoftwareUseError};

/// How a provider interacts with software, ordered from strongest to weakest contract.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    /// A typed vendor or application API.
    Typed,
    /// Structured UI semantics bound to an exact target.
    SemanticUi,
    /// Pixel observation and bounded visual input.
    VisualUi,
}

/// The default-policy safety class of a capability.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationSafety {
    /// Observation that must not mutate application state.
    ReadOnly,
    /// Any operation that may change application or external state.
    Mutating,
}

/// A versioned capability contract with schemas on both sides of execution.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySpec {
    /// Stable capability identity.
    pub id: CapabilityId,
    /// Semantic version of this capability contract.
    pub version: Version,
    /// Short human-readable label.
    pub title: String,
    /// Bounded explanatory text for discovery clients.
    pub description: String,
    /// JSON Schema for invocation input.
    pub input_schema: Value,
    /// JSON Schema for successful output.
    pub output_schema: Value,
    /// Safety class evaluated by runtime policy.
    pub safety: OperationSafety,
}

/// Stable identity and provenance claims for one provider implementation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderDescriptor {
    /// Stable provider identity.
    pub id: ProviderId,
    /// Semantic version of the provider implementation.
    pub version: Version,
    /// Human-readable name.
    pub name: String,
    /// Human-readable provider description.
    pub description: String,
}

/// One capability as implemented by a provider through one or more interactions.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCapability {
    /// Versioned capability contract.
    pub spec: CapabilitySpec,
    /// Non-empty supported interaction kinds; runtimes validate this claim.
    pub interactions: Vec<InteractionKind>,
}

/// A concrete, serializable candidate produced by discovery.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityOffer {
    /// Provider that owns execution and target integrity.
    pub provider: ProviderDescriptor,
    /// Selected capability contract.
    pub capability: CapabilitySpec,
    /// One supported interaction kind for deterministic ranking.
    pub interaction: InteractionKind,
}

/// Filters for transport-neutral capability discovery.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityQuery {
    /// Optional exact capability identity.
    pub capability_id: Option<CapabilityId>,
    /// Optional accepted capability contract versions.
    pub version_requirement: Option<VersionReq>,
    /// Optional exact provider identity.
    pub provider_id: Option<ProviderId>,
    /// Optional required interaction kind.
    pub interaction: Option<InteractionKind>,
}

/// The immutable request handed to the selected provider.
#[derive(Clone, Debug)]
pub struct ProviderInvocation {
    /// Original caller request.
    pub request: InvocationRequest,
    /// Capability/provider/interaction pinned by the runtime.
    pub offer: CapabilityOffer,
}

/// Cloneable cooperative cancellation shared across the provider boundary.
#[derive(Clone, Debug, Default)]
pub struct CancellationSignal(CancellationToken);

impl CancellationSignal {
    /// Creates an uncancelled signal.
    #[must_use]
    pub fn new() -> Self {
        Self(CancellationToken::new())
    }

    /// Requests cancellation. Repeated calls are idempotent.
    pub fn cancel(&self) {
        self.0.cancel();
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }

    /// Waits until cancellation is requested.
    pub async fn cancelled(&self) {
        self.0.cancelled().await;
    }
}

/// Provider-side execution boundary; target discovery and integrity remain implementation-owned.
#[async_trait]
pub trait CapabilityProvider: Send + Sync {
    /// Describes the provider without performing software interaction.
    fn descriptor(&self) -> ProviderDescriptor;

    /// Lists the provider's versioned capabilities.
    fn capabilities(&self) -> Vec<ProviderCapability>;

    /// Executes exactly the offer selected by the runtime.
    async fn invoke(
        &self,
        invocation: ProviderInvocation,
        cancellation: CancellationSignal,
    ) -> Result<InvocationResult, SoftwareUseError>;
}

#[cfg(test)]
mod tests {
    use super::{CancellationSignal, InteractionKind};

    #[test]
    fn interaction_order_is_typed_then_semantic_then_visual() {
        let mut kinds = vec![
            InteractionKind::VisualUi,
            InteractionKind::Typed,
            InteractionKind::SemanticUi,
        ];
        kinds.sort();
        assert_eq!(
            kinds,
            vec![
                InteractionKind::Typed,
                InteractionKind::SemanticUi,
                InteractionKind::VisualUi,
            ]
        );
    }

    #[tokio::test]
    async fn cancellation_is_cloneable_and_idempotent() {
        let signal = CancellationSignal::new();
        let observer = signal.clone();
        assert!(!observer.is_cancelled());
        signal.cancel();
        signal.cancel();
        observer.cancelled().await;
        assert!(observer.is_cancelled());
    }
}

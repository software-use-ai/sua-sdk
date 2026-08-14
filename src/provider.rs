use async_trait::async_trait;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    CapabilityId, InvocationRequest, InvocationResult, MetadataText, ProviderId, SoftwareUseError,
};

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

/// Explicit JSON Schema dialect for reproducible capability contracts.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaDialect {
    /// JSON Schema Draft 2020-12.
    Draft202012,
}

impl SchemaDialect {
    /// Returns the canonical dialect URI.
    #[must_use]
    pub const fn canonical_uri(self) -> &'static str {
        match self {
            Self::Draft202012 => "https://json-schema.org/draft/2020-12/schema",
        }
    }
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
    pub title: MetadataText,
    /// Bounded explanatory text for discovery clients.
    pub description: MetadataText,
    /// Explicit dialect applied to both schemas.
    pub schema_dialect: SchemaDialect,
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
    pub name: MetadataText,
    /// Human-readable provider description.
    pub description: MetadataText,
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

/// Runtime-owned authority for cooperative cancellation.
///
/// Providers receive only [`CancellationSignal`], so they cannot manufacture a
/// caller cancellation or a corresponding runtime event.
#[derive(Clone, Debug, Default)]
pub struct CancellationSource(CancellationToken);

impl CancellationSource {
    /// Creates an uncancelled source.
    #[must_use]
    pub fn new() -> Self {
        Self(CancellationToken::new())
    }

    /// Creates a read-only signal for a provider.
    #[must_use]
    pub fn signal(&self) -> CancellationSignal {
        CancellationSignal(self.0.clone())
    }

    /// Requests cancellation. Repeated calls are idempotent and owner-only.
    pub fn cancel(&self) {
        self.0.cancel();
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

/// Cloneable, read-only cooperative cancellation observed by providers.
#[derive(Clone, Debug)]
pub struct CancellationSignal(CancellationToken);

impl CancellationSignal {
    /// Returns whether the runtime has requested cancellation.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }

    /// Waits until the runtime requests cancellation.
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
    use super::{CancellationSource, InteractionKind};

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
    async fn cancellation_source_is_idempotent_and_signal_is_read_only() {
        let source = CancellationSource::new();
        let signal = source.signal();
        let observer = signal.clone();
        assert!(!observer.is_cancelled());
        source.cancel();
        source.cancel();
        observer.cancelled().await;
        assert!(observer.is_cancelled());
    }
}

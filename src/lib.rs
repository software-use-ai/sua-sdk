//! Transport-neutral contracts for the Software Use ecosystem.
//!
//! This crate defines the stable boundary shared by agents, runtimes, providers,
//! registries, and protocol adapters. It intentionally contains no transport,
//! process/window automation, package loading, or runtime policy implementation.

mod error;
mod id;
mod invocation;
mod plugin;
mod provider;
mod text;

pub use error::{ErrorCode, MAX_ERROR_MESSAGE_CHARS, SoftwareUseError};
pub use id::{
    CapabilityId, EventId, IdentifierError, InvocationId, PermissionId, PluginId, ProtocolId,
    ProviderId, ServiceId,
};
pub use invocation::{
    CancellationOutcome, EventCursor, EventKind, EventPage, EventPageValidationError,
    InvocationConstraints, InvocationPort, InvocationReceipt, InvocationRequest, InvocationResult,
    InvocationSnapshot, InvocationState, InvocationStatus, RuntimeEvent, RuntimeEventData,
};
pub use plugin::{
    CapabilityContractRef, CapabilityRequirement, EventContract, EventMode,
    MAX_PERMISSION_CONSTRAINT_BYTES, MAX_PERMISSION_CONSTRAINT_COLLECTION_ITEMS,
    MAX_PERMISSION_CONSTRAINT_DEPTH, MAX_PERMISSION_CONSTRAINT_KEY_CHARS,
    MAX_PERMISSION_CONSTRAINT_NODES, MAX_PERMISSION_CONSTRAINT_STRING_CHARS,
    MAX_PLUGIN_DECLARATIONS, MAX_PLUGIN_DECLARATIONS_PER_COLLECTION, PLUGIN_MANIFEST_VERSION,
    PermissionConstraints, PermissionConstraintsError, PermissionRequest, PluginExecution,
    PluginManifest, PluginManifestCollection, PluginManifestValidationError, ServiceProvision,
    ServiceRequirement,
};
pub use provider::{
    CancellationSignal, CancellationSource, CapabilityOffer, CapabilityProvider, CapabilityQuery,
    CapabilitySpec, InteractionKind, OperationSafety, ProviderCapability, ProviderDescriptor,
    ProviderInvocation, SchemaDialect,
};
pub use text::{MAX_METADATA_TEXT_CHARS, MetadataText, MetadataTextError};

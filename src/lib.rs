//! Transport-neutral contracts for the Software Use ecosystem.
//!
//! This crate defines the stable boundary shared by agents, runtimes, providers,
//! registries, and protocol adapters. It intentionally contains no transport,
//! process/window automation, package loading, or runtime policy implementation.

mod error;
mod id;
mod invocation;
mod provider;
mod text;

pub use error::{ErrorCode, MAX_ERROR_MESSAGE_CHARS, SoftwareUseError};
pub use id::{CapabilityId, IdentifierError, InvocationId, ProviderId};
pub use invocation::{
    CancellationOutcome, EventCursor, EventKind, EventPage, EventPageValidationError,
    InvocationConstraints, InvocationPort, InvocationReceipt, InvocationRequest, InvocationResult,
    InvocationSnapshot, InvocationState, InvocationStatus, RuntimeEvent, RuntimeEventData,
};
pub use provider::{
    CancellationSignal, CancellationSource, CapabilityOffer, CapabilityProvider, CapabilityQuery,
    CapabilitySpec, InteractionKind, OperationSafety, ProviderCapability, ProviderDescriptor,
    ProviderInvocation, SchemaDialect,
};
pub use text::{MAX_METADATA_TEXT_CHARS, MetadataText, MetadataTextError};

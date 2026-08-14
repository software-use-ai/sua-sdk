//! Transport-neutral contracts for the Software Use ecosystem.
//!
//! This crate defines the stable boundary shared by agents, runtimes, providers,
//! registries, and protocol adapters. It intentionally contains no transport,
//! process/window automation, package loading, or runtime policy implementation.

mod error;
mod id;
mod invocation;
mod provider;

pub use error::{ErrorCode, SoftwareUseError};
pub use id::{CapabilityId, IdentifierError, InvocationId, ProviderId};
pub use invocation::{
    EventCursor, EventKind, EventPage, InvocationConstraints, InvocationPort, InvocationReceipt,
    InvocationRequest, InvocationResult, InvocationSnapshot, InvocationState, RuntimeEvent,
};
pub use provider::{
    CancellationSignal, CancellationSource, CapabilityOffer, CapabilityProvider, CapabilityQuery,
    CapabilitySpec, InteractionKind, OperationSafety, ProviderCapability, ProviderDescriptor,
    ProviderInvocation,
};

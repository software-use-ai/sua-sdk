//! Declarative contracts for Cordis-style Software Use plugins.
//!
//! A plugin is a semantic and lifecycle unit: it declares services, events,
//! capabilities, and permission requests that a runtime can admit and compose.
//! It is not a Rust dynamic-library ABI. Version `0.1` supports statically
//! linked factories and bounded isolated-process protocols; package catalogs
//! and runtimes own entrypoint resolution, trust decisions, grants, activation,
//! rollback, and disposal.

use std::{collections::BTreeSet, fmt};

use semver::{Version, VersionReq};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;
use thiserror::Error;

use crate::{CapabilityId, EventId, MetadataText, PermissionId, PluginId, ProtocolId, ServiceId};

/// The plugin manifest schema version supported by this SDK release.
pub const PLUGIN_MANIFEST_VERSION: u32 = 1;

/// Maximum entries accepted in any one plugin declaration collection.
pub const MAX_PLUGIN_DECLARATIONS_PER_COLLECTION: usize = 128;

/// Maximum entries accepted across all plugin declaration collections.
pub const MAX_PLUGIN_DECLARATIONS: usize = 512;

/// Maximum encoded JSON bytes in one permission constraint object.
pub const MAX_PERMISSION_CONSTRAINT_BYTES: usize = 16 * 1024;

/// Maximum nesting below the root of one permission constraint object.
pub const MAX_PERMISSION_CONSTRAINT_DEPTH: usize = 8;

/// Maximum entries in one permission constraint object or array.
pub const MAX_PERMISSION_CONSTRAINT_COLLECTION_ITEMS: usize = 64;

/// Maximum nodes in one permission constraint tree.
pub const MAX_PERMISSION_CONSTRAINT_NODES: usize = 256;

/// Maximum Unicode scalar values in one permission constraint string.
pub const MAX_PERMISSION_CONSTRAINT_STRING_CHARS: usize = 1024;

/// Maximum Unicode scalar values in one permission constraint object key.
pub const MAX_PERMISSION_CONSTRAINT_KEY_CHARS: usize = 128;

/// How a runtime obtains and isolates a plugin implementation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PluginExecution {
    /// A trusted factory linked into the runtime binary.
    StaticFactory,
    /// A separately supervised process speaking a bounded, versioned protocol.
    IsolatedProcess {
        /// Stable protocol identity used during the handshake.
        protocol_id: ProtocolId,
        /// Protocol versions the plugin process can speak.
        protocol_requirement: VersionReq,
    },
}

impl<'de> Deserialize<'de> for PluginExecution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let Value::Object(mut fields) = Value::deserialize(deserializer)? else {
            return Err(de::Error::custom("plugin execution must be a JSON object"));
        };
        let Some(Value::String(kind)) = fields.remove("kind") else {
            return Err(de::Error::missing_field("kind"));
        };

        match kind.as_str() {
            "static_factory" => {
                if let Some(unknown) = fields.keys().next() {
                    return Err(de::Error::unknown_field(unknown, &["kind"]));
                }
                Ok(Self::StaticFactory)
            }
            "isolated_process" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct IsolatedProcessFields {
                    protocol_id: ProtocolId,
                    protocol_requirement: VersionReq,
                }

                let fields = IsolatedProcessFields::deserialize(Value::Object(fields))
                    .map_err(de::Error::custom)?;
                Ok(Self::IsolatedProcess {
                    protocol_id: fields.protocol_id,
                    protocol_requirement: fields.protocol_requirement,
                })
            }
            _ => Err(de::Error::unknown_variant(
                &kind,
                &["static_factory", "isolated_process"],
            )),
        }
    }
}

impl PluginExecution {
    /// Returns the isolated-process protocol contract, when applicable.
    #[must_use]
    pub const fn protocol(&self) -> Option<(&ProtocolId, &VersionReq)> {
        match self {
            Self::StaticFactory => None,
            Self::IsolatedProcess {
                protocol_id,
                protocol_requirement,
            } => Some((protocol_id, protocol_requirement)),
        }
    }
}

/// One versioned service made available by a plugin while it is active.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProvision {
    /// Stable service identity.
    pub service_id: ServiceId,
    /// Exact implementation contract version.
    pub version: Version,
}

/// One service dependency declared by a plugin.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceRequirement {
    /// Stable service identity.
    pub service_id: ServiceId,
    /// Service contract versions accepted by the consumer.
    pub version_requirement: VersionReq,
}

/// Dispatch semantics shared by publishers and subscribers of a typed event.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventMode {
    /// Notify matching listeners without transforming the event payload.
    Emit,
    /// Run all matching listeners concurrently.
    Parallel,
    /// Run matching listeners in deterministic sequence.
    Serial,
    /// Run listeners until one returns a terminal result.
    Bail,
    /// Pass each listener's output to the next listener.
    Waterfall,
}

/// One exact typed event contract published or consumed by a plugin.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventContract {
    /// Stable event identity.
    pub event_id: EventId,
    /// Exact event payload contract version.
    pub version: Version,
    /// Required dispatch semantics.
    pub mode: EventMode,
}

/// A plugin's exact implementation of one capability contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityContractRef {
    /// Stable capability identity.
    pub capability_id: CapabilityId,
    /// Exact capability contract version.
    pub version: Version,
}

/// A capability contract dependency declared by a plugin.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRequirement {
    /// Stable capability identity.
    pub capability_id: CapabilityId,
    /// Capability contract versions accepted by the consumer.
    pub version_requirement: VersionReq,
}

/// A bounded, declarative JSON object that narrows a permission request.
///
/// The object is data, not an authority token. A host may reject it or issue a
/// narrower grant. It must never contain credentials or executable commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PermissionConstraints(Value);

impl PermissionConstraints {
    /// Returns the validated constraint object.
    #[must_use]
    pub const fn as_value(&self) -> &Value {
        &self.0
    }

    /// Consumes this wrapper and returns its JSON object.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.0
    }
}

impl Default for PermissionConstraints {
    fn default() -> Self {
        Self(Value::Object(serde_json::Map::new()))
    }
}

impl TryFrom<Value> for PermissionConstraints {
    type Error = PermissionConstraintsError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        validate_permission_constraints(&value)?;
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for PermissionConstraints {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(Value::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Why a permission constraint object was rejected.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum PermissionConstraintsError {
    /// The top-level value was not an object.
    #[error("permission constraints must be a JSON object")]
    NotAnObject,
    /// The encoded object exceeded the wire-size limit.
    #[error("permission constraints exceed 16384 encoded JSON bytes")]
    EncodedSizeExceeded,
    /// The object nested too deeply.
    #[error("permission constraints exceed 8 levels below the root")]
    DepthExceeded,
    /// One object or array had too many direct entries.
    #[error("a permission constraint collection exceeds 64 entries")]
    CollectionItemsExceeded,
    /// The overall constraint tree had too many values.
    #[error("permission constraints exceed 256 nodes")]
    NodesExceeded,
    /// One object key was empty, padded, controlled, or too long.
    #[error(
        "permission constraint keys must be trimmed, non-empty, contain no control characters, and contain at most 128 characters"
    )]
    InvalidKey,
    /// One string value was controlled or too long.
    #[error(
        "permission constraint strings must contain no control characters and contain at most 1024 characters"
    )]
    InvalidString,
}

/// A declared request for authority that a host may independently grant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionRequest {
    /// Stable permission identity understood by the admitting host.
    pub permission_id: PermissionId,
    /// Whether activation must fail when the permission is not granted.
    pub required: bool,
    /// Human-readable justification for audit and approval surfaces.
    pub reason: MetadataText,
    /// Bounded restrictions requested by the plugin.
    pub constraints: PermissionConstraints,
}

/// A complete, transport-neutral declaration of one plugin generation.
///
/// Deserialization performs [`PluginManifest::validate`], so malformed wire
/// manifests cannot enter a runtime as this type. Programmatic construction
/// must call `validate` before admission.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    /// Manifest schema version; currently [`PLUGIN_MANIFEST_VERSION`].
    pub manifest_version: u32,
    /// Stable plugin identity.
    pub plugin_id: PluginId,
    /// Exact plugin implementation version.
    pub version: Version,
    /// Runtime versions this plugin supports.
    pub runtime_requirement: VersionReq,
    /// Implementation isolation model.
    pub execution: PluginExecution,
    /// Services visible only while this plugin generation is active.
    pub provides_services: Vec<ServiceProvision>,
    /// Dependencies that must be present before activation.
    pub requires_services: Vec<ServiceRequirement>,
    /// Dependencies whose absence must not prevent activation.
    pub optional_services: Vec<ServiceRequirement>,
    /// Typed events this plugin may publish.
    pub publishes_events: Vec<EventContract>,
    /// Typed events this plugin may subscribe to.
    pub subscribes_events: Vec<EventContract>,
    /// Requests evaluated by host admission policy; they are not grants.
    pub permissions: Vec<PermissionRequest>,
    /// Exact capability contracts contributed by this plugin.
    pub provided_capabilities: Vec<CapabilityContractRef>,
    /// Capability contracts this plugin consumes.
    pub required_capabilities: Vec<CapabilityRequirement>,
}

impl PluginManifest {
    /// Validates schema version, declaration bounds, identity uniqueness, and
    /// cross-collection dependency/event invariants.
    ///
    /// # Errors
    ///
    /// Returns [`PluginManifestValidationError`] for the first deterministic
    /// invariant violation.
    pub fn validate(&self) -> Result<(), PluginManifestValidationError> {
        if self.manifest_version != PLUGIN_MANIFEST_VERSION {
            return Err(PluginManifestValidationError::UnsupportedManifestVersion {
                found: self.manifest_version,
            });
        }

        self.validate_declaration_bounds()?;
        self.validate_unique_declarations()?;
        self.validate_service_dependencies()?;
        self.validate_event_contracts()
    }

    fn declaration_counts(&self) -> [(PluginManifestCollection, usize); 8] {
        [
            (
                PluginManifestCollection::ProvidesServices,
                self.provides_services.len(),
            ),
            (
                PluginManifestCollection::RequiresServices,
                self.requires_services.len(),
            ),
            (
                PluginManifestCollection::OptionalServices,
                self.optional_services.len(),
            ),
            (
                PluginManifestCollection::PublishesEvents,
                self.publishes_events.len(),
            ),
            (
                PluginManifestCollection::SubscribesEvents,
                self.subscribes_events.len(),
            ),
            (
                PluginManifestCollection::Permissions,
                self.permissions.len(),
            ),
            (
                PluginManifestCollection::ProvidedCapabilities,
                self.provided_capabilities.len(),
            ),
            (
                PluginManifestCollection::RequiredCapabilities,
                self.required_capabilities.len(),
            ),
        ]
    }

    fn validate_declaration_bounds(&self) -> Result<(), PluginManifestValidationError> {
        let collections = self.declaration_counts();

        for (collection, count) in collections {
            if count > MAX_PLUGIN_DECLARATIONS_PER_COLLECTION {
                return Err(PluginManifestValidationError::TooManyDeclarations {
                    collection,
                    maximum: MAX_PLUGIN_DECLARATIONS_PER_COLLECTION,
                });
            }
        }

        let total = collections.iter().map(|(_, count)| count).sum::<usize>();
        if total > MAX_PLUGIN_DECLARATIONS {
            return Err(PluginManifestValidationError::TooManyTotalDeclarations {
                maximum: MAX_PLUGIN_DECLARATIONS,
            });
        }

        Ok(())
    }

    fn validate_unique_declarations(&self) -> Result<(), PluginManifestValidationError> {
        reject_duplicates(
            PluginManifestCollection::ProvidesServices,
            &self.provides_services,
            |item| item.service_id.as_str(),
        )?;
        reject_duplicates(
            PluginManifestCollection::RequiresServices,
            &self.requires_services,
            |item| item.service_id.as_str(),
        )?;
        reject_duplicates(
            PluginManifestCollection::OptionalServices,
            &self.optional_services,
            |item| item.service_id.as_str(),
        )?;
        reject_duplicates(
            PluginManifestCollection::PublishesEvents,
            &self.publishes_events,
            |item| item.event_id.as_str(),
        )?;
        reject_duplicates(
            PluginManifestCollection::SubscribesEvents,
            &self.subscribes_events,
            |item| item.event_id.as_str(),
        )?;
        reject_duplicates(
            PluginManifestCollection::Permissions,
            &self.permissions,
            |item| item.permission_id.as_str(),
        )?;
        reject_duplicates(
            PluginManifestCollection::ProvidedCapabilities,
            &self.provided_capabilities,
            |item| item.capability_id.as_str(),
        )?;
        reject_duplicates(
            PluginManifestCollection::RequiredCapabilities,
            &self.required_capabilities,
            |item| item.capability_id.as_str(),
        )
    }

    fn validate_service_dependencies(&self) -> Result<(), PluginManifestValidationError> {
        let required_services = self
            .requires_services
            .iter()
            .map(|item| &item.service_id)
            .collect::<BTreeSet<_>>();
        if let Some(overlap) = self
            .optional_services
            .iter()
            .find(|item| required_services.contains(&item.service_id))
        {
            return Err(
                PluginManifestValidationError::RequiredOptionalServiceOverlap {
                    service_id: overlap.service_id.clone(),
                },
            );
        }

        Ok(())
    }

    fn validate_event_contracts(&self) -> Result<(), PluginManifestValidationError> {
        for published in &self.publishes_events {
            let Some(subscribed) = self
                .subscribes_events
                .iter()
                .find(|event| event.event_id == published.event_id)
            else {
                continue;
            };
            if subscribed.version != published.version {
                return Err(PluginManifestValidationError::EventVersionConflict {
                    event_id: published.event_id.clone(),
                    published: published.version.clone(),
                    subscribed: subscribed.version.clone(),
                });
            }
            if subscribed.mode != published.mode {
                return Err(PluginManifestValidationError::EventModeConflict {
                    event_id: published.event_id.clone(),
                    published: published.mode,
                    subscribed: subscribed.mode,
                });
            }
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for PluginManifest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireManifest {
            manifest_version: u32,
            plugin_id: PluginId,
            version: Version,
            runtime_requirement: VersionReq,
            execution: PluginExecution,
            provides_services: BoundedDeclarations<ServiceProvision>,
            requires_services: BoundedDeclarations<ServiceRequirement>,
            optional_services: BoundedDeclarations<ServiceRequirement>,
            publishes_events: BoundedDeclarations<EventContract>,
            subscribes_events: BoundedDeclarations<EventContract>,
            permissions: BoundedDeclarations<PermissionRequest>,
            provided_capabilities: BoundedDeclarations<CapabilityContractRef>,
            required_capabilities: BoundedDeclarations<CapabilityRequirement>,
        }

        let wire = WireManifest::deserialize(deserializer)?;
        let manifest = Self {
            manifest_version: wire.manifest_version,
            plugin_id: wire.plugin_id,
            version: wire.version,
            runtime_requirement: wire.runtime_requirement,
            execution: wire.execution,
            provides_services: wire.provides_services.into_vec(),
            requires_services: wire.requires_services.into_vec(),
            optional_services: wire.optional_services.into_vec(),
            publishes_events: wire.publishes_events.into_vec(),
            subscribes_events: wire.subscribes_events.into_vec(),
            permissions: wire.permissions.into_vec(),
            provided_capabilities: wire.provided_capabilities.into_vec(),
            required_capabilities: wire.required_capabilities.into_vec(),
        };
        manifest.validate().map_err(de::Error::custom)?;
        Ok(manifest)
    }
}

/// Declaration collections identified by manifest validation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginManifestCollection {
    /// `provides_services`.
    ProvidesServices,
    /// `requires_services`.
    RequiresServices,
    /// `optional_services`.
    OptionalServices,
    /// `publishes_events`.
    PublishesEvents,
    /// `subscribes_events`.
    SubscribesEvents,
    /// `permissions`.
    Permissions,
    /// `provided_capabilities`.
    ProvidedCapabilities,
    /// `required_capabilities`.
    RequiredCapabilities,
}

impl fmt::Display for PluginManifestCollection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ProvidesServices => "provides_services",
            Self::RequiresServices => "requires_services",
            Self::OptionalServices => "optional_services",
            Self::PublishesEvents => "publishes_events",
            Self::SubscribesEvents => "subscribes_events",
            Self::Permissions => "permissions",
            Self::ProvidedCapabilities => "provided_capabilities",
            Self::RequiredCapabilities => "required_capabilities",
        })
    }
}

/// Why a plugin manifest was rejected before runtime admission.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum PluginManifestValidationError {
    /// The manifest schema version is not supported by this SDK.
    #[error("unsupported plugin manifest version {found}; expected 1")]
    UnsupportedManifestVersion {
        /// Version supplied by the manifest.
        found: u32,
    },
    /// One declaration collection exceeded its independent limit.
    #[error("plugin manifest {collection} exceeds {maximum} declarations")]
    TooManyDeclarations {
        /// Collection that exceeded the limit.
        collection: PluginManifestCollection,
        /// Supported maximum.
        maximum: usize,
    },
    /// The sum of all declarations exceeded the manifest limit.
    #[error("plugin manifest exceeds {maximum} total declarations")]
    TooManyTotalDeclarations {
        /// Supported maximum.
        maximum: usize,
    },
    /// One collection declared the same stable identity more than once.
    #[error("plugin manifest {collection} repeats identifier {identifier}")]
    DuplicateDeclaration {
        /// Collection containing the duplicate.
        collection: PluginManifestCollection,
        /// Repeated bounded identifier.
        identifier: String,
    },
    /// One service was declared as both required and optional.
    #[error("service {service_id} cannot be both required and optional")]
    RequiredOptionalServiceOverlap {
        /// Conflicting service identity.
        service_id: ServiceId,
    },
    /// One event was published and subscribed with different versions.
    #[error("event {event_id} version conflict: published {published}, subscribed {subscribed}")]
    EventVersionConflict {
        /// Conflicting event identity.
        event_id: EventId,
        /// Published version.
        published: Version,
        /// Subscribed version.
        subscribed: Version,
    },
    /// One event was published and subscribed with different dispatch modes.
    #[error("event {event_id} mode conflict: published {published:?}, subscribed {subscribed:?}")]
    EventModeConflict {
        /// Conflicting event identity.
        event_id: EventId,
        /// Published mode.
        published: EventMode,
        /// Subscribed mode.
        subscribed: EventMode,
    },
}

struct BoundedDeclarations<T>(Vec<T>);

impl<T> BoundedDeclarations<T> {
    fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<'de, T> Deserialize<'de> for BoundedDeclarations<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DeclarationsVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T> de::Visitor<'de> for DeclarationsVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = BoundedDeclarations<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "an array with at most {MAX_PLUGIN_DECLARATIONS_PER_COLLECTION} declarations"
                )
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let initial_capacity = sequence
                    .size_hint()
                    .unwrap_or_default()
                    .min(MAX_PLUGIN_DECLARATIONS_PER_COLLECTION);
                let mut values = Vec::with_capacity(initial_capacity);
                while let Some(value) = sequence.next_element()? {
                    if values.len() == MAX_PLUGIN_DECLARATIONS_PER_COLLECTION {
                        return Err(de::Error::invalid_length(
                            MAX_PLUGIN_DECLARATIONS_PER_COLLECTION + 1,
                            &self,
                        ));
                    }
                    values.push(value);
                }
                Ok(BoundedDeclarations(values))
            }
        }

        deserializer.deserialize_seq(DeclarationsVisitor(std::marker::PhantomData))
    }
}

fn reject_duplicates<T>(
    collection: PluginManifestCollection,
    values: &[T],
    identifier: impl Fn(&T) -> &str,
) -> Result<(), PluginManifestValidationError> {
    let mut seen = BTreeSet::new();
    for value in values {
        let identifier = identifier(value);
        if !seen.insert(identifier) {
            return Err(PluginManifestValidationError::DuplicateDeclaration {
                collection,
                identifier: identifier.to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_permission_constraints(value: &Value) -> Result<(), PermissionConstraintsError> {
    if !value.is_object() {
        return Err(PermissionConstraintsError::NotAnObject);
    }
    if serde_json::to_vec(value)
        .map_err(|_| PermissionConstraintsError::EncodedSizeExceeded)?
        .len()
        > MAX_PERMISSION_CONSTRAINT_BYTES
    {
        return Err(PermissionConstraintsError::EncodedSizeExceeded);
    }

    let mut nodes = 0;
    validate_permission_node(value, 0, &mut nodes)
}

fn validate_permission_node(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), PermissionConstraintsError> {
    if depth > MAX_PERMISSION_CONSTRAINT_DEPTH {
        return Err(PermissionConstraintsError::DepthExceeded);
    }
    *nodes += 1;
    if *nodes > MAX_PERMISSION_CONSTRAINT_NODES {
        return Err(PermissionConstraintsError::NodesExceeded);
    }

    match value {
        Value::Object(entries) => {
            if entries.len() > MAX_PERMISSION_CONSTRAINT_COLLECTION_ITEMS {
                return Err(PermissionConstraintsError::CollectionItemsExceeded);
            }
            for (key, child) in entries {
                if key.is_empty()
                    || key.trim() != key
                    || key.chars().count() > MAX_PERMISSION_CONSTRAINT_KEY_CHARS
                    || key.chars().any(char::is_control)
                {
                    return Err(PermissionConstraintsError::InvalidKey);
                }
                validate_permission_node(child, depth + 1, nodes)?;
            }
        }
        Value::Array(entries) => {
            if entries.len() > MAX_PERMISSION_CONSTRAINT_COLLECTION_ITEMS {
                return Err(PermissionConstraintsError::CollectionItemsExceeded);
            }
            for child in entries {
                validate_permission_node(child, depth + 1, nodes)?;
            }
        }
        Value::String(text) => {
            if text.chars().count() > MAX_PERMISSION_CONSTRAINT_STRING_CHARS
                || text.chars().any(char::is_control)
            {
                return Err(PermissionConstraintsError::InvalidString);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use semver::Version;
    use serde_json::{Value, json};

    use super::{
        EventContract, EventMode, MAX_PERMISSION_CONSTRAINT_COLLECTION_ITEMS,
        MAX_PERMISSION_CONSTRAINT_DEPTH, MAX_PERMISSION_CONSTRAINT_STRING_CHARS,
        MAX_PLUGIN_DECLARATIONS_PER_COLLECTION, PermissionConstraints, PermissionConstraintsError,
        PluginExecution, PluginManifest, PluginManifestValidationError,
    };

    fn valid_manifest_json() -> Value {
        json!({
            "manifest_version": 1,
            "plugin_id": "software-use.paint",
            "version": "0.1.0",
            "runtime_requirement": "^0.1",
            "execution": {
                "kind": "isolated_process",
                "protocol_id": "software-use.plugin-rpc",
                "protocol_requirement": "^1"
            },
            "provides_services": [{
                "service_id": "software.paint-controller",
                "version": "1.0.0"
            }],
            "requires_services": [{
                "service_id": "software.target-binding",
                "version_requirement": "^1"
            }],
            "optional_services": [{
                "service_id": "runtime.telemetry",
                "version_requirement": "^1"
            }],
            "publishes_events": [{
                "event_id": "software.paint/saved",
                "version": "1.0.0",
                "mode": "emit"
            }],
            "subscribes_events": [{
                "event_id": "runtime.shutdown",
                "version": "1.0.0",
                "mode": "serial"
            }],
            "permissions": [{
                "permission_id": "filesystem.write",
                "required": true,
                "reason": "Write the caller-approved PNG output.",
                "constraints": {"extensions": ["png"], "overwrite": false}
            }],
            "provided_capabilities": [{
                "capability_id": "software.windows.paint.image.compose",
                "version": "1.0.0"
            }],
            "required_capabilities": [{
                "capability_id": "software.window.observe",
                "version_requirement": "^1"
            }]
        })
    }

    #[test]
    fn manifest_round_trip_preserves_strict_wire_shape() {
        let value = valid_manifest_json();
        let manifest: PluginManifest =
            serde_json::from_value(value.clone()).expect("valid plugin manifest");
        manifest.validate().expect("manifest remains valid");
        assert_eq!(serde_json::to_value(&manifest).expect("serialize"), value);
        assert_eq!(
            manifest
                .execution
                .protocol()
                .map(|(id, requirement)| (id.as_str(), requirement.to_string())),
            Some(("software-use.plugin-rpc", "^1".to_owned()))
        );
    }

    #[test]
    fn static_factory_has_no_dynamic_abi_fields() {
        let value = serde_json::to_value(PluginExecution::StaticFactory).expect("serialize");
        assert_eq!(value, json!({"kind": "static_factory"}));
        assert!(
            serde_json::from_value::<PluginExecution>(json!({
                "kind": "static_factory",
                "library": "plugin.dll"
            }))
            .is_err()
        );
    }

    #[test]
    fn deserialization_rejects_unknown_fields_and_manifest_versions() {
        let mut unknown = valid_manifest_json();
        unknown
            .as_object_mut()
            .expect("object")
            .insert("implicit_grants".to_owned(), json!(["filesystem.write"]));
        assert!(serde_json::from_value::<PluginManifest>(unknown).is_err());

        let mut unsupported = valid_manifest_json();
        unsupported["manifest_version"] = json!(2);
        assert!(serde_json::from_value::<PluginManifest>(unsupported).is_err());
    }

    #[test]
    fn duplicate_and_overlapping_service_declarations_are_rejected() {
        let mut duplicate = valid_manifest_json();
        duplicate["provides_services"] = json!([
            {"service_id":"software.paint-controller","version":"1.0.0"},
            {"service_id":"software.paint-controller","version":"2.0.0"}
        ]);
        assert!(serde_json::from_value::<PluginManifest>(duplicate).is_err());

        let mut overlap = valid_manifest_json();
        overlap["optional_services"] = json!([{
            "service_id": "software.target-binding",
            "version_requirement": "^2"
        }]);
        assert!(serde_json::from_value::<PluginManifest>(overlap).is_err());
    }

    #[test]
    fn conflicting_event_contracts_are_rejected() {
        let mut mode = valid_manifest_json();
        mode["subscribes_events"] = json!([{
            "event_id": "software.paint/saved",
            "version": "1.0.0",
            "mode": "parallel"
        }]);
        assert!(serde_json::from_value::<PluginManifest>(mode).is_err());

        let mut version = valid_manifest_json();
        version["subscribes_events"] = json!([{
            "event_id": "software.paint/saved",
            "version": "2.0.0",
            "mode": "emit"
        }]);
        assert!(serde_json::from_value::<PluginManifest>(version).is_err());
    }

    #[test]
    fn declaration_arrays_are_bounded_during_deserialization() {
        let mut value = valid_manifest_json();
        value["permissions"] = Value::Array(
            (0..=MAX_PLUGIN_DECLARATIONS_PER_COLLECTION)
                .map(|index| {
                    json!({
                        "permission_id": format!("permission.item-{index}"),
                        "required": false,
                        "reason": "Bound test.",
                        "constraints": {}
                    })
                })
                .collect(),
        );
        assert!(serde_json::from_value::<PluginManifest>(value).is_err());
    }

    #[test]
    fn permission_constraints_are_objects_and_strictly_bounded() {
        assert_eq!(
            PermissionConstraints::try_from(json!(["not", "an", "object"])),
            Err(PermissionConstraintsError::NotAnObject)
        );
        assert_eq!(
            PermissionConstraints::try_from(
                json!({"value": "x".repeat(MAX_PERMISSION_CONSTRAINT_STRING_CHARS + 1)})
            ),
            Err(PermissionConstraintsError::InvalidString)
        );

        let too_many = (0..=MAX_PERMISSION_CONSTRAINT_COLLECTION_ITEMS)
            .map(|index| (format!("key-{index}"), Value::Bool(true)))
            .collect::<serde_json::Map<_, _>>();
        assert_eq!(
            PermissionConstraints::try_from(Value::Object(too_many)),
            Err(PermissionConstraintsError::CollectionItemsExceeded)
        );
    }

    #[test]
    fn permission_constraint_depth_and_total_nodes_are_bounded() {
        let mut nested = Value::Bool(true);
        for _ in 0..=MAX_PERMISSION_CONSTRAINT_DEPTH {
            nested = json!({"nested": nested});
        }
        assert_eq!(
            PermissionConstraints::try_from(nested),
            Err(PermissionConstraintsError::DepthExceeded)
        );

        let leaves = (0..MAX_PERMISSION_CONSTRAINT_COLLECTION_ITEMS)
            .map(|_| Value::Array(vec![Value::Bool(true); 4]))
            .collect::<Vec<_>>();
        let many_nodes = json!({"items": leaves});
        assert_eq!(
            PermissionConstraints::try_from(many_nodes),
            Err(PermissionConstraintsError::NodesExceeded)
        );
    }

    #[test]
    fn manifest_validation_error_exposes_event_mode_evidence() {
        let mut manifest: PluginManifest =
            serde_json::from_value(valid_manifest_json()).expect("valid manifest");
        manifest.subscribes_events = vec![EventContract {
            event_id: "software.paint/saved".parse().expect("event id"),
            version: Version::new(1, 0, 0),
            mode: EventMode::Waterfall,
        }];

        assert!(matches!(
            manifest.validate(),
            Err(PluginManifestValidationError::EventModeConflict {
                published: EventMode::Emit,
                subscribed: EventMode::Waterfall,
                ..
            })
        ));
    }
}

# Software Use SDK

`sua-sdk` is the transport-neutral contract layer for **Software Use — Let
agents use any software**. It lets agents discover versioned capabilities and
invoke them through a runtime without coupling the ecosystem to MCP, HTTP, a UI
driver, a DCC host, or a specific agent framework.

The contract graph is deliberately small, while its implementations are
composable plugins:

```text
Plugin -> provides/requires Service
       -> publishes/subscribes typed Event
       -> provides/requires Capability
       -> requests Permission (the host grants authority)
Agent  -> consumes Capability through InvocationPort
```

## What this crate owns

- validated capability, provider, and invocation identities;
- versioned Draft 2020-12 input/output capability contracts and safety
  classification;
- deterministic interaction kinds: `Typed`, `SemanticUi`, `VisualUi`;
- provider discovery offers and hard invocation constraints;
- state-valid invocation receipts, tagged snapshots/events, bounded errors, and
  owner/source-separated cooperative cancellation;
- the provider-facing `CapabilityProvider` port and agent-facing
  `InvocationPort`.
- Cordis-style plugin declaration contracts: bounded manifests, service and
  event dependencies, permission requests, and static-factory or
  isolated-process execution models.

## What this crate does not own

- provider selection, policy, schema enforcement, job storage, or retries
  (`sua-runtime`);
- package publication, integrity declarations, or catalog loading
  (`sua-registry`);
- transports such as MCP, HTTP, CLI, or IPC;
- DCC adapters, CUA drivers, PID/HWND binding, or application-specific logic.

## Plugin contract

A plugin is a semantic and lifecycle unit, not a Rust dynamic-library ABI.
`StaticFactory` identifies code linked into a trusted runtime;
`IsolatedProcess` declares only a versioned protocol. The registry resolves a
package entrypoint, while the runtime owns dependency ordering, activation,
leases, rollback, draining, and disposal.

Every manifest collection and every permission constraint tree is bounded.
Wire deserialization rejects unknown fields, unsupported manifest versions,
duplicate identities, required/optional service overlap, and conflicting event
versions or dispatch modes.

```rust
use semver::{Version, VersionReq};
use sua_sdk::{
    PLUGIN_MANIFEST_VERSION, PluginExecution, PluginManifest, ServiceProvision,
};

let manifest = PluginManifest {
    manifest_version: PLUGIN_MANIFEST_VERSION,
    plugin_id: "software-use.paint".parse()?,
    version: Version::new(0, 1, 0),
    runtime_requirement: VersionReq::parse("^0.1")?,
    execution: PluginExecution::StaticFactory,
    provides_services: vec![ServiceProvision {
        service_id: "software.paint-controller".parse()?,
        version: Version::new(1, 0, 0),
    }],
    requires_services: vec![],
    optional_services: vec![],
    publishes_events: vec![],
    subscribes_events: vec![],
    permissions: vec![],
    provided_capabilities: vec![],
    required_capabilities: vec![],
};
manifest.validate()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

A `PermissionRequest` is never a grant. Admission policy may deny it or issue
narrower authority, and a native in-process plugin is trusted code rather than
a sandboxed extension. See
[`docs/adr/0001-plugin-contract-boundary.md`](docs/adr/0001-plugin-contract-boundary.md).

In particular, exact software-target integrity remains a provider concern. The
SDK carries no generic process/window handle because those types are not
portable capability contracts.

Public discovery text and error diagnostics are length-bounded validated types.
Snapshot/event enums make contradictory lifecycle evidence unrepresentable, and
event sequences are non-zero. Event pages validate cross-record identity,
gap-free ordering, cursor, and terminal evidence at construction and wire
deserialization. Runtime implementations still own provider trust, secret
redaction, policy, payload-size limits, and execution semantics.

## Compatibility

The minimum supported Rust version is 1.97. Public wire types use closed serde
schemas; additive or breaking changes follow semantic versioning.

```powershell
vx cargo fmt --all -- --check
vx cargo check --locked --all-targets
vx cargo test --locked --all-targets
vx cargo clippy --locked --all-targets -- -D warnings
```

## License

MIT

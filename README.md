# Software Use SDK

`sua-sdk` is the transport-neutral contract layer for **Software Use — Let
agents use any software**. It lets agents discover versioned capabilities and
invoke them through a runtime without coupling the ecosystem to MCP, HTTP, a UI
driver, a DCC host, or a specific agent framework.

The contract graph is deliberately small:

```text
Provider -> provides Capability
Skill    -> requires Capability
Profile  -> composes Capability
Agent    -> consumes Capability through InvocationPort
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

## What this crate does not own

- provider selection, policy, schema enforcement, job storage, or retries
  (`sua-runtime`);
- package publication, integrity declarations, or catalog loading
  (`sua-registry`);
- transports such as MCP, HTTP, CLI, or IPC;
- DCC adapters, CUA drivers, PID/HWND binding, or application-specific logic.

In particular, exact software-target integrity remains a provider concern. The
SDK carries no generic process/window handle because those types are not
portable capability contracts.

Public discovery text and error diagnostics are length-bounded validated types.
Snapshot/event enums make contradictory lifecycle evidence unrepresentable, and
event sequences are non-zero. Runtime implementations still own provider trust,
secret redaction, policy, payload-size limits, and execution semantics.

## Compatibility

The minimum supported Rust version is 1.85. Public wire types use closed serde
schemas; additive or breaking changes follow semantic versioning.

```powershell
vx cargo fmt --all -- --check
vx cargo check --locked --all-targets
vx cargo test --locked --all-targets
vx cargo clippy --locked --all-targets -- -D warnings
```

## License

MIT

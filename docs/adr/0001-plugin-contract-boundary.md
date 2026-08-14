# ADR 0001: Plugin contract boundary

- Status: Accepted
- Date: 2026-08-15

## Context

Software Use needs the Cordis composition properties that made the DSH design
useful: services shared through a context, typed events, dependency-aware
activation, and reversible lifecycle effects. It also needs to support Rust,
Python, DCC sidecars, UI-control providers, and transports without making one
language ABI or package system the architecture.

Loading arbitrary Rust dynamic libraries would couple every extension to
compiler, target, allocator, and crate-version details. Treating manifest
permissions as authority would also move the trust boundary into untrusted
package metadata.

## Decision

`PluginManifest` is a bounded, declarative SDK contract. A plugin is a semantic
and lifecycle unit that declares:

- versioned services it provides, requires, or optionally consumes;
- exact typed event contracts and their dispatch modes;
- capability contracts it provides or requires;
- permission requests with bounded declarative constraints; and
- either a trusted `StaticFactory` execution model or a versioned
  `IsolatedProcess` protocol.

The SDK does not define a Rust DLL/`cdylib` ABI, discover entrypoints, load
packages, grant permissions, or execute plugins. A registry binds catalog
artifacts to factory IDs or process entrypoints. A runtime is the trust root: it
admits manifests, grants a subset of requested authority, orders dependencies,
activates plugins, leases contributed services/listeners, and reverses effects
during draining and disposal.

Manifest deserialization rejects unknown fields and validates schema version,
collection bounds, duplicates, required/optional overlap, and event contract
conflicts. Permission constraints are size-, depth-, collection-, node-, key-,
and string-bounded JSON objects. A request never implies a grant.

## Consequences

- Static plugins are straightforward and type-safe, but are trusted in-process
  code and cannot be force-terminated safely.
- Untrusted, Python, or independently terminable plugins use supervised process
  protocols and can expose only wire-adapted services.
- Package formats, MCP, DCC-MCP, CUA, policy, telemetry, and schema validators
  remain replaceable plugins instead of privileged core behavior.
- Runtime lifecycle and dependency semantics can evolve independently of
  package catalogs while sharing one validated manifest boundary.

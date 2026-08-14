# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- Establish transport-neutral capability, provider, invocation, event, error,
  and cancellation contracts.
- Enforce explicit schema dialects, bounded metadata/errors, state-valid wire
  evidence, provider-version receipts, and owner-only cancellation authority.
- Add bounded Cordis-style plugin manifests for services, typed events,
  capabilities, permission requests, and static or isolated execution.
- Validate event-page identity, ordering, cursor, and terminal invariants during construction and
  deserialization.
- Avoid echoing rejected identifier values in diagnostics.

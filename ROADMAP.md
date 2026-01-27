# HalvexPOS Core – Roadmap

This roadmap outlines the planned evolution of **HalvexPOS Core**, the open-source, event-sourced POS engine written in Rust.  
The goal is to keep the core **stable, auditable, and extensible**, while enabling premium products and integrations to be built on top.

Dates are indicative; stability and correctness take priority over speed.

---

## 🎯 Vision

HalvexPOS Core provides:
- A deterministic, event-sourced POS engine
- A stable wire protocol (commands/events)
- Clear host integration contracts
- Strong guarantees for auditability and correctness

The core **does not** include:
- UI
- Networking
- Databases
- Payment processor SDKs
- Vendor-specific integrations

Those live in host applications or commercial layers.

---

## ✅ MVP (v1.0) — Core Complete

**Status:** Implemented / In Progress Stabilisation

### Core Domains
- Orders
- Checks (bills)
- Payments (authorise, capture, refund, void)
- Tables & table sessions
- Split checks / multi-aggregate workflows

### Architecture
- Event-sourced aggregates
- Unified `CommandEnvelope` / `EventEnvelope`
- Deterministic routing engine
- Stable serde schema (`kind` / `data`)
- Schema versioning with compatibility rules

### Projections
- Check totals & balance helpers
- Table availability helpers
- Kitchen Display System (KDS) projection
- Kitchen ticket routing & delta printing support

### Quality & Safety
- Unit tests for all domains
- Property-based tests for invariants
- Abuse/fuzz tests (never panic, only `PosError`)
- Clear audit trails (reasons + actor_id)

### OSS Readiness
- Apache-2.0 license
- SECURITY.md, CONTRIBUTING.md
- CI (fmt, clippy, test)
- Golden integration example
- Storage contract documentation

---

## 🧱 v1.x — Hardening & Adoption (Next)

**Goal:** Make HalvexPOS Core easy and safe to adopt in production systems.

### API & Compatibility
- Finalise public API surface
- Add `#[non_exhaustive]` where appropriate
- Lock JSON schema with snapshot tests
- Publish first tagged release

### Storage & Performance
- Snapshot helpers for aggregates & KDS
- Rebuild helpers from snapshots + tail events
- Document recommended snapshot strategies

### Developer Experience
- Host integration skeleton (in-memory store example)
- More golden examples (restaurant vs café)
- Better error codes & diagnostics

---

## 🔌 v2.0 — Extensibility & Ecosystem

**Goal:** Enable a rich ecosystem without bloating the core.

### Extension Points
- Plugin hooks for:
  - Pricing rules
  - Tax engines
  - Loyalty logic
- Explicit extension APIs (no internal coupling)

### Multi-Schema Support (Optional)
- Support two concurrent schema versions
- Compatibility layer for older clients
- Migration tooling guidance

### Observability
- Event metadata conventions (trace IDs, sources)
- Metrics hooks (host-implemented)

---

## 💼 Commercial / Non-Core (Out of Scope for OSS Core)

These are **intentionally not part of HalvexPOS Core**:

- Payment processor SDKs (Stripe, Adyen, Square, etc.)
- Fiscal/tax compliance per country
- Reporting dashboards
- Inventory management
- Back-office UIs
- Networking / APIs

These belong in:
- Host applications
- Separate open-source projects
- Commercial offerings built on top of the core

---

## 🧭 Long-Term Direction

- Treat the core like a database engine:
  - Stable
  - Predictable
  - Boring (in a good way)
- Favour **explicit contracts over magic**
- Optimise for correctness, auditability, and trust

---

## 🤝 Contributing & Governance

- Breaking changes require:
  - Schema version bump
  - Migration notes
  - Clear justification
- New commands/events should be proposed via issues
- Core maintainers prioritise stability over feature growth

---

*HalvexPOS Core is the foundation.  
Everything else is built on top.*

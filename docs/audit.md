# Audit Metadata

This core supports audit-friendly metadata without binding to storage or transport.

## Actor ID

`actor_id` is an optional field on both `CommandEnvelope` and `EventEnvelope` that should be supplied by the host:

- Staff user ID (cashier, bartender, manager)
- System actor (auto-discounts, integrations)
- Device-specific actor (kiosk, tablet)

When a `CommandEnvelope` includes `actor_id`, the core propagates it to every emitted `EventEnvelope`.

## Reasons

Sensitive actions should include a human-readable `reason` string. This makes audit logs explainable and defensible.

Current actions carrying `reason` end-to-end:

- Order item removal
- Check line removal
- Discount application
- Service charge application
- Payment voids
- Refunds
- Table unassignment

## Host recommendations

- Always supply `actor_id` on command envelopes.
- Store event envelopes exactly as emitted for audit trails.
- Preserve `reason` strings and include UI prompts for staff actions.

## Backwards compatibility

All audit fields are optional with serde defaults, so older JSON remains compatible.

# eal-interfaces

Canonical Rust, JSON Schema, OpenAPI, AsyncAPI, and PostgreSQL contracts for
[Embedded Alerts](https://github.com/embedded-alerts).

Embedded Alerts is a Rust-first, API-first alternative to inbox-only keyword alerts.
Registered users define interests and delivery targets. Workers ingest explicitly
configured search-provider queries, feeds, sitemaps, pages, or APIs; store immutable
URL revisions and model-versioned vectors; create explainable matches; and enqueue
idempotent notifications.

## Contract flow

1. Register a tenant-owned source with `POST /v1/sources`.
2. Fetch it with conditional requests and a bounded, SSRF-safe crawler.
3. Canonicalize each page URL and store a new `eal_source_revisions` row only when
   the normalized content hash changes.
4. Embed the revision in one declared `eal_embedding_spaces` space.
5. Evaluate immutable `eal_alert_rule_revisions` using lexical and semantic scores.
6. Persist a deterministic `eal_matches.identity_key` before delivery.
7. Create durable `eal_delivery_attempts` with provider idempotency keys.
8. Surface matches through the Mash/HTMX, Leptos, and Dioxus Rust clients.

## Important invariants

- Every user-owned or content-owned row carries `tenant_id`.
- PostgreSQL row-level security is forced on all tenant tables.
- A request or worker transaction must set `SET LOCAL app.tenant_id = '<uuid>'`.
- Vectors from different providers, model versions, dimensions, or normalization
  strategies are never compared.
- Content revisions and rule revisions are append-only; match evidence and terminal
  delivery receipts are retained, while retries create new attempt rows.
- Delivery configuration contains only an opaque secret-manager reference; API
  responses never return webhook credentials or provider tokens.
- Re-fetching unchanged content is a no-op, and retrying work cannot create a second
  logical match.

## Layout

- `src/lib.rs` and `src/semantic.rs` — canonical serializable Rust types and validation.
- `schemas/` — JSON Schema 2020-12 wire contracts.
- `openapi.yaml` — authenticated HTTP control-plane contract.
- `asyncapi.yaml` — tenant- and user-filtered WebSocket events.
- `sql/001_initial.sql` — legacy bootstrap schema.
- `sql/002_semantic_alerts_v2.sql` — normalized tenant, source, revision, vector,
  match, and delivery model.

YAML is canonical for OpenAPI and AsyncAPI. JSON mirrors are generated as release
artifacts instead of being hand-maintained in source control.

## Validation

```bash
python3 scripts/verify_repo.py
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The v2 migration deliberately leaves the original bootstrap tables intact. New
services must use the `eal_*` tables; data migration and removal of legacy tables
should happen only after restart, tenant-isolation, and delivery-idempotency canaries
pass.

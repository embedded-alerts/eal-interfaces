# eal-interfaces

Canonical Rust, JSON Schema, OpenAPI, AsyncAPI, and PostgreSQL contracts for Embedded Alerts.

**Product:** Embedded Alerts — Embedding-based alerting for semantically relevant new information.

Define semantic alert rules, ingest source documents, compare embeddings, rank matches, and deliver explainable notifications.

## Safety and production boundary

Similarity scores are ranking signals, not truth guarantees. Production ingestion must respect source terms, robots rules, privacy requirements, retention limits, and notification consent.

This repository is an executable bootstrap, not a production deployment. Before live
use, add authentication, tenant authorization, rate limits, durable migrations,
observability, backups, incident response, dependency review, and secret management.
## Contract authority

- `src/lib.rs` is the Rust model and validation surface.
- `schemas/` contains JSON Schema Draft 2020-12 wire contracts.
- `openapi.yaml` defines REST endpoints.
- `asyncapi.yaml` defines WebSocket event envelopes.
- `sql/` provides a deny-by-default PostgreSQL/Supabase migration baseline.
- `fixtures/` provides cross-language conformance examples.

Downstream services should consume a tagged release and run fixture compatibility
tests before deployment.

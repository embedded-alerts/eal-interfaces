# Embedded Alerts contract architecture

## Product boundary

Embedded Alerts monitors sources that a tenant explicitly registers. The initial
crawler is not a general-purpose search-engine spider: it accepts registered
search-provider queries, RSS, Atom, JSON Feed, sitemap, web-page, and API roots,
follows only configured scope rules, and must obey robots policy, per-host
concurrency, response-size, MIME-type, and fetch-budget limits.

The platform is split into four runtime concerns:

- **Control plane (`eal-api`)** — authenticated source, rule, match, and delivery
  APIs backed by PostgreSQL/SeaORM.
- **Ingestion plane (`eal-sync`)** — due-source scheduling, conditional fetch,
  canonicalization, extraction, hashing, revision persistence, embedding, and match
  candidate creation.
- **Domain core (`eal-libs`)** — deterministic URL normalization, lexical scoring,
  cosine scoring, match identity, cooldown, retry, and delivery decisions.
- **User experiences** — Mash/Maud/HTMX server-rendered UI plus Leptos and Dioxus
  Rust clients. None of these clients owns business state.

## Identity and revision model

A source connector is unique by the SHA-256 digest of its kind, canonical endpoint,
and canonical public configuration. This permits multiple search queries against the
same provider endpoint without duplicating equivalent connector configurations.
Credentials are represented only by an opaque secret-manager reference.

A page URL is canonicalized before persistence. A source document is unique by
`(tenant_id, canonical_url)`. It can be discovered by multiple registered sources
through `eal_source_document_sources`.

A fetch produces a normalized-text SHA-256 digest. The pair
`(document_id, content_sha256)` is unique, so retrying an unchanged page does not
create another revision. A changed page creates a new immutable revision linked to
its predecessor, and `current_revision_id` advances transactionally.

Alert matching inputs are also immutable. Editing query text, terms, weights,
thresholds, source filters, cooldown, or embedding space creates a new
`eal_alert_rule_revisions` row. Historical matches always point to the exact rule
and source revisions used to produce their scores.

## Embedding spaces

`eal_embedding_spaces` names the provider, model, model version, dimensions, and
normalization strategy. Both page vectors and query vectors reference one space.
Database triggers reject vectors whose dimensions disagree with that space.

The base migration intentionally does not create one global HNSW index because a
single unbounded `vector` column can hold multiple dimensions while one pgvector
index cannot safely mix those spaces. Operations should create a partition-local or
partial index for each approved production space and route search by
`embedding_space_id`.

A lexical-only match is valid and records a null embedding space. Semantic scores
are computed only when the rule and content vectors share the exact same space.

## Match and notification identity

The canonical logical match identity is the SHA-256 digest of:

- tenant,
- immutable rule revision,
- immutable source revision,
- embedding space or `lexical-only`, and
- normalized content hash.

`eal_matches` has a unique constraint on that identity. Workers must insert the
match before scheduling delivery. Concurrent inserts therefore collapse into one
logical match while preserving every source and score revision.

Cooldown and grouping suppress notification delivery, not evidence. Suppressed
matches remain queryable with their explanation and `suppressed_until` value.

Every provider call is represented by an `eal_delivery_attempts` row. Its in-flight
state may advance to one terminal outcome, while retry and manual replay create new
attempt rows; terminal response and error evidence is never erased. Provider
credentials live outside PostgreSQL and are addressed through `config_reference`.

## Tenant isolation

All tenant-owned tables force PostgreSQL row-level security. HTTP requests and
worker transactions derive tenant and user identifiers from verified Shared-Auth
claims, then set the tenant in a transaction-local GUC:

```sql
begin;
set local app.tenant_id = '00000000-0000-0000-0000-000000000000';
-- tenant-scoped reads and writes
commit;
```

A connection must never be returned to the pool while a session-level tenant value
is set. Use `SET LOCAL` inside a transaction only.

RLS is a second boundary, not a substitute for ownership checks. The API must also
ensure users can read and mutate only their own rules, delivery targets, and match
read-state unless an explicit tenant-admin permission is present.

## Event delivery

The WebSocket contract carries sequence-numbered, tenant- and user-filtered events.
Sockets are an acceleration path, not the source of truth. Clients reconnect using
bounded HTTP list cursors and then resume live updates. The API must never use a
process-global broadcast that leaks one tenant's events to another tenant.

## Rollout order

1. Apply the v2 migration in a non-production database.
2. Generate SeaORM entities and implement transaction-local tenant context.
3. Certify unchanged-ingest, changed-revision, dimension-mismatch, and duplicate
   match tests.
4. Implement delivery outbox workers and retry/dead-letter canaries.
5. Replace synthetic UI data with authenticated API clients.
6. Run restart and cross-tenant isolation tests in `embedded-alerts-test`.
7. Enable production ingestion first; enable external notification providers only
   after idempotency and cooldown canaries pass.

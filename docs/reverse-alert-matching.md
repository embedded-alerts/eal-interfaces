# Reverse semantic alert matching

Tracking: `DEN-3461`

## Product boundary

Embedded Alerts and historical Embedded Search share embedding/model-space contracts but have different hot indexes.

- **Embedded Alerts:** active saved alert-rule revision vectors are the hot index. Each new/materially changed page embedding is a transient query over that bounded active-alert index.
- **Embedded Search:** persisted page vectors support retrospective semantic search and are optional for real-time alert delivery.

A real-time match MUST NOT require a durable page-vector row.

## Worker handoffs

`AlertRuleEmbeddingUpsertRequest` installs one model-versioned embedding for an immutable alert-rule revision. The service must verify that the revision belongs to the route rule, is the rule's current active revision, is enabled, and uses the requested embedding model before accepting the vector.

`ReverseAlertPageRequest` carries a fetched page plus its compact first-pass embedding. The request is authenticated as crawler/embedding-worker traffic, re-validates source scope, canonicalizes both URLs, deduplicates normalized content, and persists the page revision before semantic candidate generation.

The vector in `ReverseAlertPageRequest` is transient by default. Implementations may retain it only under an explicit historical-search/retention policy.

## Matching invariants

Before comparing two vectors, model, model version, dimensions, and normalization must match exactly. The matcher then:

1. filters to the tenant's active and enabled alert-rule revisions;
2. applies source filters before candidate admission;
3. performs bounded top-K candidate generation (`1..=500` at the API contract; production defaults should be lower);
4. computes/rechecks exact cosine similarity for admitted candidates;
5. rejects candidates below the immutable rule revision's threshold;
6. writes immutable match evidence using tenant, alert rule/revision, page revision/content hash, and model-space provenance;
7. hands the durable candidate to the DEN-3460 delivery state machine rather than notifying directly.

Candidate generation must remain bounded under overload. A full or unavailable downstream queue must apply backpressure/retry policy instead of growing process memory without bound.

## Evidence and retention

Reverse-stream matches bind to an immutable alert embedding and a SHA-256 fingerprint/provenance record for the transient page embedding. Historical matches may continue to bind to a persisted page embedding. Both forms use the same immutable page revision and canonical match identity.

A page-vector retention policy should distinguish at least:

- `transient`: do not persist the vector after reverse matching;
- `matched`/warm evidence: optional bounded retention when a product requirement needs the vector itself;
- `historical`: explicit Embedded Search retention with TTL/storage-budget controls.

Do not silently promote transient traffic into historical storage.

## User limit

The durable rule boundary enforces at most **10 enabled active embedded searches per owner subject**. The check must be concurrency-safe; two simultaneous creates must not both pass a stale count.

## Observability

Record bounded, non-content-cardinality metrics for page discovery/ingestion, duplicate suppression, embedding token/input size, alert-index candidate count, exact-rerank count, match count, queue lag, provider throttling, transient versus persisted vector counts, and retention expiry. Never attach raw page/query text or vectors as metric labels.

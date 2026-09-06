# External index discovery strategy

Embedded Alerts can reduce crawl cost and improve discovery latency by querying an existing public index, but the external index is only a **URL nomination layer**. It is not the source of truth for content, authorization, canonical identity, revisions, embeddings, or match evidence.

The provider-neutral handoff is defined by `contracts/external-index-candidates.schema.json`.

## Admission pipeline

Every external candidate follows the same admission pipeline as a sitemap, RSS/Atom/JSON feed, seed URL, or same-domain link:

```text
external index candidate
        │
        ├── tenant + source policy revision exists and is enabled
        ├── parse HTTPS URL; reject credentials, IP literals, fragments, invalid ports
        ├── exact-host/subdomain policy check
        ├── canonicalization + tracking-parameter policy
        ├── per-source dedupe and page/host budgets
        ├── DNS resolution; all selected destinations must be public
        ├── pinned no-proxy request with manual redirect validation
        ├── robots/content-type/size/timeout checks
        ├── Embedded Alerts extracts and hashes normalized page text
        └── revision + semantic input/vector pipeline
```

A provider snippet, digest, score, or cached body can be retained as discovery provenance, but it must never become the indexed page body or the evidence shown to a user without a successful Embedded Alerts fetch.

## Recommended first adapters

### Common Crawl

Use Common Crawl's index to enumerate recently observed URLs for an allowed host or path prefix. Pin every request to a named crawl collection and store that collection identifier with the candidate batch. This is useful for broad public-web discovery and replay, but recency varies by domain.

### Search API

Use a commercial or self-hosted search API only for low-volume, high-value discovery. Generate provider queries from source/domain policy and broad rule concepts without sending the user's raw private alert text when avoidable. Hash the generated query for audit. Treat provider ranking only as a crawl-priority hint.

### Custom public index

Organizations may register a custom index endpoint for their own public corpus. It must return the same candidate-batch contract. Endpoint credentials remain server-side and are never stored in browser contracts or candidate provenance.

## Query privacy

External discovery should primarily use domain/path/time constraints. Semantic alert evaluation happens after Embedded Alerts fetches and embeds the page. When concept terms are useful for discovery:

- prefer coarse generated concepts over the verbatim user query;
- remove personal or tenant-confidential details;
- record a query fingerprint, adapter version, collection, and timestamp;
- support a per-source setting that disables concept-bearing external queries entirely.

## Scheduling and budgets

Each source keeps separate budgets for:

- external candidate requests;
- admitted URLs;
- network fetches;
- bytes;
- revisions;
- embedding inputs;
- provider tokens/cost.

A provider outage or rate limit pauses that discovery mode only. Seed, sitemap, feed, and same-domain discovery continue independently. Cursors are immutable attempt state so a retry cannot skip or duplicate pages silently.

## Dedupe and freshness

Canonical URL identity is still computed by Embedded Alerts. Candidate records dedupe by tenant, source policy revision, canonical URL, adapter, collection, and provider document identifier where available. A previously indexed URL may be re-fetched according to HTTP validators, source cadence, change probability, or an explicit freshness policy; a provider score alone never creates a new page revision.

## Security and compliance gates

Before enabling an adapter in production:

1. certify that it cannot submit arbitrary URLs outside the source policy;
2. bound provider response size, item count, cursors, and provenance fields;
3. redact secrets and disallow provider-supplied authorization headers per candidate;
4. verify retry/backoff and quota behavior;
5. test DNS rebinding and redirect escape attempts after admission;
6. document provider retention/privacy terms;
7. add tenant-isolation and replay tests;
8. keep indexing limited to public pages and honor removal/takedown policy.

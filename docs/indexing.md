# Domain-scoped indexing

Embedded Alerts uses an owned, domain-scoped index as the authority for matching.
External search APIs, feeds, sitemaps, and social or internal firehoses may contribute
candidate URLs, but candidates never become matches directly.

Every candidate passes the same pipeline:

1. Resolve the tenant-owned source policy.
2. Canonicalize the URL and reject non-HTTP(S), credentialed, local, private, or
   out-of-scope hosts and paths.
3. Resolve DNS immediately before each request and each redirect to prevent SSRF and
   DNS rebinding.
4. Apply robots.txt, per-host concurrency, request budgets, response-size limits, and
   content-type allowlists.
5. Extract readable text, compute a normalized SHA-256 content identity, and create a
   linked revision only when content changed.
6. Persist embedding model, version, dimensions, normalization, and generation time.
7. Compare only vectors with identical provenance and create explainable match
   candidates; a separate delivery state machine decides whether and where to notify.

This hybrid approach provides broader discovery without trusting an opaque third-party
index for tenancy, freshness, canonical identity, semantic score, or delivery decisions.

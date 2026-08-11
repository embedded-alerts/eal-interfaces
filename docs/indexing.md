# Domain-scoped indexing

Embedded Alerts owns the authoritative semantic index for every alert decision.

External search APIs, RSS/Atom/JSON feeds, sitemaps, manual submissions, and link
discovery can suggest candidate URLs. They are discovery accelerators, not a trusted
index and not a notification source. A candidate is eligible for matching only after
the Rust ingestion worker:

1. loads the authenticated tenant's source policy;
2. canonicalizes the URL and rejects credentials, literal IPs, non-default ports,
   disallowed hosts, disallowed paths, and unsupported media types;
3. resolves DNS immediately before every request and redirect and rejects loopback,
   link-local, private, multicast, documentation, and otherwise non-public addresses;
4. applies robots.txt, per-host concurrency, timeout, response-size, depth, and
   per-run page budgets;
5. extracts normalized text and writes a new immutable revision only when its
   SHA-256 content identity changed;
6. records the embedding provider, model, model version, dimensions, normalization,
   and generation time; and
7. evaluates lexical and semantic criteria in one tenant and one embedding space,
   creating a deterministic match candidate rather than notifying directly.

Delivery is downstream. Cooldown, grouping, approval, provider idempotency, retries,
receipts, dead-letter state, and replay remain in the DEN-3460 state machine.

## Why not rely only on an existing search index?

A search provider can improve discovery coverage and reduce crawl frontier cost, but
it cannot be the source of truth for tenant scope, canonical identity, current page
content, model provenance, semantic score, or notification idempotency. The hybrid
design uses provider results as candidates while retaining local verification and
better matching.

## Initial production posture

The initial source set is deny-by-default:

- exact public DNS hosts only; no wildcard hosts;
- subdomains only when explicitly enabled;
- explicit path prefixes;
- robots compliance cannot be disabled;
- bounded response sizes and request timeouts;
- no direct crawler-to-delivery path; and
- production remains blocked until Shared Auth claims, PostgreSQL migrations,
  crawler SSRF/DNS-rebinding canaries, restart isolation, and delivery canaries pass.

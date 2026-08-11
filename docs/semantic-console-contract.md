# Semantic console contract

`contracts/semantic-console.schema.json` is the browser/SDK boundary shared by the MASH/HTMX, Leptos, Dioxus, CLI, desktop, and mobile clients.

## Non-negotiable properties

1. **The browser never handles raw vectors.** A client submits a complete natural-language query. The API loads the immutable alert-rule revision, derives query views, calls the pinned embedding provider, checks model compatibility, searches, and stores candidates.
2. **Only configured public domains are eligible.** External indexes can suggest candidate URLs, but an Embedded Alerts worker still applies the tenant's domain policy, resolves public addresses, fetches the page itself, canonicalizes the URL, and records provenance.
3. **Alert rules are revisioned.** Editing a query creates a new immutable revision with a pinned embedding-space identity. Existing match candidates continue to reference the exact rule and page revisions that produced them.
4. **Matches are explainable.** Candidate responses expose score components and bounded source text such as the best complete sentence, title/heading, keywords, and proper nouns/entities. They never expose vector values.
5. **Evaluation and delivery are separate.** Evaluation creates deterministic candidate records. Approval only makes a candidate eligible for the DEN-3460 delivery state machine; it does not send directly from the search request.
6. **Concurrent review is explicit.** Candidate actions carry `expected_state` so stale browser tabs cannot silently overwrite a later approval, suppression, delivery, or failure transition.

## Shared workflows

### Register a source

The client submits `DomainSourceCreate`. The server additionally rejects:

- IP literals, localhost/internal hostnames, and non-public DNS results;
- URL credentials, fragments, and unsupported schemes;
- seed URLs outside the exact host/subdomain policy;
- off-policy redirects;
- a disabled robots policy in production;
- page budgets or priorities outside the schema bounds.

The returned `DomainSourceSummary` describes operational state without exposing infrastructure secrets.

### Create an alert-rule revision

The client submits `AlertRuleRevisionCreate` with a complete interest statement, for example:

> Notify me when a Colombian renewable-energy company launches tooling for engineering teams.

The server persists the statement in an immutable revision. `QueryPreview` may show the complete query, companion keywords, proper nouns/entities, and bounded weighted input text. It reports `vector_values_exposed: false` as a structural invariant.

### Evaluate

A client invokes evaluation for a stored rule revision using `EvaluationRequest`. The request can optionally bound evaluation to known page revision IDs, but it cannot include query text overrides, provider names, model identifiers, dimensions, or vector values.

The server:

1. authorizes the tenant, rule, revision, and sources;
2. loads the immutable query and pinned embedding space;
3. derives and embeds query inputs;
4. searches only page revisions in the exact same embedding space;
5. combines semantic, lexical, entity, recency, and source-priority evidence;
6. persists deterministic candidates;
7. returns `MatchCandidateSummary` records.

Provider failure, missing production configuration, model/dimension mismatch, non-finite values, zero vectors, tenant mismatch, or stale rule state fails closed and creates no candidate.

### Review a candidate

`CandidateReviewAction` supports `approve`, `suppress`, and `dismiss` with an `expected_state`. The API performs the transition transactionally and records actor, time, reason, and prior/new state. An approved candidate still flows through DEN-3460 suppression, cooldown, grouping, destination authorization, retry, receipt, and dead-letter behavior.

## Code generation and compatibility

The JSON Schema is the source for generated clients and form/view models. Additive optional fields are backward compatible. Removing fields, narrowing enums/ranges, changing state semantics, exposing vectors, or changing mutable/immutable identity rules requires a new contract version and coordinated migration.

Every renderer should pass the same contract scenarios:

- source accepted and rejected;
- query preview preserves the complete query first;
- candidate evidence is readable without vectors;
- model-space mismatch fails closed;
- stale candidate action is rejected;
- tenant A cannot observe or mutate tenant B;
- approval does not bypass delivery policy.

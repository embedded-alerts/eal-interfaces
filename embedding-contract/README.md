# EmbeddedAlerts embedding contract v2

This product-owned contract covers web pages, tracked changes, and next-generation web alerts. It supports provider
outputs up to 4,096 dimensions and stores every vector in exactly 4,100 slots.
The padding operation only appends zeros; it never truncates a source vector or
pretends the padding contains learned information.

## Provider provenance

`embeddingProvider` records the service or model family that actually emitted
the vector. Anthropic is deliberately not an embedding-provider value because
Anthropic does not offer an embedding model. When Claude produced or transformed
the source material, record `generationProvider = anthropic` and record the
actual embedding provider separately (for example, Voyage, OpenAI, or Google).

OpenAI `text-embedding-3-large` defaults to 3,072 dimensions. A 1,536 value is
valid only when the caller explicitly requests the shortened output.

## Storage and indexing

- PostgreSQL stores `vector(4100)`. Since full-precision HNSW is limited to
  2,000 dimensions (and half precision to 4,000), candidate generation uses a
  4,100-bit binary-quantized HNSW expression and exact full-vector cosine
  re-ranking. Lexical ranking uses a GIN-indexed `tsvector`.
- CockroachDB stores `VECTOR(4100)`, uses a tenant-prefixed native vector
  index, and combines it with a GIN-indexed `TSVECTOR`.
- Every row has a deterministic embedding-space identity (provider, model,
  original dimension, and normalization). Search requires that identity, so
  padding never causes mathematically unrelated model spaces to be compared.
- Tenant predicates and forced row-level security are applied before candidate
  rows become visible. Alert-match rows carry a deduplication key so retries do
  not emit duplicate notifications.

## Code-first and DB-first agreement

This interfaces repository keeps Microsoft TypeSpec and JSON Schema beside the
Rust model. The product ORM core owns forward-only migrations, while this interface contract remains wire-compatible.
Run `node scripts/verify-embedding-contract.mjs` and compile the TypeSpec before
publishing a change.

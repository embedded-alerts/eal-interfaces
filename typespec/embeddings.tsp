// GENERATED FILE - do not edit by hand.
// Regenerate: node scripts/embeddings/generate.mjs
// Contract: ores-embedding-contract 2026.08.29 (sha256 39b8e1599d227a97f2362fdb6d3495dd38853b3345d7b05e90b64088884f9250)
//
// TypeSpec is the interface-level statement of the same contract the SQL in
// db/schema enforces at the storage level. When they disagree, the SQL wins and
// this file is stale - the generator is what keeps them from disagreeing.

import "@typespec/json-schema";
using TypeSpec.JsonSchema;

@jsonSchema
namespace EmbeddedAlerts.Embeddings;

@doc("Embedding provider. Anthropic ships no first-party embedding model; the voyage provider is what "Anthropic embeddings" means in our code.")
union EmbeddingProvider {
  "baai",
  "google",
  "intfloat",
  "nvidia",
  "openai",
  "qwen",
  "voyage",
}

@doc("provider:modelName:modelVersion. Two rows may only be compared when this matches.")
union EmbeddingModelKey {
  "openai:text-embedding-3-large:1",
  "openai:text-embedding-3-small:1",
  "openai:text-embedding-ada-002:2",
  "voyage:voyage-3-large:1",
  "voyage:voyage-3.5:1",
  "voyage:voyage-3.5-lite:1",
  "voyage:voyage-code-3:1",
  "google:gemini-embedding-001:1",
  "google:text-embedding-004:1",
  "google:text-multilingual-embedding-002:1",
  "qwen:Qwen3-Embedding-8B:1",
  "qwen:Qwen3-Embedding-4B:1",
  "qwen:Qwen3-Embedding-0.6B:1",
  "nvidia:NV-Embed-v2:1",
  "baai:bge-en-icl:1",
  "baai:bge-m3:1",
  "intfloat:e5-mistral-7b-instruct:1",
}

@doc("What the embedded row describes in embedded-alerts.")
union EntityKind {
  "page",
  "page_section",
  "feed_item",
  "alert_rule",
  "snapshot_diff",
  "extracted_claim",
}


@doc("""
Canonical stored embedding: exactly 4100 components, left-aligned to nativeDims,
zero past nativeDims, L2-normalized to unit length within 0.001. The zero pad is
norm-preserving, so a 1536-component OpenAI vector padded to 4100 ranks exactly as it
would at its native width.
""")
@minItems(4100)
@maxItems(4100)
model EmbeddingVector is float32[];

@doc("One row of the model registry, mirroring eal.embedding_models.")
model EmbeddingModel {
  modelKey: EmbeddingModelKey;
  provider: EmbeddingProvider;
  @maxLength(160) modelName: string;
  @maxLength(40) modelVersion: string;

  @doc("Native width of this model before padding.")
  @minValue(1) @maxValue(4100) nativeDims: int32;

  @doc("Matryoshka-trained: a prefix of the vector approximates the whole. Non-MRL models are excluded from the prefix ANN index because a prefix of them is an arbitrary coordinate subset.")
  mrl: boolean;

  @doc("mrl AND nativeDims >= 1024. Governs eligibility for the halfvec(1024) prefix index.")
  mrlPrefixValid: boolean;

  @doc("False means the client must L2-normalize before insert or the unit-norm CHECK rejects the row.")
  normalizedByProvider: boolean;

  family: string;
  notes?: string;
}

@doc("Turn the web into embeddings and run a semantic replacement for Google Alerts: a saved alert rule is itself an embedding, and a crawl fires when a new page or page section lands close enough to it.")
model PageEmbedding {
  @format("uuid") id: string;
  @format("uuid") workspaceId: string;

  entityKind: EntityKind;
  @format("uuid") entityId: string;

  modelKey: EmbeddingModelKey;
  provider: EmbeddingProvider;
  @maxLength(160) modelName: string;
  @maxLength(40) modelVersion: string;
  @minValue(1) @maxValue(4100) nativeDims: int32;
  mrlPrefixValid: boolean;

  embedding: EmbeddingVector;

  @doc("Weight A in the tsvector: titles, names, human tags.")
  @maxLength(2048) titleText: string;
  @doc("Weight B: human descriptions, captions, abstracts.")
  @maxLength(8192) summaryText: string;
  @doc("Weight D: full body, transcript or OCR text.")
  @maxLength(65536) bodyText: string;

  @doc("sha256 of the exact text that was embedded. Lets a re-embed pass skip unchanged rows and gives exact-duplicate detection a path that never touches a vector.")
  @pattern("^[0-9a-f]{64}$") contentSha256: string;

  @maxLength(2048) sourceUri?: string | null;
  metadata: Record<unknown>;


  @format("date-time") generatedAt: string;
  @format("date-time") createdAt: string;
  @format("date-time") updatedAt: string;
}

@doc("A hybrid retrieval request. queryText drives the lexical half, queryEmbedding the vector half; supplying only one is legal and degrades to that half alone.")
model HybridSearchRequest {
  @format("uuid") workspaceId: string;
  modelKey: EmbeddingModelKey;
  @maxLength(4096) queryText?: string | null;
  queryEmbedding?: EmbeddingVector | null;
  entityKinds?: EntityKind[];
  @minValue(1) @maxValue(4000) candidateCount?: int32 = 400;
  @minValue(1) @maxValue(200) resultCount?: int32 = 20;
}

@doc("One fused hit. vectorRank and lexicalRank are null when that half did not retrieve the row; rrfScore is the reciprocal-rank fusion of whichever halves did.")
model HybridSearchHit {
  @format("uuid") id: string;
  entityKind: EntityKind;
  @format("uuid") entityId: string;
  titleText: string;
  summaryText: string;
  sourceUri?: string | null;
  metadata: Record<unknown>;
  modelKey: EmbeddingModelKey;
  vectorRank?: int32 | null;
  lexicalRank?: int32 | null;
  rrfScore: float64;
  @minValue(-1) @maxValue(1) cosineSimilarity?: float64 | null;
  lexicalScore?: float64 | null;
}

import "@typespec/json-schema";
using TypeSpec.JsonSchema;

@jsonSchema
namespace EmbeddedAlerts.ReverseAlertMatching;

union VectorNormalization {
  "none",
  "l2",
  "unit_length",
}

model EmbeddingPayload {
  @minLength(1) @maxLength(256) model: string;
  @minLength(1) @maxLength(256) model_version: string;
  @minValue(1) @maxValue(65535) dimensions: int32;
  normalization: VectorNormalization;
  @minItems(1) @maxItems(65535) values: float32[];
}

@doc("Trusted embedding-worker handoff for one immutable active alert-rule revision.")
model AlertRuleEmbeddingUpsertRequest {
  @format("uuid") alert_rule_revision_id: string;
  embedding: EmbeddingPayload;
}

@doc("A newly fetched page used as a transient query against the active alert-rule vector index. The page vector is not required to be durably stored.")
model ReverseAlertPageRequest {
  @format("uuid") source_id: string;
  @format("uri") url: string;
  @format("uri") final_url: string;
  @maxLength(4096) title?: string | null;
  @minLength(1) @maxLength(5000000) content_text: string;
  @minLength(1) @maxLength(256) content_type: string;
  @minValue(200) @maxValue(399) http_status: int32;
  @format("date-time") published_at?: string | null;
  @format("date-time") fetched_at: string;
  embedding: EmbeddingPayload;
  @minValue(1) @maxValue(500) max_candidates?: int32 = 100;
}

@doc("Durable match evidence emitted only after model-space/source-scope checks and exact similarity scoring.")
model ReverseAlertCandidate {
  @format("uuid") id: string;
  @format("uuid") alert_rule_id: string;
  @format("uuid") alert_rule_revision_id: string;
  @format("uuid") page_revision_id: string;
  @format("uuid") alert_embedding_id: string;
  similarity: float64;
  threshold: float64;
  @pattern("^[0-9a-f]{64}$") canonical_match_key: string;
}

model ReverseAlertMatchResponse {
  @format("uuid") page_id: string;
  @format("uuid") page_revision_id: string;
  @pattern("^[0-9a-f]{64}$") content_sha256: string;
  changed: boolean;
  page_vector_persisted: boolean;
  candidates: ReverseAlertCandidate[];
}

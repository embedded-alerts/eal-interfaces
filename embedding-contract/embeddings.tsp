/**
 * EmbeddedAlerts embedding contract v2.
 *
 * Code-first consumers compile this TypeSpec. DB-first generation validates
 * the resulting shape against the product-owned SQL and JSON Schema artifacts.
 */
namespace EmbeddedAlerts.EmbeddingV2;

const storageDimensions = 4100;
const maximumSourceDimensions = 4096;

enum EmbeddingProvider {
  OpenAi: "openai",
  Google: "google",
  Voyage: "voyage",
  Qwen: "qwen",
  Nvidia: "nvidia",
  Baai: "baai",
  Custom: "custom",
}

enum GenerationProvider {
  OpenAi: "openai",
  Anthropic: "anthropic",
  Google: "google",
  Qwen: "qwen",
  Nvidia: "nvidia",
  Baai: "baai",
  Custom: "custom",
}

enum EmbeddingPurpose {
  WebpageSearch: "webpage_search",
  ChangeCorrelation: "change_correlation",
  Alert: "alert",
}

enum EmbeddingNormalization {
  Provider: "provider",
  L2: "l2",
  None: "none",
}

model EmbeddingInput {
  embeddingProvider: EmbeddingProvider;
  generationProvider?: GenerationProvider;
  @minLength(1)
  @maxLength(200)
  `model`: string;
  @minItems(1)
  @maxItems(4096)
  values: float32[];
}

model StoredEmbedding {
  @format("uuid")
  tenantId: string;
  @minLength(1)
  @maxLength(80)
  entityKind: string;
  @minLength(1)
  @maxLength(256)
  entityId: string;
  purpose: EmbeddingPurpose;
  embeddingProvider: EmbeddingProvider;
  generationProvider?: GenerationProvider;
  @minLength(1)
  @maxLength(200)
  `model`: string;
  @minValue(1)
  @maxValue(4096)
  originalDimensions: int32;
  @minLength(8)
  @maxLength(320)
  embeddingSpace: string;
  storageDimensions: 4100;
  @minItems(4100)
  @maxItems(4100)
  values: float32[];
  normalization: EmbeddingNormalization;
  searchText?: string;
  @minLength(64)
  @maxLength(64)
  contentHash: string;
  metadata?: Record<unknown>;
}

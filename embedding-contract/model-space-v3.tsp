/** Additive Embedded Alerts embedding model-space v3 wire contract. */
namespace EmbeddedAlerts.EmbeddingV3;

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

enum InputRole {
  Symmetric: "symmetric",
  Query: "query",
  Document: "document",
  Classification: "classification",
  Clustering: "clustering",
}

enum DistanceMetric {
  Cosine: "cosine",
  InnerProduct: "inner-product",
  L2: "l2",
}

enum Normalization {
  Provider: "provider",
  L2: "l2",
  None: "none",
}

enum AnnStrategy {
  HalfvecExact: "halfvec-exact",
  HalfvecMrl: "halfvec-mrl",
  BinaryFullRerank: "binary-full-rerank",
  ExactOnly: "exact-only",
}

enum EmbeddingPurpose {
  WebpageSearch: "webpage_search",
  ChangeCorrelation: "change_correlation",
  Alert: "alert",
}

model EmbeddingProfileV3 {
  @minLength(16)
  @maxLength(512)
  profileKey: string;
  embeddingProvider: EmbeddingProvider;
  generationProvider?: GenerationProvider;
  @minLength(1)
  @maxLength(200)
  `model`: string;
  @minLength(1)
  @maxLength(200)
  modelRevision: string;
  @minValue(1)
  @maxValue(4096)
  sourceDimensions: int32;
  storageDimensions: 4100;
  normalization: Normalization;
  @minLength(1)
  @maxLength(128)
  taskProfile: string;
  @minLength(1)
  @maxLength(128)
  instructionFamily: string;
  @minLength(64)
  @maxLength(64)
  instructionSha256: string;
  @minLength(64)
  @maxLength(64)
  preprocessingSha256: string;
  inputRole: InputRole;
  @minLength(1)
  @maxLength(80)
  modality: string;
  distanceMetric: DistanceMetric;
  annStrategy: AnnStrategy;
  @minValue(0)
  @maxValue(4100)
  annDimensions: int32;
  supportsMrl: boolean;
}

model ComparisonSpaceV3 {
  @minLength(16)
  @maxLength(512)
  spaceKey: string;
  purpose: EmbeddingPurpose;
  queryProfile: EmbeddingProfileV3;
  documentProfile: EmbeddingProfileV3;
  @minValue(1)
  @maxValue(4000)
  lexicalCandidateLimit: int32;
  @minValue(1)
  @maxValue(4000)
  semanticCandidateLimit: int32;
  @minValue(1)
  @maxValue(200)
  resultLimit: int32;
  @minValue(1)
  @maxValue(1000)
  fusionK: int32;
  enabled: boolean;
}

model StoredEmbeddingV3 {
  @format("uuid")
  embeddingId: string;
  @format("uuid")
  tenantId: string;
  @minLength(1)
  @maxLength(80)
  entityKind: string;
  @minLength(1)
  @maxLength(256)
  entityId: string;
  purpose: EmbeddingPurpose;
  @minLength(16)
  @maxLength(512)
  spaceKey: string;
  @minLength(16)
  @maxLength(512)
  profileKey: string;
  inputRole: InputRole;
  @minValue(1)
  @maxValue(4096)
  sourceDimensions: int32;
  storageDimensions: 4100;
  @minItems(4100)
  @maxItems(4100)
  values: float32[];
  @minLength(64)
  @maxLength(64)
  contentSha256: string;
  @minLength(1)
  @maxLength(256)
  sourceRevision: string;
  @minLength(64)
  @maxLength(64)
  sourcePolicySha256: string;
}

model HybridSearchRequestV3 {
  @format("uuid")
  tenantId: string;
  purpose: EmbeddingPurpose;
  @minLength(16)
  @maxLength(512)
  spaceKey: string;
  queryProfile: EmbeddingProfileV3;
  documentProfile: EmbeddingProfileV3;
  @minItems(4100)
  @maxItems(4100)
  queryEmbedding: float32[];
  @maxLength(200000)
  queryText?: string;
  @minLength(64)
  @maxLength(64)
  sourcePolicySha256: string;
  @minValue(1)
  @maxValue(200)
  limit: int32;
}

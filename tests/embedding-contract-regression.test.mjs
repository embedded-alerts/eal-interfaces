import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");
const config = JSON.parse(await read("embedding-contract/generation.json"));
const schema = JSON.parse(await read(config.codeFirst.jsonSchema));
const typeSpec = await read(config.codeFirst.typeSpec);

test("stored and source dimensions cannot drift or become ambiguous", () => {
  assert.equal(config.dimensions.storage, 4100);
  assert.equal(config.dimensions.maximumSource, 4096);
  assert.equal(config.dimensions.padding, "trailing-zero");
  assert.equal(schema.additionalProperties, false);
  assert.equal(schema.properties.storageDimensions.const, 4100);
  assert.equal(schema.properties.values.minItems, 4100);
  assert.equal(schema.properties.values.maxItems, 4100);
  assert.equal(schema.properties.originalDimensions.minimum, 1);
  assert.equal(schema.properties.originalDimensions.maximum, 4096);
  assert.equal(schema.$defs.EmbeddingInput.additionalProperties, false);
  assert.equal(schema.$defs.EmbeddingInput.properties.values.minItems, 1);
  assert.equal(schema.$defs.EmbeddingInput.properties.values.maxItems, 4096);
});

test("embedding and generation provider roles remain distinct", () => {
  const embeddingProviders = schema.properties.embeddingProvider.enum;
  const generationProviders = schema.properties.generationProvider.enum;
  assert.equal(new Set(embeddingProviders).size, embeddingProviders.length);
  assert.equal(new Set(generationProviders).size, generationProviders.length);
  assert(!embeddingProviders.includes("anthropic"));
  assert(generationProviders.includes("anthropic"));
  assert(
    embeddingProviders.some((provider) => !generationProviders.includes(provider)),
    "an embedding-only provider must remain distinguishable",
  );
  assert(
    generationProviders.some(
      (provider) => provider !== null && !embeddingProviders.includes(provider),
    ),
    "a generation-only provider must remain distinguishable",
  );
});

test("identity, provenance, and integrity fields stay mandatory and bounded", () => {
  const required = new Set(schema.required);
  for (const field of [
    "tenantId",
    "entityKind",
    "entityId",
    "purpose",
    "embeddingProvider",
    "model",
    "originalDimensions",
    "embeddingSpace",
    "storageDimensions",
    "values",
    "normalization",
    "contentHash",
  ]) {
    assert(required.has(field), `missing required field: ${field}`);
  }
  assert.equal(schema.properties.embeddingSpace.minLength, 8);
  assert.equal(schema.properties.contentHash.pattern, "^[0-9a-f]{64}$");
  assert(schema.properties.tenantId.format === "uuid");
});

test("code-first paths are local and application startup cannot own migrations", () => {
  assert.deepEqual(config.externalSqlAuthorities, []);
  assert.equal(config.databaseFirst.migrationsAreAppliedAtStartup, false);
  for (const path of Object.values(config.codeFirst).filter(
    (value) => typeof value === "string",
  )) {
    assert(!path.startsWith("/"), `absolute contract path: ${path}`);
    assert(!path.split("/").includes(".."), `escaping contract path: ${path}`);
  }
  assert.match(typeSpec, /const storageDimensions = 4100/);
  assert.match(typeSpec, /const maximumSourceDimensions = 4096/);
  assert.match(typeSpec, /enum EmbeddingProvider/);
  assert.match(typeSpec, /enum GenerationProvider/);
  assert.match(typeSpec, /Anthropic: "anthropic"/);
});

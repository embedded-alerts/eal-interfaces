#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (file) => readFile(new URL(`../${file}`, import.meta.url), "utf8");
const config = JSON.parse(await read("embedding-contract/generation.json"));
const schema = JSON.parse(await read(config.codeFirst.jsonSchema));
const tsp = await read(config.codeFirst.typeSpec);

assert.equal(config.dimensions.storage, 4100);
assert.equal(config.dimensions.maximumSource, 4096);
assert.equal(config.dimensions.padding, "trailing-zero");
assert.deepEqual(config.externalSqlAuthorities, []);
assert.equal(schema.properties.storageDimensions.const, 4100);
assert.equal(schema.properties.values.minItems, 4100);
assert.equal(schema.properties.values.maxItems, 4100);
assert(schema.required.includes("embeddingSpace"));
assert.equal(schema.$defs.EmbeddingInput.properties.values.maxItems, 4096);
assert(!schema.properties.embeddingProvider.enum.includes("anthropic"));
assert(schema.properties.generationProvider.enum.includes("anthropic"));
assert.match(tsp, /const storageDimensions = 4100/);
assert.match(tsp, /const maximumSourceDimensions = 4096/);
assert.match(tsp, /Anthropic: "anthropic"/);

if (config.databaseFirst.postgres) {
  const postgres = await read(config.databaseFirst.postgres);
  const cockroach = await read(config.databaseFirst.cockroachdb);
  const postgresQuery = await read(config.databaseFirst.postgresQuery);
  const cockroachQuery = await read(config.databaseFirst.cockroachdbQuery);
  for (const sql of [postgres, cockroach]) {
    assert.match(sql, /VECTOR\(4100\)/i);
    assert.match(sql, /TSVECTOR/i);
    assert.match(sql, /ROW LEVEL SECURITY/i);
    assert.match(sql, /notification_dedupe_key/i);
    assert.match(sql, /original_dimensions[^;]+4096/is);
    assert.match(sql, /embedding_space/i);
  }
  assert.match(postgres, /binary_quantize\(embedding\)::bit\(4100\)/);
  assert.match(postgres, /USING hnsw/i);
  assert.match(cockroach, /CREATE VECTOR INDEX/i);
  assert.match(cockroach, /tenant_id, embedding_space, purpose, embedding/);
  assert.match(postgresQuery, /exact_rerank/i);
  assert.match(postgresQuery, /embedding_space = \$6::TEXT/i);
  assert.match(cockroachQuery, /tenant_id = \$1::UUID/i);
  assert.match(cockroachQuery, /embedding_space = \$6::STRING/i);
}

if (config.codeFirst.rustModel) {
  const rust = await read(config.codeFirst.rustModel);
  assert.match(rust, /EMBEDDING_STORAGE_DIMENSIONS: usize = 4100/);
  assert.match(rust, /MAXIMUM_SOURCE_DIMENSIONS: usize = 4096/);
}

process.stdout.write("embedding contract verified: 4100-slot storage, 4096-source cap, hybrid search, and provider provenance\n");

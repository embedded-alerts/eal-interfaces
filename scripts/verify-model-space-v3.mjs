import fs from 'node:fs';

const schema = JSON.parse(
  fs.readFileSync('embedding-contract/model-space-v3.schema.json', 'utf8'),
);
const typeSpec = fs.readFileSync(
  'embedding-contract/model-space-v3.tsp',
  'utf8',
);

if (schema.properties.product.const !== 'embedded-alerts') {
  throw new Error('product identity drift');
}
if (schema.properties.contractVersion.const !== '3.1.0') {
  throw new Error('contract-version drift');
}
if (!schema.required.includes('$schema')) {
  throw new Error('registry meta-schema URI must be represented');
}
for (const token of [
  'Voyage: "voyage"',
  'Anthropic: "anthropic"',
  'storageDimensions: 4100',
  'sourceDimensions: int32',
  'queryProfile: EmbeddingProfileV3',
  'documentProfile: EmbeddingProfileV3',
  'queryEmbedding: float32[]',
  'sourcePolicySha256: string',
]) {
  if (!typeSpec.includes(token)) {
    throw new Error(`TypeSpec missing ${token}`);
  }
}
for (const purpose of ['webpage_search', 'change_correlation', 'alert']) {
  if (!typeSpec.includes(`"${purpose}"`)) {
    throw new Error(`TypeSpec missing purpose ${purpose}`);
  }
}
const providers = schema.properties.providers.properties;
if (providers.anthropic.const !== 'generation-provenance-only') {
  throw new Error('Anthropic provenance drift');
}
const database = schema.properties.database.properties;
if (database.globalFilteredAnnIndex.const !== false) {
  throw new Error('global filtered ANN index must remain disabled');
}
if (database.fusion.const !== 'reciprocal-rank-fusion') {
  throw new Error('hybrid fusion drift');
}
if (database.tenantCompositeForeignKeys.const !== true) {
  throw new Error('tenant-composite foreign-key policy drift');
}
const activation = schema.properties.activation.properties;
if (
  activation.profilesEnabledByDefault.const !== false ||
  activation.spacesEnabledByDefault.const !== false
) {
  throw new Error('v3 activation must default to disabled');
}

console.log('Embedded Alerts interface model-space v3 verified');

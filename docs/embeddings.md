# Embedding contracts

`typespec/embeddings.tsp` and `schemas/embeddings.schema.json` in this repo are
**generated**, and generated somewhere else: they are copies of the artefacts
produced in [`eal-lib-core`](https://github.com/embedded-alerts/eal-lib-core) by
`scripts/embeddings/generate.mjs`, from `db/embedding-contract.json` plus that
repo's `db/org-manifest.json`.

Edit neither file here. Change the contract or the manifest in `eal-lib-core`,
regenerate, and re-copy - the CI gate in that repo fails on drift, and these
copies are what keep the interface layer honest about what the storage layer
will actually accept.

The substance, in one paragraph: an embedding is stored as `vector(4100)`,
left-aligned to the model's native width, zero on every index past it, and
L2-normalized to unit length. 4100 is a superset width that clears the
4096-dimension open-weight models with headroom; the zero pad is norm-preserving,
so a 1536-dimension OpenAI vector ranks exactly as it would at native width. Two
vectors are only ever compared when their `modelKey` matches - padding equalizes
width, not meaning. See `eal-lib-core/docs/embeddings.md` for the retrieval design
and why there is no ANN index on the embedding column itself.

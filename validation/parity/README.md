# Validation authority parity and scope layout

TypeSpec and JSON Schema Draft 2020-12 are independent, peer, human-authored authorities. Neither is generated from the other. `ORESoftware/typespec-json-schema-validator` (`tjsv`) is the fail-closed admission gate for their semantic convergence. `ORESoftware/api-docs` remains pinned downstream tooling for generated-language candidate checks and route/HTTP binding verification; it is not a competing authority or substitute for TJSV admission.

## Required flow

1. Edit the TypeSpec and JSON Schema authorities independently.
2. Run TJSV against the exact Git head. Any structural, normalized-semantic, or differential-validation discrepancy is a stop-and-evaluate condition.
3. Preserve both authored authorities byte-for-byte during comparison; generated comparison evidence belongs only under the dedicated temporary/output directory.
4. After TJSV admission, generate candidate signatures and TypeScript, Rust, Go, and Gleam definitions from each authority and run the pinned downstream parity/codegen checks.
5. Only after agreement, write `generated/final/**` and `parity-receipt.v2.json` to Git.
6. Require producer CI plus independent consumer certification from a `*-test` organization before release.

## Scope folders

Every model belongs to exactly one scope. `isomorphic` is safe everywhere; `client` is client-only; `edge` is edge-only; `server` is private/server-only. New non-isomorphic sources belong under `validation/authorities/<scope>/` with separate independently maintained `.json` and `.tsp` files. Generated candidates and finals preserve the same scope. Browser and edge TypeScript entrypoints cannot export server scope. Node.js, Deno, and Bun entrypoints remain distinct even when they currently re-export identical isomorphic types.

Runtime validators live in the companion `*-lib-core`: Zod (TypeScript), Garde (Rust), `go-playground/validator/v10` (Go), and Gleam decoders. Public `*-clients` import those public validation SDK entrypoints; clients must not copy schemas or import server validators.

Route/HTTP signatures use stable `operationId` values from `ORESoftware/api-docs`. Their binding document is digest-bound into the downstream parity receipt.
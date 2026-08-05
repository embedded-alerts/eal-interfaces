# Architecture

Canonical Embedded Alerts OpenAPI, AsyncAPI, JSON Schema, embedding, rule, and event contracts.

## Fleet

- `eal-interfaces`
- `eal-api`
- `eal-mash-web`
- `eal-leptos-web`
- `eal-dioxus-web`
- `eal-sync`
- `eal-cli`
- `eal-infra`
- `embedded-alerts-clients`
- `embedded-alerts-libs`
- `embedded-alerts.github.io`
- `embedded-alerts-monorepo`

Interfaces own wire formats; libraries own reusable domain behavior; clients consume versioned contracts; runtimes own deployment behavior; monorepos coordinate pinned revisions. Edge code is allowlisted and never a generic proxy.

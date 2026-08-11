#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED_PACKAGE = "embedded-alerts/eal-interfaces"
EXPECTED_REPOSITORY = "https://github.com/embedded-alerts/eal-interfaces"

REQUIRED_CONTRACT_PATHS = [
    "openapi.yaml",
    "asyncapi.yaml",
    "schemas/alert_rule.json",
    "schemas/source.json",
    "schemas/source_fetch_policy.json",
    "schemas/source_revision.json",
    "schemas/embedding_vector.json",
    "schemas/match_candidate.json",
    "schemas/delivery_attempt.json",
    "schemas/event.schema.json",
    "sql/001_initial.sql",
    "sql/002_semantic_alerts_v2.sql",
    "sql/003_domain_scoped_crawl_policy.sql",
    "docs/indexing.md",
]


def main() -> int:
    metadata = json.loads((ROOT / "project.json").read_text(encoding="utf-8"))
    required = [
        "README.md",
        "AGENTS.md",
        "project.json",
        "docs/architecture.md",
        ".zpkg.toml",
        *metadata.get("required_paths", []),
        *REQUIRED_CONTRACT_PATHS,
    ]
    missing = sorted({path for path in required if not (ROOT / path).exists()})
    if missing:
        raise SystemExit(f"missing required paths: {missing}")

    for path in ROOT.rglob("*"):
        if not path.is_file() or ".git" in path.parts or path.stat().st_size > 1_000_000:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        if any(marker in text for marker in ("<" * 7, "=" * 7, ">" * 7)):
            raise SystemExit(f"conflict marker in {path}")
        if re.search(
            r"gh[pousr]_[A-Za-z0-9]{20,}|lin_api_[A-Za-z0-9]{20,}|BEGIN [A-Z ]*PRIVATE KEY",
            text,
        ):
            raise SystemExit(f"credential-shaped content in {path}")

    for path in (ROOT / "openapi/openapi.json", ROOT / "asyncapi/asyncapi.json"):
        if path.exists():
            raise SystemExit(
                f"stale generated contract mirror is forbidden: {path.relative_to(ROOT)}; "
                "generate JSON for a release artifact instead"
            )

    for path in sorted(ROOT.rglob("*.json")):
        try:
            json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as error:
            raise SystemExit(f"invalid JSON in {path}: {error}") from error

    openapi = (ROOT / "openapi.yaml").read_text(encoding="utf-8")
    for marker in (
        "openapi: 3.1.0",
        "bearerAuth:",
        "/v1/sources:",
        "/v1/alert-rules:",
        "/v1/embeddings/search:",
        "/v1/matches:",
        "/v1/delivery-targets:",
        "source_fetch_policy.json",
    ):
        if marker not in openapi:
            raise SystemExit(f"OpenAPI contract is missing {marker!r}")

    asyncapi = (ROOT / "asyncapi.yaml").read_text(encoding="utf-8")
    for marker in (
        "asyncapi: 3.0.0",
        "address: /v1/ws",
        "source.ingested",
        "match.created",
        "delivery.retry_scheduled",
    ):
        if marker not in asyncapi:
            raise SystemExit(f"AsyncAPI contract is missing {marker!r}")

    migration = (ROOT / "sql/002_semantic_alerts_v2.sql").read_text(encoding="utf-8")
    for marker in (
        "create table if not exists eal_sources",
        "create table if not exists eal_source_revisions",
        "create table if not exists eal_embedding_spaces",
        "create table if not exists eal_matches",
        "create table if not exists eal_delivery_attempts",
        "force row level security",
        "eal_match_identity",
    ):
        if marker not in migration:
            raise SystemExit(f"semantic migration is missing {marker!r}")

    crawl_migration = (
        ROOT / "sql/003_domain_scoped_crawl_policy.sql"
    ).read_text(encoding="utf-8")
    for marker in (
        "allowed_hosts",
        "allowed_path_prefixes",
        "obey_robots",
        "create table if not exists eal_crawl_queue",
        "force row level security",
        "eal_tenant_isolation",
    ):
        if marker not in crawl_migration:
            raise SystemExit(f"crawl-policy migration is missing {marker!r}")

    manifest = tomllib.loads((ROOT / ".zpkg.toml").read_text(encoding="utf-8"))
    package = manifest.get("package", {})
    coordinate = f"{package.get('org')}/{package.get('name')}"
    if coordinate != EXPECTED_PACKAGE:
        raise SystemExit(f"unexpected Zed package identity: {coordinate}")
    if package.get("version") != "0.1.0":
        raise SystemExit("Zed package version must remain 0.1.0")
    if package.get("language") != "universal":
        raise SystemExit("Zed package language must use the supported universal variant")
    repository = package.get("repository", {})
    if repository.get("vcs") != "git" or repository.get("url") != EXPECTED_REPOSITORY:
        raise SystemExit("Zed package repository identity is not canonical")
    publish = manifest.get("publish", {})
    if publish.get("tag_format") != "v{version}":
        raise SystemExit("Zed package tag format must remain v{version}")
    dependencies = manifest.get("dependencies", {})
    if dependencies not in ({}, None):
        raise SystemExit("interface package must remain a dependency root")
    target = manifest.get("targets", {}).get("repository")
    if target is not None and target.get("dir") != ".":
        raise SystemExit("repository target must publish the repository root")

    print(
        f"validated {metadata['organization']}/{metadata['repository']}, "
        f"{EXPECTED_PACKAGE}, and semantic contract v2"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

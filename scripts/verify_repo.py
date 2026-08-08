#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPECTED_PACKAGE = "embedded-alerts/eal-interfaces"
EXPECTED_REPOSITORY = "https://github.com/embedded-alerts/eal-interfaces"


def main() -> int:
    metadata = json.loads((ROOT / "project.json").read_text(encoding="utf-8"))
    required = [
        "README.md",
        "AGENTS.md",
        "project.json",
        "docs/architecture.md",
        ".zpkg.toml",
        *metadata.get("required_paths", []),
    ]
    missing = [path for path in required if not (ROOT / path).exists()]
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

    print(f"validated {metadata['organization']}/{metadata['repository']} and {EXPECTED_PACKAGE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""
Pre-commit hook: sync version badge in README.md / README.en.md
with the version from Cargo.toml.

Runs whenever Cargo.toml or the README files change.
"""
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

CARGO_TOML = REPO / "Cargo.toml"
README_FILES = [
    REPO / "README.md",
    REPO / "docs/gh/README.en.md",
]

# Matches: [![Version](https://img.shields.io/badge/Version-v<semver>-blue.svg)]()
BADGE_RE = re.compile(
    r'(\[!\[Version\]\(https://img\.shields\.io/badge/Version-)'
    r'v[^)]+'
    r'(-blue\.svg\]\(\))'
)


def get_version() -> str | None:
    m = re.search(r'^version\s*=\s*"([^"]+)"', CARGO_TOML.read_text(encoding="utf-8"), re.MULTILINE)
    return m.group(1) if m else None


def sync_badge(path: Path, version: str) -> bool:
    text = path.read_text(encoding="utf-8")
    new_text, n = BADGE_RE.subn(rf"\1v{version}\2", text)
    if n == 0:
        return False  # no badge found — nothing to do
    if new_text == text:
        return False  # already up to date
    path.write_text(new_text, encoding="utf-8")
    print(f"  synced version badge → v{version}  ({path.relative_to(REPO)})")
    return True


def main() -> int:
    version = get_version()
    if not version:
        print("error: could not parse version from Cargo.toml")
        return 1

    changed = False
    for p in README_FILES:
        if p.exists():
            changed |= sync_badge(p, version)

    if changed:
        print("  run `git add` to stage the updated badge(s)")
    return 0


if __name__ == "__main__":
    exit(main())

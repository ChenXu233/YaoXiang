#!/usr/bin/env python3
"""AOT 七条最小等价账本门禁。

账本是机器可读的 `tests/aot-ledger.json`。本脚本先解析并校验恰好七条，
再用 `cargo test --all-features --test integration native_aot -- --list`
精确核对测试名，最后逐条用 `--exact` 执行并断言 1 passed / 0 failed。
缺条目、多条目、重复、未知测试名或没有执行到测试都必须 exit 非 0。
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LEDGER = ROOT / "tests" / "aot-ledger.json"
TARGET = "native_aot"
EXPECTED_ENTRIES = 7
STATES = {"supported", "expected-reject"}
LIST_CMD = [
    "cargo",
    "test",
    "--all-features",
    "--test",
    "integration",
    "native_aot",
    "--",
    "--list",
]


def fail(message: str) -> "NoReturn":
    print(f"[aot-ledger] FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def run_cargo(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            cmd,
            cwd=ROOT,
            text=True,
            capture_output=True,
            timeout=900,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"无法执行 {' '.join(cmd)}: {error}")


def load_entries() -> list[dict[str, str]]:
    try:
        raw = json.loads(LEDGER.read_text(encoding="utf-8"))
    except FileNotFoundError:
        fail(f"账本不存在: {LEDGER.relative_to(ROOT)}")
    except json.JSONDecodeError as error:
        fail(f"账本不是合法 JSON: {error}")

    if not isinstance(raw, dict):
        fail("账本顶层必须是对象")
    if raw.get("schema") != 1:
        fail("账本 schema 必须为 1")
    if raw.get("target") != TARGET:
        fail(f"账本 target 必须为 {TARGET}")
    entries = raw.get("entries")
    if not isinstance(entries, list):
        fail("账本 entries 必须是数组")
    if len(entries) != EXPECTED_ENTRIES:
        fail(
            f"账本必须恰好 {EXPECTED_ENTRIES} 条，实际 {len(entries)} 条"
            "（缺条目或多条目都必须红）"
        )

    seen_ids: set[str] = set()
    seen_tests: set[str] = set()
    validated: list[dict[str, str]] = []
    for index, entry in enumerate(entries):
        where = f"entries[{index}]"
        if not isinstance(entry, dict):
            fail(f"{where} 必须是对象")
        for field in ("id", "state", "decision", "test"):
            value = entry.get(field)
            if not isinstance(value, str) or not value.strip():
                fail(f"{where}.{field} 必须是非空字符串")
        if entry["state"] not in STATES:
            fail(f"{where}.state 必须是 {sorted(STATES)} 之一")
        if entry["id"] in seen_ids:
            fail(f"重复 id: {entry['id']}")
        if entry["test"] in seen_tests:
            fail(f"重复 test: {entry['test']}")
        seen_ids.add(entry["id"])
        seen_tests.add(entry["test"])
        validated.append(
            {
                "id": entry["id"],
                "state": entry["state"],
                "decision": entry["decision"],
                "test": entry["test"],
            }
        )
    return validated


def discover_tests() -> set[str]:
    result = run_cargo(LIST_CMD)
    if result.returncode != 0:
        print(result.stdout, end="", file=sys.stderr)
        print(result.stderr, end="", file=sys.stderr)
        fail("cargo test --list 失败，无法核对账本测试名")
    names = set()
    for line in result.stdout.splitlines():
        name, separator, kind = line.partition(": ")
        if separator and kind == "test":
            names.add(name)
    if not names:
        fail("cargo test --list 没有列出任何测试")
    return names


def run_entry(entry: dict[str, str], available: set[str]) -> None:
    test = entry["test"]
    if test not in available:
        fail(f"账本条目的测试不存在: {test}")
    cmd = [
        "cargo",
        "test",
        "--all-features",
        "--test",
        "integration",
        "--",
        "--exact",
        test,
    ]
    result = run_cargo(cmd)
    output = result.stdout + result.stderr
    passed = "1 passed; 0 failed" in output
    if result.returncode != 0 or not passed:
        print(f"[aot-ledger] FAIL {entry['id']} ({entry['state']}): {test}")
        print(output[-4000:], file=sys.stderr)
        fail("测试没有以 1 passed / 0 failed 完成（0 执行也必须红）")
    print(f"[aot-ledger] OK {entry['id']} ({entry['state']}): {test}")


def main() -> None:
    entries = load_entries()
    available = discover_tests()
    print(f"[aot-ledger] 已发现 {len(available)} 条 {TARGET} 测试；账本 {len(entries)} 条")
    for entry in entries:
        run_entry(entry, available)
    print(f"[aot-ledger] PASS: {len(entries)}/{len(entries)} 条等价边界可执行")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""check-test-channel: 测试通道口径同源门禁（#486 第 2 项）

四处在声明「提交前跑哪些测试」：`AGENTS.md` 常用命令区、
`docs/src/dev/test-specification.md` 规则 18.1、`docs/src/dev/HOWTO.md`
提交前验证（改代码）、`.pre-commit-config.yaml` 的 cargo-test 钩子。它们曾各写一份
且互不一致——文档写 `cargo test`、HOWTO 没有测试命令、pre-commit 跑
`cargo test --all`，而 `testsuite.py run fast` 当时只跑 `cargo test --lib`。
后果是本地自检全绿、提交时才红（`n: N` 强制后两条集成用例仍写 `f(42)(5)`）。

本门禁把四处按同一目标集机械对齐（任一不满足即 exit 1）：

  1. `scripts/dev/testsuite.py` 的 `EXPLICIT_TARGETS` 与 `_base_command()` 拼出的
     cargo 选择器必须一致，且等于本脚本钉住的 `CANONICAL_TARGETS`
     （扩/缩目标集 = 覆盖面变更，必须同时改两处并在提交说明里写理由）；
     命令里不得出现 `--all-targets`（会把 criterion bench 当测试执行）。
  2. 三处文档小节都必须逐字给出规范命令 `python scripts/dev/testsuite.py run fast`，
     且小节里出现的每个 `testsuite.py run <channel>` 都是已知通道。
  3. `.pre-commit-config.yaml`：`cargo-test-fast` 的 entry 是规范命令、
     `cargo-test` 的 entry 是拦提交口径 `cargo test --all --quiet`，
     且 fast 段排在 cargo-test 段之前（配置开 `fail_fast: true`，顺序即优先级）。

用法:
    python scripts/ci/check-test-channel.py [--root <仓库根>]

退出码: 0 = 四处同源; 1 = 口径不一致; 3 = 环境错误（文件缺失 / testsuite 导入失败）。
"""
from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# 门禁钉住的提交前目标集。改这里必须同时改 scripts/dev/testsuite.py 的
# EXPLICIT_TARGETS 与三处文档口径——这是覆盖面变更，不是措辞调整。
CANONICAL_TARGETS: tuple[str, ...] = (
    "lib",
    "integration",
    "formatter_integration",
    "yx_runner",
    "yx_multifile_runner",
)

CANONICAL_COMMAND = "python scripts/dev/testsuite.py run fast"
FULL_GATE_COMMAND = "cargo test --all --quiet"

# (文件, 小节起始正则, 小节结束正则, 人读名字)
DOC_SITES: tuple[tuple[str, str, str, str], ...] = (
    ("AGENTS.md", r"^## 常用命令\s*$", r"^## ", "AGENTS.md 常用命令区"),
    (
        "docs/src/dev/test-specification.md",
        r"^\*\*规则 18\.1\*\*",
        r"^(## |### |\*\*规则 18\.2\*\*)",
        "test-specification 规则 18.1",
    ),
    ("docs/src/dev/HOWTO.md", r"^### 提交前验证（改代码）\s*$", r"^(## |### )", "HOWTO 提交前验证（改代码）"),
)

CHANNEL_RE = re.compile(r"scripts/dev/testsuite\.py\s+run\s+([a-z]+)")
KNOWN_CHANNELS = ("fast", "full")


class CheckError(Exception):
    """环境错误（退出码 3）：前提文件缺失、testsuite 导入失败。"""


class SectionMissing(Exception):
    """文档小节缺失：口径不一致（退出码 1），不是环境错误。"""


def read_text(path: Path) -> str:
    if not path.is_file():
        raise CheckError(f"读不到 {path}（门禁比对的前提文件缺失）")
    return path.read_text(encoding="utf-8")


def extract_section(text: str, start_re: str, end_re: str, name: str) -> tuple[int, str]:
    """抽出一个文档小节（含标题行，到下一个同级/更高级标题前为止）。"""
    lines = text.splitlines()
    start_re_c = re.compile(start_re)
    end_re_c = re.compile(end_re)
    for idx, line in enumerate(lines):
        if not start_re_c.search(line):
            continue
        body = [line]
        for follow in lines[idx + 1 :]:
            if end_re_c.search(follow):
                break
            body.append(follow)
        return idx + 1, "\n".join(body)
    raise SectionMissing(f"找不到小节「{name}」：标题正则 {start_re!r} 在文件里没有命中")


def load_testsuite(root: Path):
    path = root / "scripts" / "dev" / "testsuite.py"
    if not path.is_file():
        raise CheckError(f"读不到 {path}")
    spec = importlib.util.spec_from_file_location("yx_testsuite", path)
    if spec is None or spec.loader is None:
        raise CheckError(f"无法导入 {path}")
    module = importlib.util.module_from_spec(spec)
    # 必须先登记进 sys.modules：testsuite.py 用了 dataclasses，dataclasses 解析
    # 注解时要按 cls.__module__ 反查模块（未登记 → 'NoneType' object has no
    # attribute '__dict__'）。
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except Exception as exc:  # noqa: BLE001 —— 导入期任何异常都是环境错误
        raise CheckError(f"导入 {path} 失败：{exc}")
    return module


def force_utf8_stdio() -> None:
    """统一 UTF-8 输出：Windows 控制台默认 cp936，中文报告会乱码或崩。"""
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass


def parse_hooks(text: str) -> dict[str, dict]:
    """极简 pre-commit 钩子解析：按 `- id: <name>` 切块，取出 entry 与行号。"""
    hooks: dict[str, dict] = {}
    current = None
    for idx, line in enumerate(text.splitlines(), 1):
        m = re.match(r"^\s*-\s+id:\s*(\S+)\s*$", line)
        if m:
            current = m.group(1)
            hooks[current] = {"line": idx, "entry": None}
            continue
        if current is None:
            continue
        m = re.match(r"^\s+entry:\s*(.+?)\s*$", line)
        if m:
            hooks[current]["entry"] = m.group(1).strip().strip("'\"")
    return hooks


def channel_targets(testsuite, channel: str) -> tuple[str, ...] | None:
    """通道实际覆盖的目标集：fast/full 都跑 EXPLICIT_TARGETS（manual 档永不隐式执行）。"""
    if channel in KNOWN_CHANNELS:
        return tuple(testsuite.EXPLICIT_TARGETS)
    return None

def main(argv=None) -> int:
    force_utf8_stdio()
    parser = argparse.ArgumentParser(description="测试通道口径同源门禁")
    parser.add_argument("--root", default=None, help="仓库根（默认脚本所在仓库）")
    args = parser.parse_args(argv)
    root = Path(args.root).resolve() if args.root else REPO_ROOT

    problems: list[str] = []

    # ---- 1. testsuite.py：目标集与命令构造
    try:
        testsuite = load_testsuite(root)
    except CheckError as exc:
        print(f"[error] {exc}", file=sys.stderr)
        return 3
    actual_targets = tuple(getattr(testsuite, "EXPLICIT_TARGETS", ()))
    if actual_targets != CANONICAL_TARGETS:
        problems.append(
            "testsuite.py 的 EXPLICIT_TARGETS 与门禁钉住的目标集不一致："
            f"实际 {actual_targets}，门禁 {CANONICAL_TARGETS}"
            "（扩/缩覆盖面必须同时改 testsuite.py、本脚本与三处文档，并在提交说明里写清理由）"
        )
    base_command = list(testsuite._base_command())
    expected_command = ["cargo", "test"] + list(testsuite.target_args()) + ["--"]
    if base_command != expected_command:
        problems.append(
            f"testsuite.py 的 _base_command() 与 target_args() 不一致：{base_command} != {expected_command}"
        )
    if "--all-targets" in base_command:
        problems.append("testsuite.py 的基命令出现 --all-targets（会把 criterion bench 当测试执行）")
    for target in CANONICAL_TARGETS:
        selector = ["--lib"] if target == "lib" else ["--test", target]
        if selector[0] == "--test" and " ".join(selector) not in " ".join(base_command):
            problems.append(f"testsuite.py 的基命令漏了目标 {target}（期望片段 {' '.join(selector)!r}）")
    print(f"[testsuite] 目标集：{' + '.join(actual_targets) or '（空）'}")
    print(f"[testsuite] 基命令：{' '.join(base_command)}")

    # ---- 2. 三处文档口径
    site_target_sets: dict[str, tuple[str, ...]] = {}
    for rel_path, start_re, end_re, name in DOC_SITES:
        try:
            text = read_text(root / rel_path)
        except CheckError as exc:
            print(f"[error] {exc}", file=sys.stderr)
            return 3
        try:
            line_no, section = extract_section(text, start_re, end_re, name)
        except SectionMissing as exc:
            problems.append(f"{rel_path}: {exc}")
            continue
        if CANONICAL_COMMAND not in section:
            problems.append(f"{rel_path}:{line_no} {name} 未给出规范命令 {CANONICAL_COMMAND!r}")
        channels = CHANNEL_RE.findall(section)
        unknown = [c for c in channels if c not in KNOWN_CHANNELS]
        if unknown:
            problems.append(
                f"{rel_path}:{line_no} {name} 引用了未知通道 {sorted(set(unknown))}（已知 {KNOWN_CHANNELS}）"
            )
        if "fast" not in channels:
            problems.append(f"{rel_path}:{line_no} {name} 没有把提交前命令指向 fast 通道")
        site_targets = channel_targets(testsuite, "fast") or ()
        site_target_sets[f"{rel_path} {name}"] = tuple(site_targets)
        print(f"[docs] {rel_path}:{line_no} {name} → fast 通道，目标集 {' + '.join(site_targets)}")

    # ---- 3. pre-commit 接线
    try:
        pre_commit_text = read_text(root / ".pre-commit-config.yaml")
    except CheckError as exc:
        print(f"[error] {exc}", file=sys.stderr)
        return 3
    hooks = parse_hooks(pre_commit_text)
    fast_hook = hooks.get("cargo-test-fast")
    gate_hook = hooks.get("cargo-test")
    if fast_hook is None:
        problems.append(".pre-commit-config.yaml 缺 cargo-test-fast 钩子（提交前快反馈段）")
    else:
        if fast_hook["entry"] != CANONICAL_COMMAND:
            problems.append(
                f".pre-commit-config.yaml: 钩子 cargo-test-fast 的 entry 是 {fast_hook['entry']!r}，"
                f"规范命令是 {CANONICAL_COMMAND!r}"
            )
        print(f"[pre-commit] cargo-test-fast → {fast_hook['entry']}")
    if gate_hook is None:
        problems.append(".pre-commit-config.yaml 缺 cargo-test 钩子（拦提交全量段）")
    else:
        if gate_hook["entry"] != FULL_GATE_COMMAND:
            problems.append(
                f".pre-commit-config.yaml: 钩子 cargo-test 的 entry 是 {gate_hook['entry']!r}，"
                f"拦提交口径是 {FULL_GATE_COMMAND!r}"
            )
        print(f"[pre-commit] cargo-test → {gate_hook['entry']}")
    if fast_hook is not None and gate_hook is not None and fast_hook["line"] > gate_hook["line"]:
        problems.append(
            ".pre-commit-config.yaml: cargo-test-fast 必须排在 cargo-test 之前"
            f"（当前 fast 在第 {fast_hook['line']} 行、全量在第 {gate_hook['line']} 行；"
            "配置开 fail_fast: true，顺序即优先级）"
        )

    # ---- 4. 三处文档是否指向同一目标集
    distinct = {tuple(sorted(value)) for value in site_target_sets.values()}
    if len(distinct) > 1:
        problems.append(f"三处文档指向的目标集不一致：{site_target_sets}")

    if problems:
        for problem in problems:
            print(f"[FAIL] {problem}")
        print(f"[check-test-channel] 不一致 {len(problems)} 条")
        return 1
    print(
        f"[check-test-channel] OK：testsuite / 三处文档 / pre-commit 同源，"
        f"提交前目标集 = {' + '.join(CANONICAL_TARGETS)}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

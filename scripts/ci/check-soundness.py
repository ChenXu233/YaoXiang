#!/usr/bin/env python3
"""check-soundness: 健全性探针——「check 通过 ⇒ run 不得出现 E0–E5 同族错误」（#486 / WBS 0.6.3）

判据（宽口径 E0–E5）
--------------------
判据面 = **先项目形态、后单文件语料**：
  * 项目形态：`tests/project-corpus/<case>/`（含 `yaoxiang.toml`，入口 `main.yx`）——
    三类已知分叉（E3022 / E3020 / E3005）的入库判据面；这些目录**不进**语料差分基线；
  * 单文件/既有语料：`tests/yaoxiang`、`src/std/tests`、`tests/yaoxiang-multifile`。
对每条语料在**其所在目录**为 cwd 跑 `check`：
  * `check` 退出码非 0 ⇒ 该语料不进判据面（负控面：编译期已经报错，run 的行为不在
    本探针判定范围），只计入「check 未通过」；
  * `check` 退出码 0 ⇒ 再跑 `run`，从输出里抓 `error [EXXXX]`；只要出现 E0–E5 族
    （词法/语法 E0、类型 E1、语义 E2、代码生成 E3、泛型 E4、模块 E5）即记一处分叉。

族 → 类别权威源
----------------
`src/util/diagnostic/codes/mod.rs` 的 `ErrorCategory` 枚举（E0=Lexer/Parser、E1=TypeCheck、
E2=Semantic、E3=Codegen、E4=Generic、E5=Module、E6=Runtime、E7=Io、E8=Internal、W1=Warning）。
本脚本启动时解析该文件拿到映射，解析不到就 exit 3——不把族映射硬编码成第二份事实。

三个已实测的坑（判据正确性依赖它们）
------------------------------------
1. **必须先剥 ANSI 再抓码**：诊断带颜色转义（`\x1b[31merror\x1b[0m [\x1b[1mE6007\x1b[0m]`），
   不剥一个码都读不到，判据会「假绿」。
2. **不按退出码判**：`run` 退出 1 可能零诊断（「main 未被调用」= exit 1 + 两行提示），
   按退出码判会把这种正常形态记成分叉；反过来 run 退出 0 也可能带诊断。本脚本只看码。
3. **诊断可能挂在别的文件上**：`check` 在项目内按 project root 归组，错误位置不一定是
   被指定的那个文件。故整段 stdout+stderr 全扫，不按文件名过滤。

当前状态：report-only（本轮不阻塞）
----------------------------------
三种形态：
  * 默认：分叉只打印、exit 0；环境问题（二进制缺失 / 权威源读不到）仍 exit 3；
  * `--report-only`：连环境问题也不阻塞（exit 恒 0）——pre-commit 钩子用这一形态；
  * `--strict`：翻闸后的形态，分叉 > 0 即 exit 1。
pre-commit 里挂的是 `--report-only`，只在暂存区含语料 `.yx` 时触发。

翻转成阻塞的判据（翻闸时把 pre-commit entry 的 `--report-only` 换成 `--strict`）
-----------------------------------------------------
1. 三类已知分叉在语料上归零：**E3022**（bin 项目 main 带参，check=0 / run=E3022）、
   **E3020**（有 manifest 未声明 `[run] main` 且无 main 绑定，check=0 / run=E3020）、
   **E3005**（check 不读 `yaoxiang.toml` 的 `[mono]`：`max_depth = 1` 时 run 按上限 1
   报错，check 仍按默认 100 放过，同一程序 check=0 / run=E3005）。
   **进度（2026-11-06）**：三类全修——E3005（`[mono]` 贯通到 check 链）、E3020 /
   E3022（入口契约判定补进 check），语料上的分叉已归零（实测 0 处）；三套夹具转为
   「check 与 run 同口径」的回归守卫（任一处判据被退回 ⇒ 探针重新报出分叉）。
   这三类的入库判据面是 `tests/project-corpus/` 下的三套项目夹具，探针默认就纳入；
   **单文件语料（今天的 375 条）上这三类一个都没有**（实测 0），故项目形态必须单独
   成层——不得把它们塞进 `tests/yaoxiang`（会被 corpus_probe 收进差分基线，把违规
   录成期望行为）；
2. 变异自证：去掉 `src/driver/program.rs` 的 `CHECK_STAGES` 里的 `Stage::Typecheck`
   并重编 `cargo build --bin yaoxiang-rs`，本探针必须报出分叉；还原后归零、补丁前后
   blob 一致；
3. 耗时进得了门禁预算（实测见提交说明；`--jobs` 控制并发）。

用法
----
    python scripts/ci/check-soundness.py                 # report-only（默认），exit 0
    python scripts/ci/check-soundness.py --strict        # 翻闸后的形态：有分叉即 exit 1
    python scripts/ci/check-soundness.py --json          # 机器可读摘要
    python scripts/ci/check-soundness.py --bin <path>    # 指定 CLI 二进制（变异自证用）
    python scripts/ci/check-soundness.py --root <path>   # 指定仓库根
    python scripts/ci/check-soundness.py --jobs 8        # 并发数（默认 8，1 = 顺序）
    python scripts/ci/check-soundness.py --report-only   # pre-commit 形态：任何情况都不阻塞
    python scripts/ci/check-soundness.py --corpus <dir>  # 额外语料根：喂场景做复现/自证

退出码：0 = 默认 / --report-only（不论分叉数）；1 = --strict 且分叉 > 0；
3 = 环境错误（默认与 --strict 下；--report-only 恒 0）。
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

BIN_NAME = "yaoxiang-rs"

# 项目形态判据面（先跑）：tests/project-corpus/<case>/ 下含 yaoxiang.toml 的项目，
# 入口约定 main.yx（与 tests/yaoxiang-multifile 同一套约定）。这三套复现的是
# 「check 放过、run 才报」的分叉（E3022 / E3020 / E3005），**不进语料差分基线**
# （corpus_probe 不扫这个目录），也不被 yx_multifile_runner 收集。
PROJECT_CORPUS_ROOTS: tuple[tuple[str, str], ...] = (
    ("tests/project-corpus", "**/main.yx"),
)

# 单文件/既有语料（后跑）：三处，合计 = 语料差分基线的记录数
CORPUS_ROOTS: tuple[tuple[str, str], ...] = (
    ("tests/yaoxiang", "**/*.yx"),
    ("src/std/tests", "**/*.yx"),
    ("tests/yaoxiang-multifile", "**/main.yx"),
)

# 健全性面 = E0–E5；E6 运行时、E7 I/O、E8 内部、W1 警告不算分叉
SOUNDNESS_FAMILIES: tuple[str, ...] = ("E0", "E1", "E2", "E3", "E4", "E5")

ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
ERROR_CODE_RE = re.compile(r"error\s*\[\s*(E\d{4})\s*\]")
# ErrorCategory 枚举条目：`    Lexer,     // E0xxx: 词法分析`
CATEGORY_RE = re.compile(r"^\s+(\w+),\s*//\s*(W?E\d)xxx:", re.MULTILINE)

DEFAULT_TIMEOUT = 60.0


class ToolError(Exception):
    """环境错误（退出码 3）。"""


def force_utf8_stdio() -> None:
    """统一 UTF-8 输出：Windows 控制台默认 cp936，中文报告会乱码。"""
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass


def strip_ansi(text: str) -> str:
    """剥掉颜色/样式转义——不剥的话 `error [E1002]` 一个都匹配不到。"""
    return ANSI_RE.sub("", text)


def resolve_bin(root: Path, explicit: str | None) -> Path:
    if explicit:
        path = Path(explicit)
        if not path.is_absolute():
            path = root / path
        if not path.is_file():
            raise ToolError(f"指定的二进制不存在：{path}")
        return path
    for name in (BIN_NAME + ".exe", BIN_NAME):
        candidate = root / "target" / "debug" / name
        if candidate.is_file():
            return candidate
    raise ToolError(
        f"找不到 {root / 'target' / 'debug' / BIN_NAME}：先跑 cargo build --bin {BIN_NAME}"
    )


def load_categories(root: Path) -> dict[str, tuple[str, ...]]:
    """解析 ErrorCategory 枚举，得到 族前缀 → 类别名 的权威映射。"""
    path = root / "src" / "util" / "diagnostic" / "codes" / "mod.rs"
    if not path.is_file():
        raise ToolError(f"读不到族→类别权威源：{path}")
    text = path.read_text(encoding="utf-8")
    families: dict[str, list[str]] = {}
    for category, prefix in CATEGORY_RE.findall(text):
        families.setdefault(prefix, []).append(category)
    if not all(family in families for family in SOUNDNESS_FAMILIES):
        raise ToolError(
            f"从 {path} 解析不出完整的 E0–E5 族映射（实际拿到 {sorted(families)}）"
            "——门禁赖以判族的权威源变了，先修解析再跑"
        )
    return {prefix: tuple(names) for prefix, names in sorted(families.items())}


def discover_corpus(root: Path, extra_roots: list[str]) -> list[tuple[str, Path]]:
    """枚举语料：返回 (来源标签, 文件路径) 列表。

    `extra_roots` 是 --corpus 传入的额外语料根（按 **/*.yx 枚举）：用来把
    「项目形态的场景」喂给探针做复现/自证，**不改官方语料**。
    """
    found: list[tuple[str, Path]] = []
    for label, pattern in PROJECT_CORPUS_ROOTS:
        base = root / label
        if not base.is_dir():
            raise ToolError(f"项目形态判据面不存在：{base}（缺了它三类已知分叉就没有入库判据面）")
        paths = sorted(base.glob(pattern))
        if not paths:
            raise ToolError(f"{base} 下没有 {pattern}：判据面会「绿但空」")
        for path in paths:
            if not (path.parent / "yaoxiang.toml").is_file():
                raise ToolError(
                    f"{path} 缺同级 yaoxiang.toml：项目形态夹具必须是含 manifest 的一级子目录"
                )
        found.extend((label, path) for path in paths)
    for label, pattern in CORPUS_ROOTS:
        base = root / label
        if not base.is_dir():
            raise ToolError(f"语料目录不存在：{base}")
        found.extend((label, path) for path in sorted(base.glob(pattern)))
    for raw in extra_roots:
        base = Path(raw)
        if not base.is_absolute():
            base = root / base
        if not base.is_dir():
            raise ToolError(f"--corpus 目录不存在：{base}")
        paths = sorted(base.glob("**/*.yx"))
        if not paths:
            raise ToolError(f"--corpus 目录里没有 .yx：{base}")
        found.extend((raw, path) for path in paths)
    if not found:
        raise ToolError("语料枚举为空：判据会「绿但空」，先检查 CORPUS_ROOTS 的 glob")
    return found


def run_cli(
    binary: Path, subcommand: str, filename: str, cwd: Path, timeout: float
) -> tuple[int, str, bool]:
    """执行一次 CLI。返回 (退出码, stdout+stderr, 是否超时)。"""
    try:
        proc = subprocess.run(
            [str(binary), subcommand, filename],
            cwd=str(cwd),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        partial = ""
        for chunk in (exc.stdout, exc.stderr):
            if isinstance(chunk, bytes):
                partial += chunk.decode("utf-8", "replace")
            elif chunk:
                partial += chunk
        return -1, partial, True
    return proc.returncode, proc.stdout + "\n" + proc.stderr, False

def probe_file(binary: Path, corpus_label: str, path: Path, root: Path, timeout: float) -> dict:
    """单条语料：check → （通过才）run → 抓码 → 按族判。"""
    record = {
        "file": path.relative_to(root).as_posix(),
        "corpus": corpus_label,
        "check_exit": None,
        "check_timeout": False,
        "check_passed": False,
        "run_exit": None,
        "run_timeout": False,
        "codes": [],
        "divergences": [],
    }
    code, _out, timed_out = run_cli(binary, "check", path.name, path.parent, timeout)
    record["check_exit"] = code
    record["check_timeout"] = timed_out
    record["check_passed"] = code == 0 and not timed_out
    if not record["check_passed"]:
        return record
    code, out, timed_out = run_cli(binary, "run", path.name, path.parent, timeout)
    record["run_exit"] = code
    record["run_timeout"] = timed_out
    text = strip_ansi(out)  # 坑 1：不剥 ANSI 一个码都读不到
    codes = sorted(set(ERROR_CODE_RE.findall(text)))
    record["codes"] = codes
    record["divergences"] = [c for c in codes if c[:2] in SOUNDNESS_FAMILIES]
    return record


def summarize(records: list[dict], categories: dict[str, tuple[str, ...]]) -> dict:
    passed = [r for r in records if r["check_passed"]]
    diverged = [r for r in passed if r["divergences"]]
    by_code: dict[str, int] = {}
    by_family: dict[str, int] = {}
    for record in diverged:
        for code in record["divergences"]:
            by_code[code] = by_code.get(code, 0) + 1
            by_family[code[:2]] = by_family.get(code[:2], 0) + 1
    runtime_only = [r for r in passed if r["codes"] and not r["divergences"]]
    silent_failure = [r for r in passed if r["run_exit"] not in (0, None) and not r["codes"]]
    labels: list[str] = []
    for label, _ in PROJECT_CORPUS_ROOTS + CORPUS_ROOTS:
        if label not in labels:
            labels.append(label)
    for record in records:  # --corpus 传入的额外根
        if record["corpus"] not in labels:
            labels.append(record["corpus"])
    return {
        "corpus": len(records),
        "corpus_by_root": {label: sum(1 for r in records if r["corpus"] == label) for label in labels},
        "check_passed": len(passed),
        "check_failed": len(records) - len(passed),
        "check_timeouts": sum(1 for r in records if r["check_timeout"]),
        "run_timeouts": sum(1 for r in passed if r["run_timeout"]),
        "divergence_count": len(diverged),
        "divergences": [
            {
                "file": r["file"],
                "run_exit": r["run_exit"],
                "codes": r["divergences"],
                "families": sorted({c[:2] for c in r["divergences"]}),
            }
            for r in diverged
        ],
        "by_code": dict(sorted(by_code.items())),
        "by_family": dict(sorted(by_family.items())),
        "families": {key: list(names) for key, names in categories.items()},
        "runtime_only": len(runtime_only),
        "silent_failure_no_code": len(silent_failure),
    }


def print_report(
    summary: dict, categories: dict[str, tuple[str, ...]], binary: str, elapsed: float, jobs: int
) -> None:
    roots = " / ".join(f"{label} {count}" for label, count in summary["corpus_by_root"].items())
    print(f"[soundness] 语料 {summary['corpus']} 条：{roots}")
    print(f"[soundness] CLI：{binary}；族→类别权威源：src/util/diagnostic/codes/mod.rs")
    print(
        f"[soundness] 判定面：check 通过 {summary['check_passed']} 条；"
        f"check 未通过 {summary['check_failed']} 条（负控面，不判）"
    )
    print(f"[soundness] 分叉 {summary['divergence_count']} 处（check 通过但 run 报 E0–E5 同族错误）：")
    for item in summary["divergences"]:
        names = "、".join(
            name for family in item["families"] for name in categories.get(family, ())
        )
        print(
            f"  - {item['file']}  run_exit={item['run_exit']}  "
            f"{' '.join(item['codes'])}（{names}）"
        )
    by_code = " · ".join(f"{code} {count}" for code, count in summary["by_code"].items()) or "（无）"
    by_family = (
        " · ".join(
            f"{family} {count}（{'、'.join(categories.get(family, ()))}）"
            for family, count in summary["by_family"].items()
        )
        or "（无）"
    )
    print(f"[soundness] 按码：{by_code}")
    print(f"[soundness] 按族：{by_family}")
    print(
        f"[soundness] 附：run 退出≠0 但零 error 码 {summary['silent_failure_no_code']} 条"
        "（设计形态：如「main 未被调用」= exit 1 + 提示；不判——见脚本头第 2 条）"
    )
    print(
        f"[soundness] 附：只出现 E6–E8 码 {summary['runtime_only']} 条"
        "（运行时 / I/O / 内部，不在健全性面）"
    )
    if summary["check_timeouts"] or summary["run_timeouts"]:
        print(
            f"[soundness] 附：超时 check {summary['check_timeouts']} 次 / run {summary['run_timeouts']} 次"
            "（--timeout 调整）"
        )
    print(f"[soundness] 耗时 {elapsed:.1f}s（{summary['corpus']} 条语料 × 2 次 CLI，jobs={jobs}）")
    if summary["divergence_count"]:
        print(
            "[soundness] report-only：本轮不阻塞（分叉非零也只 exit 0）；"
            "翻闸判据见脚本头「翻转成阻塞的判据」"
        )
    else:
        print("[soundness] OK：check 通过的语料在 run 上未出现 E0–E5 同族错误")


def main(argv=None) -> int:
    force_utf8_stdio()
    parser = argparse.ArgumentParser(
        description="健全性探针：check 通过 ⇒ run 不得出现 E0–E5 同族错误（默认 report-only）"
    )
    parser.add_argument("--root", default=None, help="仓库根（默认脚本所在仓库）")
    parser.add_argument("--bin", default=None, help="CLI 二进制（默认 target/debug/yaoxiang-rs[.exe]）")
    parser.add_argument("--strict", action="store_true", help="分叉 > 0 时 exit 1（翻闸后的形态）")
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="任何情况都 exit 0（pre-commit 钩子形态）：连环境错误也不阻塞",
    )
    parser.add_argument("--json", action="store_true", help="输出机器可读摘要")
    parser.add_argument("--jobs", type=int, default=8, help="并发 CLI 调用数（默认 8；1 = 顺序）")
    parser.add_argument(
        "--corpus",
        action="append",
        default=[],
        metavar="DIR",
        help="额外语料根（可重复，按 **/*.yx 枚举）：把项目形态的场景喂给探针做复现，不改官方语料",
    )
    parser.add_argument("--timeout", type=float, default=DEFAULT_TIMEOUT, help="单次 CLI 超时秒数")
    args = parser.parse_args(argv)

    if args.strict and args.report_only:
        parser.error("--strict 与 --report-only 互斥")
    root = Path(args.root).resolve() if args.root else REPO_ROOT
    try:
        categories = load_categories(root)
        binary = resolve_bin(root, args.bin)
        corpus = discover_corpus(root, args.corpus)
    except ToolError as exc:
        print(f"[error] {exc}", file=sys.stderr)
        return 0 if args.report_only else 3

    jobs = max(1, args.jobs)
    started = time.monotonic()
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        records = list(
            pool.map(
                lambda item: probe_file(binary, item[0], item[1], root, args.timeout),
                corpus,
            )
        )
    elapsed = time.monotonic() - started

    summary = summarize(records, categories)
    summary["elapsed_secs"] = round(elapsed, 2)
    summary["jobs"] = jobs
    if args.json:
        print(json.dumps(summary, ensure_ascii=False, indent=2))
    else:
        print_report(summary, categories, str(binary), elapsed, jobs)

    if args.strict and summary["divergence_count"]:
        return 1
    return 0  # 默认与 --report-only 都不阻塞


if __name__ == "__main__":
    sys.exit(main())

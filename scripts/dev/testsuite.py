#!/usr/bin/env python3
"""测试资源画像标记与预算通道 —— #486 第 1 项 + 第 2 项的最小消费面。

用法（本仓唯一入口）：

    python scripts/dev/testsuite.py markers --check     # 双向守卫：标记 <-> #[test] <-> 独占助手
    python scripts/dev/testsuite.py markers --selftest  # 文法与扫描器的正反例自检
    python scripts/dev/testsuite.py plan [--json]       # 只打印计划（预算 / 每波成员与峰值 / 实际 jobs 与 threads）
    python scripts/dev/testsuite.py run fast            # 默认集按预算装箱；独占测试各占一波；装不下的 defer
    python scripts/dev/testsuite.py run full            # 默认集必须全部跑到；预算不足即报红并列出缺口

标记文法（就地声明，不建中央清单 —— RFC-036 2026-09-06 已批准：中央清单必腐烂）：

    // cost: peak=<整数><B|KiB|MiB|GiB>  time=<整数或小数>s  mode=shared|exclusive|manual  source=...
    /// cost: ...    # 文档注释形式同样接受

  * 位置：紧邻 #[test] 上方的属性/文档注释块内，单行；键序无关，空格分隔。
  * source = measured@YYYY-MM-DD 或 extrapolated@<无空格依据标识>；
    mode=exclusive|manual 缺 source 是硬错误（外推必须自报外推）。
  * 未知键 / 未知单位 / 未知 mode / 未知 source 形态一律硬错误。

退出码：0 通过（含 fast 的 defer）；1 标记违规或测试失败；2 预算不足（full）；3 环境错误。
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

GIB = 1 << 30
MIB = 1 << 20
KIB = 1 << 10

UNITS = {"B": 1, "KiB": KIB, "MiB": MIB, "GiB": GIB}
MODES = ("shared", "exclusive", "manual")

# 预算口径（全部可用环境变量覆盖）
DEFAULT_RESERVE = 2 * GIB  # 机器可用内存里留给 OS / 编辑器的保留量
DEFAULT_TEST_PEAK = 128 * MIB  # 未标记测试的每线程峰值估计（装箱用）
DEFAULT_BUILD_JOB_PEAK = 1024 * MIB  # 每个 rustc 作业的峰值估计


class ToolError(Exception):
    """环境错误（退出码 3）：读不到机器内存、cargo 调用失败等。"""


def human_bytes(n: int) -> str:
    sign = "-" if n < 0 else ""
    n = abs(n)
    for unit, size in (("GiB", GIB), ("MiB", MIB), ("KiB", KIB)):
        if n >= size:
            return f"{sign}{n / size:.2f} {unit}"
    return f"{sign}{n} B"


def parse_size(text: str) -> int:
    m = re.fullmatch(r"\s*([0-9]+(?:\.[0-9]+)?)\s*(B|KiB|MiB|GiB)\s*", text)
    if not m:
        raise ToolError(f"无法解析容量 {text!r}（期望形如 1GiB / 512MiB / 800B）")
    return int(float(m.group(1)) * UNITS[m.group(2)])


# ---------------------------------------------------------------- 机器内存


def _windows_memory() -> tuple[int, int, str]:
    import ctypes
    from ctypes import wintypes

    class MemoryStatusEx(ctypes.Structure):
        _fields_ = [
            ("dwLength", wintypes.DWORD),
            ("dwMemoryLoad", wintypes.DWORD),
            ("ullTotalPhys", ctypes.c_ulonglong),
            ("ullAvailPhys", ctypes.c_ulonglong),
            ("ullTotalPageFile", ctypes.c_ulonglong),
            ("ullAvailPageFile", ctypes.c_ulonglong),
            ("ullTotalVirtual", ctypes.c_ulonglong),
            ("ullAvailVirtual", ctypes.c_ulonglong),
            ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
        ]

    status = MemoryStatusEx()
    status.dwLength = ctypes.sizeof(MemoryStatusEx)
    if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
        raise ToolError("GlobalMemoryStatusEx 调用失败，无法读取可用内存")
    return int(status.ullAvailPhys), int(status.ullTotalPhys), "windows-global-memory-status-ex"


def _linux_memory() -> tuple[int, int, str]:
    values = {}
    with open("/proc/meminfo", encoding="utf-8") as handle:
        for line in handle:
            key, _, rest = line.partition(":")
            values[key.strip()] = int(rest.split()[0]) * 1024
    if "MemAvailable" not in values or "MemTotal" not in values:
        raise ToolError("/proc/meminfo 缺少 MemAvailable/MemTotal")
    return values["MemAvailable"], values["MemTotal"], "linux-proc-meminfo"


def _darwin_memory() -> tuple[int, int, str]:
    total = int(
        subprocess.run(["sysctl", "-n", "hw.memsize"], capture_output=True, text=True, check=True).stdout
    )
    out = subprocess.run(["vm_stat"], capture_output=True, text=True, check=True).stdout
    page_size = 4096
    m = re.search(r"page size of (\d+) bytes", out)
    if m:
        page_size = int(m.group(1))
    pages = {}
    for line in out.splitlines():
        key, _, rest = line.partition(":")
        mm = re.match(r"\s*(\d+)\.?$", rest.strip())
        if mm:
            pages[key.strip()] = int(mm.group(1))
    free = pages.get("Pages free", 0) + pages.get("Pages inactive", 0) + pages.get("Pages speculative", 0)
    return free * page_size, total, "darwin-vm-stat"


def machine_memory() -> tuple[int, int, str]:
    if sys.platform.startswith("win"):
        return _windows_memory()
    if sys.platform.startswith("linux"):
        return _linux_memory()
    if sys.platform == "darwin":
        return _darwin_memory()
    raise ToolError(f"不支持的平台 {sys.platform!r}：请用 YX_TEST_MEM_BUDGET 显式给预算")


# ---------------------------------------------------------------- 预算 / 并发上限


@dataclasses.dataclass
class Limits:
    budget_bytes: int
    budget_explain: str
    machine_available: int | None
    machine_total: int | None
    machine_source: str
    reserve_bytes: int
    cpus: int
    default_test_peak: int
    build_job_peak: int
    cargo_build_jobs: int
    cargo_build_jobs_origin: str
    rust_test_threads: int
    rust_test_threads_origin: str


def _env_size(env, key: str, default: int | None) -> int | None:
    raw = env.get(key)
    if raw is None or not raw.strip():
        return default
    return parse_size(raw)


def _env_positive_int(env, key: str) -> int | None:
    raw = env.get(key)
    if raw is None or not raw.strip():
        return None
    try:
        value = int(raw.strip())
    except ValueError:
        raise ToolError(f"{key} 必须是正整数，实际为 {raw!r}")
    if value <= 0:
        raise ToolError(f"{key} 必须是正整数，实际为 {raw!r}")
    return value


def resolve_limits(env=None) -> Limits:
    env = dict(os.environ if env is None else env)
    cpus = os.cpu_count() or 1
    reserve = _env_size(env, "YX_TEST_MEM_RESERVE", DEFAULT_RESERVE)
    default_test_peak = _env_size(env, "YX_TEST_DEFAULT_PEAK", DEFAULT_TEST_PEAK)
    build_job_peak = _env_size(env, "YX_TEST_BUILD_JOB_PEAK", DEFAULT_BUILD_JOB_PEAK)
    if default_test_peak <= 0 or build_job_peak <= 0:
        raise ToolError("YX_TEST_DEFAULT_PEAK / YX_TEST_BUILD_JOB_PEAK 必须为正")
    explicit = _env_size(env, "YX_TEST_MEM_BUDGET", None)

    available = total = None
    source = "none"
    try:
        available, total, source = machine_memory()
    except ToolError:
        if explicit is None:
            raise

    pieces = []
    if explicit is not None:
        pieces.append(f"YX_TEST_MEM_BUDGET={human_bytes(explicit)}")
    machine_budget = None
    if available is not None:
        machine_budget = available - reserve
        pieces.append(f"可用 {human_bytes(available)} - 保留 {human_bytes(reserve)}")
        if machine_budget <= 0:
            machine_budget = None
            pieces.append("机器可用量已低于保留量")
    if explicit is not None and machine_budget is not None:
        budget = min(explicit, machine_budget)
    elif explicit is not None:
        budget = explicit
    elif machine_budget is not None:
        budget = machine_budget
    else:
        raise ToolError("既读不到机器可用内存，也没有设置 YX_TEST_MEM_BUDGET")
    if budget <= 0:
        raise ToolError("预算 <= 0：请提高 YX_TEST_MEM_BUDGET 或降低 YX_TEST_MEM_RESERVE")

    derived_jobs = max(1, min(cpus, budget // build_job_peak))
    env_jobs = _env_positive_int(env, "CARGO_BUILD_JOBS")
    if env_jobs is None:
        jobs = derived_jobs
        jobs_origin = f"按预算派生（{human_bytes(budget)} // {human_bytes(build_job_peak)}/作业，上限 cpu {cpus}）"
    elif env_jobs > derived_jobs:
        jobs = derived_jobs
        jobs_origin = f"环境给 {env_jobs}，按预算收紧到 {derived_jobs}（{human_bytes(build_job_peak)}/作业）"
    else:
        jobs = env_jobs
        jobs_origin = f"环境显式 {env_jobs}（预算允许 {derived_jobs}）"

    derived_threads = max(1, min(cpus, budget // default_test_peak))
    env_threads = _env_positive_int(env, "RUST_TEST_THREADS")
    if env_threads is None:
        threads = derived_threads
        threads_origin = f"按预算派生（{human_bytes(budget)} // {human_bytes(default_test_peak)}/线程，上限 cpu {cpus}）"
    elif env_threads > derived_threads:
        threads = derived_threads
        threads_origin = f"环境给 {env_threads}，按预算收紧到 {derived_threads}（{human_bytes(default_test_peak)}/线程）"
    else:
        threads = env_threads
        threads_origin = f"环境显式 {env_threads}（预算允许 {derived_threads}）"

    explain = "min(" + ", ".join(pieces) + f") = {human_bytes(budget)}"
    return Limits(
        budget_bytes=budget,
        budget_explain=explain,
        machine_available=available,
        machine_total=total,
        machine_source=source,
        reserve_bytes=reserve,
        cpus=cpus,
        default_test_peak=default_test_peak,
        build_job_peak=build_job_peak,
        cargo_build_jobs=jobs,
        cargo_build_jobs_origin=jobs_origin,
        rust_test_threads=threads,
        rust_test_threads_origin=threads_origin,
    )


def print_limits(limits: Limits) -> None:
    """预算与「实际使用的 CARGO_BUILD_JOBS / RUST_TEST_THREADS」——每条 cargo 调用的前置事实。"""
    machine = "未知"
    if limits.machine_available is not None:
        machine = f"可用 {human_bytes(limits.machine_available)} / 总计 {human_bytes(limits.machine_total)}"
    print(
        f"[budget] 机器 {machine}（{limits.machine_source}），保留 {human_bytes(limits.reserve_bytes)}，cpu {limits.cpus}",
        file=sys.stderr,
    )
    print(f"[budget] 预算 = {limits.budget_explain}", file=sys.stderr)
    print(f"[limits] CARGO_BUILD_JOBS={limits.cargo_build_jobs}（{limits.cargo_build_jobs_origin}）", file=sys.stderr)
    print(f"[limits] RUST_TEST_THREADS={limits.rust_test_threads}（{limits.rust_test_threads_origin}）", file=sys.stderr)


# ---------------------------------------------------------------- 标记文法


class MarkerError(Exception):
    pass


@dataclasses.dataclass(frozen=True)
class CostMarker:
    peak_bytes: int
    time_secs: float
    mode: str
    source: str | None
    raw: str


PEAK_RE = re.compile(r"^([0-9]+)(B|KiB|MiB|GiB)$")
TIME_RE = re.compile(r"^([0-9]+(?:\.[0-9]+)?)s$")
MEASURED_RE = re.compile(r"^measured@([0-9]{4})-([0-9]{2})-([0-9]{2})$")
EXTRAPOLATED_RE = re.compile(r"^extrapolated@(\S+)$")


def parse_cost_marker(text: str) -> CostMarker:
    """严格 token 解析：空格分隔的 key=value，键序无关，未知键/单位/mode 一律抛 MarkerError。"""
    raw = text.strip()
    tokens = raw.split()
    if not tokens:
        raise MarkerError("cost: 标记为空")
    seen: dict[str, str] = {}
    for token in tokens:
        if "=" not in token:
            raise MarkerError(f"token {token!r} 不是 key=value 形态（严格文法不接受散文）")
        key, _, value = token.partition("=")
        if key in seen:
            raise MarkerError(f"键 {key!r} 重复出现")
        if key not in ("peak", "time", "mode", "source"):
            raise MarkerError(f"未知键 {key!r}（只允许 peak/time/mode/source）")
        seen[key] = value
    missing = [k for k in ("peak", "time", "mode") if k not in seen]
    if missing:
        raise MarkerError(f"缺少必填键 {', '.join(missing)}")

    peak_match = PEAK_RE.fullmatch(seen["peak"])
    if not peak_match:
        raise MarkerError(f"peak={seen['peak']!r} 非法（期望 <整数><B|KiB|MiB|GiB>）")
    peak_bytes = int(peak_match.group(1)) * UNITS[peak_match.group(2)]

    time_match = TIME_RE.fullmatch(seen["time"])
    if not time_match:
        raise MarkerError(f"time={seen['time']!r} 非法（期望 <整数或小数>s）")
    time_secs = float(time_match.group(1))

    mode = seen["mode"]
    if mode not in MODES:
        raise MarkerError(f"mode={mode!r} 非法（只允许 shared|exclusive|manual）")

    source = seen.get("source")
    if source is None:
        if mode in ("exclusive", "manual"):
            raise MarkerError(f"mode={mode} 必须自报 source（measured@YYYY-MM-DD 或 extrapolated@<依据>）")
    else:
        measured = MEASURED_RE.fullmatch(source)
        if measured:
            try:
                datetime.date(int(measured.group(1)), int(measured.group(2)), int(measured.group(3)))
            except ValueError:
                raise MarkerError(f"source={source!r} 的日期不存在")
        elif not EXTRAPOLATED_RE.fullmatch(source):
            raise MarkerError(
                f"source={source!r} 非法（期望 measured@YYYY-MM-DD 或 extrapolated@<无空格依据标识>）"
            )
    return CostMarker(peak_bytes=peak_bytes, time_secs=time_secs, mode=mode, source=source, raw=raw)


# ---------------------------------------------------------------- Rust 源码扫描


COST_LINE_RE = re.compile(r"^\s*///?\s*cost:\s*(?P<rest>.*)$")
TEST_ATTR_RE = re.compile(r"^\s*#\[test\]\s*$")
IGNORE_ATTR_RE = re.compile(r"^\s*#\[ignore\b")
FN_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)
EXCLUSIVE_CALL_RE = re.compile(r"exclusive_lock\s*\(\s*\"((?:[^\"\\]|\\.)*)\"")


@dataclasses.dataclass
class TestItem:
    path: Path
    line: int
    fn_name: str
    fn_line: int
    marker: CostMarker | None = None
    marker_text: str | None = None
    marker_problems: list[str] = dataclasses.field(default_factory=list)
    has_ignore: bool = False
    body: str = ""
    module_path: str | None = None
    full_name: str | None = None
    name_origin: str | None = None


@dataclasses.dataclass
class ScanResult:
    items: list[TestItem] = dataclasses.field(default_factory=list)
    stray_markers: list[tuple[Path, int, str]] = dataclasses.field(default_factory=list)


def _is_block_line(line: str) -> bool:
    s = line.strip()
    return (
        s.startswith("//")
        or (s.startswith("#[") and s.endswith("]"))
        or (s.startswith("#![") and s.endswith("]"))
    )


def _brace_body(text: str, open_index: int) -> str:
    """从 open_index 处的 { 起做括号配对，跳过注释 / 字符串 / 字符字面量。"""
    depth = 0
    i = open_index
    n = len(text)
    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if ch == "/" and nxt == "/":
            nl = text.find("\n", i)
            i = n if nl < 0 else nl + 1
            continue
        if ch == "/" and nxt == "*":
            end = text.find("*/", i + 2)
            i = n if end < 0 else end + 2
            continue
        if ch == "r" and nxt in ('"', "#"):
            m = re.match(r'r(#*)"', text[i:])
            if m:
                closing = '"' + m.group(1)
                end = text.find(closing, i + m.end())
                i = n if end < 0 else end + len(closing)
                continue
        if ch == '"':
            i += 1
            while i < n:
                if text[i] == "\\":
                    i += 2
                    continue
                if text[i] == '"':
                    i += 1
                    break
                i += 1
            continue
        if ch == "'" and nxt != "":
            m = re.match(r"'(?:[^'\\]|\\.)'", text[i:])
            if m:
                i += m.end()
                continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[open_index + 1 : i]
        i += 1
    return text[open_index + 1 :]


def scan_text(rel_path: str, text: str, module_path: str | None = None) -> ScanResult:
    """扫描单文件：#[test] 项 + 其上方紧邻注释块里的 cost 标记 + 游离标记。"""
    lines = text.splitlines()
    result = ScanResult()
    consumed_marker_lines: set[int] = set()

    for idx, line in enumerate(lines):
        if not TEST_ATTR_RE.match(line):
            continue

        # 1) 上方紧邻的属性/文档注释块（允许 #[cfg]/#[ignore]/文档注释混排）
        top = idx - 1
        while top >= 0 and _is_block_line(lines[top]):
            top -= 1
        block = list(range(top + 1, idx))

        # 2) 向下找 fn（跳过属性/注释/空行），同时收集 #[ignore]
        fn_name = None
        fn_line = None
        has_ignore = any(IGNORE_ATTR_RE.match(lines[k]) for k in block)
        cursor = idx + 1
        while cursor < len(lines) and cursor < idx + 80:
            stripped = lines[cursor].strip()
            if IGNORE_ATTR_RE.match(lines[cursor]):
                has_ignore = True
            if FN_RE.match(lines[cursor]):
                fn_name = FN_RE.match(lines[cursor]).group(1)
                fn_line = cursor
                break
            if stripped and not stripped.startswith("//") and not stripped.startswith("#["):
                break
            cursor += 1
        if fn_name is None or fn_line is None:
            continue

        # 3) 函数体（从签名之后的第一个 { 起配对）
        brace = -1
        scan_from = sum(len(l) + 1 for l in lines[:fn_line])
        for pos in range(scan_from, min(len(text), scan_from + 4000)):
            if text[pos] == "{":
                brace = pos
                break
        body = _brace_body(text, brace) if brace >= 0 else ""

        item = TestItem(
            path=Path(rel_path),
            line=idx + 1,
            fn_name=fn_name,
            fn_line=fn_line + 1,
            has_ignore=has_ignore,
            body=body,
            module_path=module_path,
        )

        # 4) 标记（上文块内，单行；0 条 = 未标记，>1 条 = 重复）
        marker_lines = [k for k in block if COST_LINE_RE.match(lines[k])]
        consumed_marker_lines.update(k + 1 for k in marker_lines)
        if len(marker_lines) > 1:
            item.marker_problems.append(
                f"同一个 #[test] 上方出现 {len(marker_lines)} 条 cost: 标记（只允许一条）"
            )
        elif len(marker_lines) == 1:
            raw = COST_LINE_RE.match(lines[marker_lines[0]]).group("rest")
            item.marker_text = raw.strip()
            try:
                item.marker = parse_cost_marker(raw)
            except MarkerError as exc:
                item.marker_problems.append(f"cost: 标记文法错误：{exc}")
        result.items.append(item)

    # 5) 游离标记：形似 cost: 但从没落在任何 #[test] 上方块内
    for idx, line in enumerate(lines):
        if COST_LINE_RE.match(line) and (idx + 1) not in consumed_marker_lines:
            result.stray_markers.append(
                (Path(rel_path), idx + 1, COST_LINE_RE.match(line).group("rest").strip())
            )
    return result


def module_path_for(path: Path, root: Path) -> str | None:
    try:
        rel = path.relative_to(root)
    except ValueError:
        return None
    parts = list(rel.parts)
    if not parts:
        return None
    if parts[0] == "tests":
        return ""  # 集成测试 crate 根 = 文件本身
    if parts[0] != "src":
        return None
    rest = parts[1:]
    if not rest[-1].endswith(".rs"):
        return None
    rest[-1] = rest[-1][: -len(".rs")]
    if rest[-1] in ("lib", "main", "mod"):
        rest = rest[:-1]
    return "::".join(rest)


def iter_source_files(root: Path):
    for base in (root / "src", root / "tests"):
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            # 判据必须按**相对 root** 的路径组件算：AGENTS 第 9 条把批次树
            # 定在 <repo>/target/yx-scratch/<batch>/tree，若按绝对路径判，
            # 树内每个文件都含 "target" 组件而被全部跳过——扫描恒为 0 项，
            # fast/full 通道会「绿但空」（2026-10-30 实测：树内 0 项 /
            # 主工作树 3252 项，同树 cargo test --lib -- --list 列得出 2882 个）。
            rel_parts = path.relative_to(root).parts
            if any(part in (".git", "target", "node_modules") for part in rel_parts):
                continue
            yield path


def scan_sources(root: Path) -> ScanResult:
    """扫描 root 下的源码，产出计划条目。

    空扫描一律报错：本仓库的 src/tests 不可能一个 .rs 都没有，扫到 0 项
    只可能意味着路径判据把整棵树都排除了——那样后续计划为空、通道
    「绿但空」，比直接失败危险得多（2026-10-30 实例：批次树内 0 项）。
    """
    aggregate = ScanResult()
    scanned = 0
    for path in iter_source_files(root):
        scanned += 1
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            raise ToolError(f"读不到 {path}: {exc}")
        result = scan_text(str(path), text, module_path_for(path, root))
        aggregate.items.extend(result.items)
        aggregate.stray_markers.extend(result.stray_markers)
    if scanned == 0:
        raise ToolError(
            f"源码扫描为 0 项（root={root}）：路径判据把整棵树排除了，"
            "计划会为空、通道将「绿但空」。请检查 iter_source_files 的排除判据。"
        )
    return aggregate


# ---------------------------------------------------------------- cargo 测试名


def cargo_env(limits: Limits) -> dict:
    env = os.environ.copy()
    env["CARGO_BUILD_JOBS"] = str(limits.cargo_build_jobs)
    env["RUST_TEST_THREADS"] = str(limits.rust_test_threads)
    return env


def decide_targets(scan: ScanResult, root: Path) -> str:
    """自动选目标：标记/ignore 只出现在 src/ 时用 lib（快），否则 all-targets（全）。"""
    for item in scan.items:
        if not (item.marker or item.has_ignore):
            continue
        rel = item.path.relative_to(root)
        if rel.parts and rel.parts[0] != "src":
            return "all"
    return "lib"


def cargo_test_list(root: Path, targets: str, env: dict) -> set[str]:
    args = ["cargo", "test"]
    args += ["--lib"] if targets == "lib" else ["--all-targets"]
    args += ["--", "--list"]
    # encoding 必须显式指定：Windows 默认 GBK，构建脚本的中文警告会让
    # 解码线程抛 UnicodeDecodeError，把真正的失败原因掩盖掉。
    proc = subprocess.run(
        args, cwd=root, env=env, capture_output=True, text=True,
        encoding="utf-8", errors="replace",
    )
    if proc.returncode != 0:
        tail = "\n".join(proc.stderr.strip().splitlines()[-15:])
        raise ToolError(f"cargo test --list 失败（exit {proc.returncode}）：\n{tail}")
    names = set()
    for line in proc.stdout.splitlines():
        m = re.match(r"^(?P<name>\S.*?): test$", line)
        if m:
            names.add(m.group("name").strip())
    if not names:
        raise ToolError("cargo test --list 没有列出任何测试名（判据无法交叉核对）")
    return names


def resolve_names(scan: ScanResult, cargo_names: set[str]) -> None:
    """给每个需要名字的项（带标记或 ignore）定出 cargo 侧的完整测试名。"""
    for item in scan.items:
        if not (item.marker or item.has_ignore):
            continue
        module = item.module_path
        if module is None:
            item.name_origin = "文件不在 src/ 或 tests/ 下，无法推导模块路径"
            continue
        full = f"{module}::{item.fn_name}" if module else item.fn_name
        if full in cargo_names:
            item.full_name = full
            item.name_origin = "exact"
            continue
        candidates = [n for n in cargo_names if n == item.fn_name or n.endswith("::" + item.fn_name)]
        if len(candidates) == 1:
            item.full_name = candidates[0]
            item.name_origin = f"suffix（{candidates[0]}）"
        else:
            item.name_origin = "cargo test --list 发现不到" if not candidates else "同名歧义"


# ---------------------------------------------------------------- markers --check


def exclusive_call_arg(body: str) -> str | None:
    stripped = re.sub(r"//[^\n]*", "", body)
    m = EXCLUSIVE_CALL_RE.search(stripped)
    return None if m is None else m.group(1)


def rel(root: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def cmd_markers(args) -> int:
    root = resolve_root(args)
    limits = resolve_limits()
    print_limits(limits)

    scan = scan_sources(root)
    problems: list[str] = []

    for item in scan.items:
        for problem in item.marker_problems:
            problems.append(f"{rel(root, item.path)}:{item.line} {item.fn_name}: {problem}")
        if item.has_ignore and item.marker is None and not item.marker_problems:
            problems.append(
                f"{rel(root, item.path)}:{item.line} {item.fn_name}: #[ignore] 测试缺 cost: 标记"
                "（就地声明在 #[test] 上方紧邻的注释块内）"
            )

    targets = decide_targets(scan, root)
    cargo_names: set[str] = set()
    if not args.no_cargo:
        cargo_names = cargo_test_list(root, targets, cargo_env(limits))
        resolve_names(scan, cargo_names)

    for item in scan.items:
        if item.marker is None:
            continue
        if not args.no_cargo and item.full_name is None:
            problems.append(
                f"{rel(root, item.path)}:{item.line} {item.fn_name}: cost: 标记落在 cargo test --list 发现不到的测试上"
                f"（{item.name_origin or '未解析'}）"
            )
        if item.marker.mode == "exclusive":
            arg = exclusive_call_arg(item.body)
            if arg is None:
                problems.append(
                    f"{rel(root, item.path)}:{item.line} {item.fn_name}: mode=exclusive 的测试体未调用独占助手"
                    ' crate::util::test_support::exclusive_lock("<测试名>")'
                )
            elif arg != item.fn_name:
                problems.append(
                    f"{rel(root, item.path)}:{item.line} {item.fn_name}: exclusive_lock({arg!r}) 与测试名不一致"
                    "（锁文件名必须等于测试名）"
                )

    for path, line, text in scan.stray_markers:
        problems.append(
            f"{rel(root, path)}:{line}: 游离的 cost: 标记（{text}）——必须落在某个 #[test] 上方紧邻块内"
        )

    marker_items = [it for it in scan.items if it.marker is not None]
    by_mode = {mode: sum(1 for it in marker_items if it.marker.mode == mode) for mode in MODES}
    ignored = [it for it in scan.items if it.has_ignore]
    print(
        f"[markers] 扫描 {len(scan.items)} 个 #[test]；cost: 标记 {len(marker_items)} 条"
        f"（shared {by_mode['shared']} / exclusive {by_mode['exclusive']} / manual {by_mode['manual']}）；"
        f"#[ignore] {len(ignored)} 个"
    )
    if args.no_cargo:
        print("[markers] 注意：--no-cargo 跳过了 cargo test --list 交叉核对（不得用于门禁）", file=sys.stderr)
    else:
        print(f"[markers] cargo 交叉核对：{targets} 目标 {len(cargo_names)} 条测试名")

    if problems:
        for problem in problems:
            print(f"[FAIL] {problem}")
        print(f"[markers] 违规 {len(problems)} 条")
        return 1
    print("[markers] OK：每个 #[ignore] 有标记、每个标记落在可发现的 #[test] 上、每个 exclusive 体都取了独占锁")
    return 0


# ---------------------------------------------------------------- markers --selftest


SELFTEST_VALID = [
    "peak=0B time=0s mode=shared",
    "peak=512MiB time=1.5s mode=shared source=measured@2026-10-13",
    "peak=100KiB time=1s mode=exclusive source=extrapolated@issue-449",
    "peak=8307MiB time=38s mode=exclusive source=extrapolated@issue-449-5000-tier-ratio",
    "peak=15974MiB time=72.6s mode=manual source=measured@2026-10-13",
    "  mode=manual   peak=1GiB   source=measured@2026-10-21   time=2s  ",
    "peak=1GiB time=0.5s mode=shared",
]

SELFTEST_INVALID = [
    ("", "为空"),
    ("time=1s mode=shared", "缺少必填键 peak"),
    ("peak=1GiB mode=shared", "缺少必填键 time"),
    ("peak=1GiB time=1s", "缺少必填键 mode"),
    ("peak=1GiB time=1s mode=shared budget=1GiB", "未知键"),
    ("peak=1TB time=1s mode=shared", "peak"),
    ("peak=1.5GiB time=1s mode=shared", "peak"),
    ("peak=-1GiB time=1s mode=shared", "peak"),
    ("peak= time=1s mode=shared", "peak"),
    ("peak=1GiB time=1 mode=shared", "time"),
    ("peak=1GiB time=-1s mode=shared", "time"),
    ("peak=1GiB time=1s mode=solo", "mode"),
    ("peak=1GiB time=1s mode=exclusive", "source"),
    ("peak=1GiB time=1s mode=manual", "source"),
    ("peak=1GiB time=1s mode=shared source=measured@2026-13-01", "日期"),
    ("peak=1GiB time=1s mode=shared source=measured@2026-1-1", "source"),
    ("peak=1GiB time=1s mode=shared source=extrapolated@", "source"),
    ("peak=1GiB time=1s mode=shared source=estimated@2026-10-13", "source"),
    ("peak=1GiB peak=2GiB time=1s mode=shared", "重复"),
    ("peak=1GiB time=1s mode=shared peak", "key=value"),
    ("peak=1GiB time=1s mode=shared 注解", "key=value"),
]

SELFTEST_SCAN_CASES = [
    (
        "文档注释块 + exclusive 助手",
        "/// 说明\n/// cost: peak=1GiB time=1s mode=exclusive source=measured@2026-10-13\n#[test]\n"
        'fn test_a() {\n    let _g = crate::util::test_support::exclusive_lock("test_a");\n}\n',
        {"markers": 1, "strays": 0, "fn": "test_a", "ignore": False, "exclusive_arg": "test_a", "problems": 0},
    ),
    (
        "行注释 + #[ignore] 标记",
        "// cost: peak=1GiB time=1s mode=manual source=measured@2026-10-13\n#[test]\n"
        '#[ignore = "人工"]\nfn test_b() {}\n',
        {"markers": 1, "strays": 0, "fn": "test_b", "ignore": True, "exclusive_arg": None, "problems": 0},
    ),
    (
        "标记与 #[test] 之间隔空行 → 游离",
        "// cost: peak=1GiB time=1s mode=shared\n\n#[test]\nfn test_c() {}\n",
        {"markers": 0, "strays": 1, "fn": "test_c", "ignore": False, "exclusive_arg": None, "problems": 0},
    ),
    (
        "标记落在普通 fn 上方 → 游离",
        "// cost: peak=1GiB time=1s mode=shared\nfn helper() {}\n",
        {"markers": 0, "strays": 1, "fn": None, "ignore": False, "exclusive_arg": None, "problems": 0},
    ),
    (
        "同一测试两条标记 → 重复违规",
        "// cost: peak=1GiB time=1s mode=shared\n// cost: peak=2GiB time=1s mode=shared\n#[test]\nfn test_d() {}\n",
        {"markers": 0, "strays": 0, "fn": "test_d", "ignore": False, "exclusive_arg": None, "problems": 1},
    ),
    (
        "exclusive 助手参数与测试名不一致",
        "// cost: peak=1GiB time=1s mode=exclusive source=measured@2026-10-13\n#[test]\n"
        'fn test_e() {\n    let _g = exclusive_lock("other");\n}\n',
        {"markers": 1, "strays": 0, "fn": "test_e", "ignore": False, "exclusive_arg": "other", "problems": 0},
    ),
]


def cmd_markers_selftest(args) -> int:
    failures = 0
    for text in SELFTEST_VALID:
        try:
            parse_cost_marker(text)
        except MarkerError as exc:
            print(f"[FAIL] 正例被拒：{text!r} -> {exc}")
            failures += 1
    for text, expected in SELFTEST_INVALID:
        try:
            parse_cost_marker(text)
            print(f"[FAIL] 反例被接受：{text!r}（期望报 {expected!r}）")
            failures += 1
        except MarkerError as exc:
            if expected not in str(exc):
                print(f"[FAIL] 反例错误信息不含 {expected!r}：{text!r} -> {exc}")
                failures += 1

    for label, source, expected in SELFTEST_SCAN_CASES:
        scan = scan_text("src/selftest.rs", source, "selftest")
        first = scan.items[0] if scan.items else None
        got = {
            "markers": sum(1 for it in scan.items if it.marker is not None),
            "strays": len(scan.stray_markers),
            "fn": first.fn_name if first else None,
            "ignore": first.has_ignore if first else False,
            "exclusive_arg": exclusive_call_arg(first.body) if first else None,
            "problems": sum(len(it.marker_problems) for it in scan.items),
        }
        for key, want in expected.items():
            if got[key] != want:
                print(f"[FAIL] 扫描器用例「{label}」的 {key} 期望 {want!r}，实际 {got[key]!r}")
                failures += 1

    valid = len(SELFTEST_VALID) + len(SELFTEST_SCAN_CASES)
    invalid = len(SELFTEST_INVALID) + 2  # 扫描器负例：游离标记两条
    if failures:
        print(f"[markers --selftest] FAIL：{failures} 条不达预期（正例 {valid} / 反例 {invalid}）")
        return 1
    print(
        f"[markers --selftest] OK：文法正例 {len(SELFTEST_VALID)} 条 / 反例 {len(SELFTEST_INVALID)} 条；"
        f"扫描器用例 {len(SELFTEST_SCAN_CASES)} 条（含游离/重复负例）—— 合计正例 {valid} / 反例 {invalid}"
    )
    return 0


# ---------------------------------------------------------------- 计划 / 波


@dataclasses.dataclass
class Wave:
    kind: str  # batch | exclusive | singleton
    items: list[TestItem]
    unmarked_count: int
    peak_bytes: int
    time_secs: float
    fits: bool
    command: list[str]
    env: dict
    total: int = 0
    index: int = 0


@dataclasses.dataclass
class Plan:
    waves: list[Wave]
    default_count: int
    batch_count: int
    ignored: list[TestItem]
    manual: list[TestItem]
    targets: str


def _wave_env(limits: Limits, threads: int) -> dict:
    return {"CARGO_BUILD_JOBS": str(limits.cargo_build_jobs), "RUST_TEST_THREADS": str(threads)}


def _base_command(targets: str) -> list[str]:
    return ["cargo", "test"] + (["--lib"] if targets == "lib" else ["--all-targets"]) + ["--"]


def _in_targets(item: TestItem, targets: str, root: Path) -> bool:
    """计划只统计实际会被这条 cargo 命令选中的目标（lib 目标 = src/ 下的单元测试）。"""
    if targets == "all":
        return True
    try:
        rel_path = item.path.relative_to(root)
    except ValueError:
        return False
    return bool(rel_path.parts) and rel_path.parts[0] == "src"


def build_plan(scan: ScanResult, limits: Limits, targets: str, root: Path) -> Plan:
    scoped = [it for it in scan.items if _in_targets(it, targets, root)]
    default = [
        it for it in scoped if not it.has_ignore and (it.marker is None or it.marker.mode != "manual")
    ]
    ignored = [it for it in scoped if it.has_ignore]
    manual = [it for it in scoped if it.marker is not None and it.marker.mode == "manual"]
    exclusive = [it for it in default if it.marker is not None and it.marker.mode == "exclusive"]
    exclusive_ids = {id(it) for it in exclusive}
    batch = [it for it in default if id(it) not in exclusive_ids]

    def batch_peak(members: list[TestItem]) -> int:
        marked = sum(it.marker.peak_bytes for it in members if it.marker is not None)
        return marked + limits.rust_test_threads * limits.default_test_peak

    # 按预算装箱：批量波超标时，把最大的已标记共享测试拆成独享波，直到批量波装得下。
    singletons: list[TestItem] = []
    peak = batch_peak(batch)
    while batch and peak > limits.budget_bytes:
        marked = [it for it in batch if it.marker is not None]
        if not marked:
            break
        worst = max(marked, key=lambda it: it.marker.peak_bytes)
        batch.remove(worst)
        singletons.append(worst)
        peak = batch_peak(batch)

    waves: list[Wave] = []
    if batch:
        # 批量波必须显式跳过：独占/独享波成员（各自单独一波）+ mode=manual（两条通道都不得执行）
        skip_names = [it.full_name for it in exclusive + singletons + manual if it.full_name]
        command = _base_command(targets) + [f"--test-threads={limits.rust_test_threads}"]
        for name in skip_names:
            command += ["--skip", name]
        waves.append(
            Wave(
                kind="batch",
                items=[it for it in batch if it.marker is not None],
                unmarked_count=sum(1 for it in batch if it.marker is None),
                peak_bytes=batch_peak(batch),
                time_secs=sum(it.marker.time_secs for it in batch if it.marker is not None),
                fits=batch_peak(batch) <= limits.budget_bytes,
                command=command,
                env=_wave_env(limits, limits.rust_test_threads),
            )
        )
    for item in exclusive + singletons:
        command = _base_command(targets) + [
            "--exact",
            item.full_name or item.fn_name,
            "--test-threads=1",
            "--nocapture",
        ]
        waves.append(
            Wave(
                kind="exclusive" if item in exclusive else "singleton",
                items=[item],
                unmarked_count=0,
                peak_bytes=item.marker.peak_bytes,
                time_secs=item.marker.time_secs,
                fits=item.marker.peak_bytes <= limits.budget_bytes,
                command=command,
                env=_wave_env(limits, 1),
            )
        )
    for idx, wave in enumerate(waves, 1):
        wave.index = idx
        wave.total = len(waves)
    return Plan(
        waves=waves,
        default_count=len(default),
        batch_count=len(batch),
        ignored=ignored,
        manual=manual,
        targets=targets,
    )


def _shell_join(args: list[str]) -> str:
    return " ".join(a if re.fullmatch(r"[A-Za-z0-9_./:=+-]+", a) else repr(a) for a in args)


def plan_to_json(plan: Plan, limits: Limits) -> dict:
    def wave_json(wave: Wave) -> dict:
        return {
            "kind": wave.kind,
            "members": [it.full_name or it.fn_name for it in wave.items],
            "unmarked_count": wave.unmarked_count,
            "peak_bytes": wave.peak_bytes,
            "peak_human": human_bytes(wave.peak_bytes),
            "time_secs": round(wave.time_secs, 3),
            "fits_budget": wave.fits,
            "action": "run" if wave.fits else "defer",
            "command": wave.command,
            "env": wave.env,
        }

    return {
        "budget": {
            "bytes": limits.budget_bytes,
            "human": human_bytes(limits.budget_bytes),
            "explain": limits.budget_explain,
            "machine": {
                "available_bytes": limits.machine_available,
                "total_bytes": limits.machine_total,
                "source": limits.machine_source,
                "reserve_bytes": limits.reserve_bytes,
                "cpus": limits.cpus,
            },
        },
        "limits": {
            "cargo_build_jobs": limits.cargo_build_jobs,
            "cargo_build_jobs_origin": limits.cargo_build_jobs_origin,
            "rust_test_threads": limits.rust_test_threads,
            "rust_test_threads_origin": limits.rust_test_threads_origin,
            "build_job_peak_bytes": limits.build_job_peak,
            "default_test_peak_bytes": limits.default_test_peak,
        },
        "counts": {
            "default_set": plan.default_count,
            "batch_members": plan.batch_count,
            "ignored": len(plan.ignored),
            "manual": len(plan.manual),
        },
        "targets": plan.targets,
        "waves": [wave_json(w) for w in plan.waves],
        "deferred": [
            {
                "kind": w.kind,
                "name": w.items[0].full_name if w.items else w.kind,
                "peak_bytes": w.peak_bytes,
                "peak_human": human_bytes(w.peak_bytes),
                "gap_bytes": w.peak_bytes - limits.budget_bytes,
                "gap_human": human_bytes(w.peak_bytes - limits.budget_bytes),
            }
            for w in plan.waves
            if not w.fits
        ],
        "manual": [it.full_name or it.fn_name for it in plan.manual],
        "ignored": [it.full_name or it.fn_name for it in plan.ignored],
    }


def print_plan(plan: Plan, limits: Limits) -> None:
    exclusive_waves = sum(1 for w in plan.waves if w.kind != "batch")
    print(
        f"[plan] 默认集 {plan.default_count} 条：批量波 {plan.batch_count} 条；"
        f"独占/独享波 {exclusive_waves} 个；跳过 #[ignore] {len(plan.ignored)} 条、mode=manual {len(plan.manual)} 条"
    )
    for wave in plan.waves:
        if wave.kind == "batch":
            members = (
                f"未标记 {wave.unmarked_count} 条（按 RUST_TEST_THREADS 并发）"
                f" + 已标记 {len(wave.items)} 条"
            )
        else:
            members = ", ".join(it.full_name or it.fn_name for it in wave.items)
        verdict = (
            "执行"
            if wave.fits
            else f"装不下（缺口 {human_bytes(wave.peak_bytes - limits.budget_bytes)}）"
        )
        print(
            f"[plan] 波 {wave.index}/{wave.total} {wave.kind:<9} 成员：{members}\n"
            f"        峰值合计 {human_bytes(wave.peak_bytes)} / 预算 {human_bytes(limits.budget_bytes)}"
            f"（预计 {wave.time_secs:.1f}s）→ {verdict}\n"
            f"        命令：CARGO_BUILD_JOBS={wave.env['CARGO_BUILD_JOBS']}"
            f" RUST_TEST_THREADS={wave.env['RUST_TEST_THREADS']} {_shell_join(wave.command)}"
        )
    if plan.manual:
        print("[plan] mode=manual（fast/full 均不执行，需人工显式跑）：")
        for item in plan.manual:
            print(f"        - {item.full_name or item.fn_name}")


def _load_plan(args):
    root = resolve_root(args)
    limits = resolve_limits()
    print_limits(limits)
    scan = scan_sources(root)
    targets = decide_targets(scan, root)
    cargo_names = cargo_test_list(root, targets, cargo_env(limits))
    resolve_names(scan, cargo_names)
    unresolved = [it for it in scan.items if (it.marker or it.has_ignore) and it.full_name is None]
    if unresolved:
        names = ", ".join(f"{rel(root, it.path)}:{it.line} {it.fn_name}" for it in unresolved)
        raise ToolError(f"这些测试无法映射到 cargo test --list 的名字，不能装箱：{names}")
    return root, limits, scan, build_plan(scan, limits, targets, root)


def cmd_plan(args) -> int:
    _root, limits, _scan, plan = _load_plan(args)
    if args.json:
        print(json.dumps(plan_to_json(plan, limits), ensure_ascii=False, indent=2))
    else:
        print_plan(plan, limits)
    return 0


# ---------------------------------------------------------------- run fast / full


def _gap_table(title: str, waves: list[Wave], limits: Limits, tail: str) -> None:
    print(title)
    for wave in waves:
        name = wave.items[0].full_name if wave.items else f"{wave.kind} 波"
        gap = human_bytes(wave.peak_bytes - limits.budget_bytes)
        print(
            f"  - {name}\n"
            f"    峰值 {human_bytes(wave.peak_bytes)} > 预算 {human_bytes(limits.budget_bytes)}，缺口 {gap}"
        )
    if tail:
        print(tail)


def _execute_wave(root: Path, wave: Wave, limits: Limits) -> int:
    print(
        f"[run] 波 {wave.index}/{wave.total} {wave.kind}：CARGO_BUILD_JOBS={wave.env['CARGO_BUILD_JOBS']}"
        f" RUST_TEST_THREADS={wave.env['RUST_TEST_THREADS']} {_shell_join(wave.command)}"
    )
    env = cargo_env(limits)
    env.update(wave.env)
    started = time.monotonic()
    proc = subprocess.run(wave.command, cwd=root, env=env)
    elapsed = time.monotonic() - started
    print(f"[run] 波 {wave.index}/{wave.total} {wave.kind} 结束：exit={proc.returncode}，耗时 {elapsed:.1f}s")
    return proc.returncode


def cmd_run(args) -> int:
    root, limits, _scan, plan = _load_plan(args)
    print_plan(plan, limits)
    if not plan.waves:
        print("[run] 默认集为空：没有可执行的波")
        return 0

    deferred = [w for w in plan.waves if not w.fits]
    if args.channel == "full" and deferred:
        _gap_table(
            "[full] 预算不足：默认集装不下，禁止静默跳过（exit 2）",
            deferred,
            limits,
            "[full] 未执行任何波：先把预算抬到能装下最重的一波，再重跑。",
        )
        return 2
    if args.channel == "fast" and len(deferred) == len(plan.waves):
        _gap_table(
            "[fast] defer 表（按预算装不下 → 跳过；本通道 exit 0，不放宽判据、不静默执行）：",
            deferred,
            limits,
            "[fast] 提示：腾出内存、调大 YX_TEST_MEM_BUDGET，或改用 run full 让缺口直接报红。",
        )
        return 0

    for wave in plan.waves:
        if not wave.fits:
            continue
        if _execute_wave(root, wave, limits) != 0:
            print(f"[run] 波 {wave.index}/{wave.total} 失败（{wave.kind}）", file=sys.stderr)
            return 1
    if deferred:
        _gap_table(
            "[fast] defer 表（按预算装不下 → 跳过；本通道 exit 0，不放宽判据、不静默执行）：",
            deferred,
            limits,
            "[fast] 提示：腾出内存、调大 YX_TEST_MEM_BUDGET，或改用 run full 让缺口直接报红。",
        )
    print(f"[run] {args.channel} 通道完成：执行 {len(plan.waves) - len(deferred)} 波，defer {len(deferred)} 波")
    return 0


# ---------------------------------------------------------------- CLI


def resolve_root(args) -> Path:
    root = Path(args.root).resolve() if getattr(args, "root", None) else REPO_ROOT
    if not (root / "Cargo.toml").is_file():
        raise ToolError(f"{root} 不像仓库根：找不到 Cargo.toml")
    return root


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="testsuite.py",
        description="测试资源画像标记（cost:）+ 预算通道（markers / plan / run fast|full）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "环境变量：\n"
            "  YX_TEST_MEM_BUDGET      显式预算（如 8GiB）；与机器可用量取 min\n"
            "  YX_TEST_MEM_RESERVE     机器可用量里保留多少（默认 2GiB）\n"
            "  YX_TEST_DEFAULT_PEAK    未标记测试的每线程峰值估计（默认 128MiB）\n"
            "  YX_TEST_BUILD_JOB_PEAK  每个 rustc 作业的峰值估计（默认 1GiB）\n"
            "  CARGO_BUILD_JOBS        显式值会被预算收紧（工具总会按预算导出实际值）\n"
            "  RUST_TEST_THREADS       同上，作用于测试并发\n"
        ),
    )
    sub = parser.add_subparsers(dest="command", required=True)

    markers = sub.add_parser("markers", help="双向守卫 / 文法自检")
    group = markers.add_mutually_exclusive_group()
    group.add_argument("--check", action="store_true", help="默认：检查标记 <-> #[test] <-> 独占助手")
    group.add_argument("--selftest", action="store_true", help="文法与扫描器自检（正反例）")
    markers.add_argument("--no-cargo", action="store_true", help="跳过 cargo test --list 交叉核对（开发期旁路）")
    markers.add_argument("--root", default=None, help="仓库根（默认脚本所在仓库）")
    markers.set_defaults(func=cmd_markers)

    plan = sub.add_parser("plan", help="只打印计划（预算 / 每波成员与峰值 / 实际 jobs 与 threads）")
    plan.add_argument("--json", action="store_true", help="机器可读 JSON")
    plan.add_argument("--root", default=None)
    plan.set_defaults(func=cmd_plan)

    run = sub.add_parser("run", help="按预算通道执行")
    run.add_argument("channel", choices=("fast", "full"))
    run.add_argument("--root", default=None)
    run.set_defaults(func=cmd_run)
    return parser


def _force_utf8_stdio() -> None:
    """统一 UTF-8 输出：Windows 控制台默认 cp936，遇 → / — 之类会 UnicodeEncodeError 崩掉。"""
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass


def main(argv=None) -> int:
    _force_utf8_stdio()
    parser = build_parser()
    args = parser.parse_args(argv)
    if getattr(args, "selftest", False):
        return cmd_markers_selftest(args)
    try:
        return args.func(args)
    except ToolError as exc:
        print(f"[error] {exc}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())

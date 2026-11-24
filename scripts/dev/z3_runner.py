#!/usr/bin/env python3
"""cargo runner：启动产物前把 Z3 共享库目录注入动态库搜索路径。

背景：cargo 1.100 起单元产物落在 `<profile>/build/<包名>/<哈希>/out`，`deps/`
不再创建。`build.rs` 只能把共享库复制到 `<profile>`（主程序所在目录），
测试与基准二进制各自落在哈希目录，构建脚本无法预知这些路径；libz3 是动态
链接，进程启动时找不到库即失败（Windows `STATUS_DLL_NOT_FOUND`，Linux 缺少
共享对象）。

`.cargo/config.toml` 的 `[target.<cfg>] runner` 指向本脚本：cargo 运行任何
编译产物（`cargo run` / `test` / `bench`）前先经过这里，脚本把 `.z3` 资产中
含本平台共享库的目录前置进动态库搜索变量，然后原样转交退出码。

找不到 Z3 资产时原样透传：未安装 Z3 的构建本来就不链接它（`build.rs` 的
降级分支只打 warning），此时注入与否没有区别。

用法（由 cargo 调用，不面向人工）：

    python scripts/dev/z3_runner.py <产物路径> [产物参数...]
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

# 脚本位于 <仓库根>/scripts/dev/，仓库根同时是 .cargo/config.toml 的父目录
REPO_ROOT = Path(__file__).resolve().parents[2]
Z3_ROOT = REPO_ROOT / ".z3"

# 平台 -> 共享库文件名；缺省为 ELF 的 libz3.so
LIBRARY_NAMES = {
    "win32": "libz3.dll",
    "darwin": "libz3.dylib",
}
# 平台 -> 动态库搜索路径变量；缺省为 glibc 的 LD_LIBRARY_PATH
LIBRARY_PATH_VARS = {
    "win32": "PATH",
    "darwin": "DYLD_LIBRARY_PATH",
}
DEFAULT_LIBRARY_NAME = "libz3.so"
DEFAULT_LIBRARY_PATH_VAR = "LD_LIBRARY_PATH"


def library_name() -> str:
    """本平台的 Z3 共享库文件名。"""
    return LIBRARY_NAMES.get(sys.platform, DEFAULT_LIBRARY_NAME)


def library_path_var() -> str:
    """本平台加载器查找动态库所用的环境变量名。"""
    return LIBRARY_PATH_VARS.get(sys.platform, DEFAULT_LIBRARY_PATH_VAR)


def z3_library_dirs() -> list[str]:
    """返回 `.z3` 下含本平台共享库的目录（`bin/`、`lib/` 两种发行布局都认）。"""
    if not Z3_ROOT.is_dir():
        return []
    name = library_name()
    found: list[str] = []
    for subdir in ("bin", "lib"):
        for path in sorted(Z3_ROOT.glob(f"*/{subdir}/{name}")):
            directory = str(path.parent)
            if directory not in found:
                found.append(directory)
    return found


def child_env() -> dict[str, str]:
    """继承当前环境，并把 Z3 库目录前置进动态库搜索变量。"""
    env = os.environ.copy()
    dirs = z3_library_dirs()
    if dirs:
        var = library_path_var()
        existing = env.get(var, "")
        # 前置：同目录优先于系统里可能存在的其它 Z3
        env[var] = os.pathsep.join(dirs + ([existing] if existing else []))
    return env


def main(argv: list[str]) -> int:
    if not argv:
        print("用法: python scripts/dev/z3_runner.py <产物> [参数...]", file=sys.stderr)
        return 2
    try:
        result = subprocess.run(argv, env=child_env(), check=False)
    except OSError as exc:
        print(f"z3_runner: 无法启动 {argv[0]}: {exc}", file=sys.stderr)
        return 127
    return result.returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

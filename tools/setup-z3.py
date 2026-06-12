#!/usr/bin/env python3
"""下载并安装 Z3 预编译包到项目本地目录。

用法:
  python tools/setup-z3.py          # 下载到 .z3/，配置 .cargo/config.toml
  cargo build                       # 直接构建，无需设环境变量
"""

import hashlib
import os
import platform
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path
from urllib.request import urlretrieve

Z3_VERSION = "4.16.0"
PROJECT_ROOT = Path(__file__).resolve().parent.parent
Z3_DIR = PROJECT_ROOT / ".z3"

# 平台 → (tag, 扩展名)
PLATFORMS = {
    ("Windows", "x86_64"): ("x64-win", "zip"),
    ("Linux", "x86_64"): ("x64-glibc-2.35", "zip"),
    ("Darwin", "x86_64"): ("x64-osx-13.7.4", "zip"),
    ("Darwin", "arm64"): ("arm64-osx-13.7.4", "zip"),
}


def get_platform():
    system = platform.system()
    machine = platform.machine().lower()
    # 统一化：AMD64/X86_64 → x86_64, ARM64/AARCH64 → arm64
    if machine in ("amd64", "x86_64", "x64"):
        machine = "x86_64"
    elif machine in ("aarch64", "arm64"):
        machine = "arm64"
    return (system, machine)


def download(url: str, dest: Path):
    print(f"  下载 {url} ...")
    urlretrieve(url, dest)


def extract(archive: Path, dest: Path):
    print(f"  解压到 {dest} ...")
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(archive, "r") as zf:
        # Z3 zip 包内有一个顶层目录 z3-x.y.z-platform/
        # 提取时去掉这层
        root = zf.namelist()[0].split("/")[0]
        zf.extractall(dest)
        extracted = dest / root
        # 把内容移到 dest 下
        for item in extracted.iterdir():
            shutil.move(str(item), str(dest / item.name))
        extracted.rmdir()


def configure_cargo(z3_dir: Path):
    cargo_dir = PROJECT_ROOT / ".cargo"
    cargo_dir.mkdir(exist_ok=True)
    config_path = cargo_dir / "config.toml"

    header = str((z3_dir / "include" / "z3.h").resolve()).replace("\\", "/")
    bin_dir = str(z3_dir.resolve() / "bin").replace("\\", "/")

    # 不覆盖用户已有的完整 config
    if config_path.exists():
        content = config_path.read_text()
        if "Z3_SYS_Z3_HEADER" in content:
            print("  .cargo/config.toml 已有 Z3 配置，跳过")
            return

    config = f'''# Z3 路径——由 tools/setup-z3.py 自动生成
[env]
Z3_SYS_Z3_HEADER = "{header}"
'''

    config_path.write_text(config)
    print(f"  已写入 {config_path}")


def main():
    key = get_platform()
    if key not in PLATFORMS:
        print(f"不支持的平台: {key}", file=sys.stderr)
        print(f"请手动安装 Z3 并设 Z3_SYS_Z3_HEADER 环境变量", file=sys.stderr)
        sys.exit(1)

    tag, fmt = PLATFORMS[key]
    archive_name = f"z3-{Z3_VERSION}-{tag}.{fmt}"
    url = (
        f"https://github.com/Z3Prover/z3/releases/download/"
        f"z3-{Z3_VERSION}/{archive_name}"
    )

    z3_install_dir = Z3_DIR / f"z3-{Z3_VERSION}-{tag}"
    header = z3_install_dir / "include" / "z3.h"

    if header.exists():
        print(f"Z3 已安装: {z3_install_dir}")
    else:
        print(f"安装 Z3 {Z3_VERSION} ({tag}):")
        archive_path = Z3_DIR / archive_name
        Z3_DIR.mkdir(parents=True, exist_ok=True)

        if not archive_path.exists():
            download(url, archive_path)

        extract(archive_path, z3_install_dir)
        archive_path.unlink()  # 清理压缩包
        print(f"  Z3 安装完成: {z3_install_dir}")

    # 配置 .cargo/config.toml
    configure_cargo(z3_install_dir)

    # 检查 linker 能否找到 lib
    lib_dir = z3_install_dir / "lib"
    if not lib_dir.exists():
        lib_dir = z3_install_dir / "bin"  # Windows 预编译包 lib 在 bin/

    print(f"\n构建时设置 RUSTFLAGS:")
    if platform.system() == "Windows":
        print(f'  set RUSTFLAGS=-L native={lib_dir}')
    else:
        print(f'  export RUSTFLAGS="-L native={lib_dir}"')
    print(f"或直接运行: cargo build")


if __name__ == "__main__":
    main()

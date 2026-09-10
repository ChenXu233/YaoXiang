#!/bin/bash
# package-dist.sh — cargo-dist 构建后重组发行包结构（RFC-037）
# 被 dist-release.yml / nightly.yml 逐 target 调用：
#   bash scripts/release/package-dist.sh <version> <target-triple>
#
# 输入：
#   target/<triple>/dist/{yaoxiang-rs,yx}[.exe]   dist build 的构建输出（profile=dist）
#   target/<triple>/dist/libz3.*                  build.rs 链接时复制进来的共享库（单一源）
#   src/std/*.yx                                   std .yx 层真实源码
# 输出：
#   target/distrib/yaoxiang-<version>-<triple>.{tar.gz|zip} + .sha256
#
# 结构（版本目录 = 发行包解压根目录；三种安装渠道零分叉）：
#   bin/{yx, yaoxiang-rs, libz3.*}  +  lib/yaoxiang/std/  +  README/LICENSE
# 注意：cargo-dist 自己产出的扁平单二进制归档不是交付物，同名重组包直接覆盖。
set -euo pipefail

VERSION="$1"
TARGET="$2"

EXE_SUFFIX=""
case "$TARGET" in
  *windows*) EXE_SUFFIX=".exe" ;;
esac

BIN_SRC="target/$TARGET/dist"
STAGE="target/distrib/stage/yaoxiang-$VERSION-$TARGET"
OUT_BASE="target/distrib/yaoxiang-$VERSION-$TARGET"

# ── 目录骨架 ─────────────────────────────────────────────────────
rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/lib/yaoxiang/std"

# ── 前门 + 引擎 ──────────────────────────────────────────────────
cp "$BIN_SRC/yaoxiang-rs$EXE_SUFFIX" "$STAGE/bin/"
cp "$BIN_SRC/yx$EXE_SUFFIX"          "$STAGE/bin/"

# ── Z3 共享库 ────────────────────────────────────────────────────
# 单一源：build.rs 链接时已选好对应平台的共享库并复制进构建输出目录
# （copy_shared_lib → target/<triple>/<profile>/），这里直接取用——
# 打包脚本不需要知道 Z3 版本与平台目录命名。系统 Z3 路径同样经
# copy_shared_lib 落盘；缺文件说明该构建没有动态链 Z3，打包应失败。
if ! ls "$BIN_SRC"/libz3.* >/dev/null 2>&1; then
  echo "libz3 shared lib not found in $BIN_SRC (dynamic Z3 must be linked by build.rs)" >&2
  exit 1
fi
cp "$BIN_SRC"/libz3.* "$STAGE/bin/"
# Z3 许可证随包分发（MIT 分发义务）；build.rs 已从 Z3 发行包根复制落盘
if [ -f "$BIN_SRC/LICENSE-Z3.txt" ]; then
  cp "$BIN_SRC/LICENSE-Z3.txt" "$STAGE/"
fi

# macOS：dylib 的 install_name 归一到 @rpath 并对二进制 ad-hoc 重签——
# 官方 dylib 若记录构建机绝对路径，rpath 救不了；arm64 上改动后必须重签
case "$TARGET" in
  *darwin*)
    if command -v install_name_tool >/dev/null 2>&1; then
      OLD_REF="$(otool -L "$STAGE/bin/yaoxiang-rs" 2>/dev/null | awk '/libz3/ {print $1}' | head -1)"
      case "$OLD_REF" in
        @rpath/* | "") : ;;  # 已是 rpath 形态或未直接引用，无需修
        *)
          install_name_tool -id @rpath/libz3.dylib "$STAGE/bin/libz3.dylib"
          install_name_tool -change "$OLD_REF" @rpath/libz3.dylib "$STAGE/bin/yaoxiang-rs"
          ;;
      esac
      if command -v codesign >/dev/null 2>&1; then
        codesign --force --sign - "$STAGE/bin/libz3.dylib" "$STAGE/bin/yaoxiang-rs"
      fi
    fi
    ;;
esac

# ── 标准库目录 ───────────────────────────────────────────────────
# 全部为仓库静态复制，无运行时生成（RFC-037：gen-std 子命令已取消）：
# native 模块 = src/std/interfaces/ 预生成接口视图（与 StdModule::exports()
# 由 test_committed_interface_files_match_generation 测试门禁强制同步）；
# .yx 层 = src/std/*.yx 真实源码（编译权威仍是 include_str! 内嵌，RFC-036）
for f in src/std/interfaces/*.yx src/std/*.yx; do
  cp "$f" "$STAGE/lib/yaoxiang/std/"
done

cp README.md LICENSE "$STAGE/"

# ── 打包 + 重算 checksum ─────────────────────────────────────────
sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1"
  else
    shasum -a 256 "$1"
  fi
}
zip_pack() {
  # $1 = stage 目录，$2 = 输出 zip 路径。归档必须含顶层目录（解压根 = 版本目录）。
  # Git Bash 常无 zip：zip → System32 bsdtar → PowerShell 三级回退
  local stage_dir="$1" out_zip="$2"
  local base dest_dir out_zip_abs
  base="$(basename "$stage_dir")"
  out_zip_abs="$(cd "$(dirname "$out_zip")" && pwd)/$(basename "$out_zip")"
  (
    cd "$(dirname "$stage_dir")"
    if zip -q -r "$out_zip_abs" "$base" 2>/dev/null; then
      :
    elif [ -x /c/Windows/System32/tar.exe ]; then
      /c/Windows/System32/tar.exe -a -cf "$out_zip_abs" "$base"
    else
      powershell.exe -NoProfile -Command \
        "Compress-Archive -Path '$(cygpath -w "$stage_dir")' -DestinationPath '$(cygpath -w "$out_zip_abs")' -Force"
    fi
  )
}

rm -f "$OUT_BASE".tar.gz "$OUT_BASE".tar.gz.sha256 "$OUT_BASE".zip "$OUT_BASE".zip.sha256
if [ -n "$EXE_SUFFIX" ]; then
  zip_pack "$STAGE" "$OUT_BASE.zip"
  sha256 "$OUT_BASE.zip" > "$OUT_BASE.zip.sha256"
else
  ( cd target/distrib/stage && tar czf "../../yaoxiang-$VERSION-$TARGET.tar.gz" "yaoxiang-$VERSION-$TARGET" )
  sha256 "$OUT_BASE.tar.gz" > "$OUT_BASE.tar.gz.sha256"
fi
# ── 傻瓜渠道 .deb（仅 Linux CI，dpkg-deb 可用时；RFC-037 阶段四）──
if [ -z "$EXE_SUFFIX" ] && command -v dpkg-deb >/dev/null 2>&1; then
  DEBARCH="amd64"
  case "$TARGET" in aarch64*) DEBARCH="arm64" ;; esac
  bash "$(dirname "$0")/build-deb.sh" "$STAGE" "$VERSION" "$DEBARCH"
fi

rm -rf "$STAGE"

echo "packaged: $OUT_BASE.*"

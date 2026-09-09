#!/bin/bash
# package-dist.sh — cargo-dist 构建后重组发行包结构（RFC-037）
# 被 dist-release.yml / nightly.yml 逐 target 调用：
#   bash scripts/release/package-dist.sh <version> <target-triple>
#
# 输入：
#   target/<triple>/dist/{yaoxiang-rs,yx}[.exe]   dist build 的构建输出（profile=dist）
#   .z3/z3-<ver>-<tag>/{lib,bin}/libz3.*          build.rs 按需下载的 Z3（同一次构建已就位）
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
Z3_VERSION="4.16.0"

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
# 目录命名与 build.rs::detect_target() 的 Z3 发行包命名一致（两处维护，
# RFC-037 已登记此债；改动任一侧必须同步另一侧）
case "$TARGET" in
  x86_64-pc-windows-msvc)   Z3_TAG="x64-win";         Z3_LIB="libz3.dll"   ;;
  x86_64-unknown-linux-gnu) Z3_TAG="x64-glibc-2.39";  Z3_LIB="libz3.so"    ;;
  aarch64-unknown-linux-gnu) Z3_TAG="arm64-glibc-2.38"; Z3_LIB="libz3.so"   ;;
  x86_64-apple-darwin)      Z3_TAG="x64-osx-15.7.3";  Z3_LIB="libz3.dylib" ;;
  aarch64-apple-darwin)     Z3_TAG="arm64-osx-15.7.3"; Z3_LIB="libz3.dylib" ;;
  *) echo "unknown target: $TARGET" >&2; exit 1 ;;
esac
Z3_DIR=".z3/z3-$Z3_VERSION-$Z3_TAG"
Z3_SRC=""
for d in "$Z3_DIR/lib" "$Z3_DIR/bin"; do
  if [ -f "$d/$Z3_LIB" ]; then Z3_SRC="$d/$Z3_LIB"; break; fi
done
if [ -z "$Z3_SRC" ]; then
  echo "Z3 shared lib not found in $Z3_DIR (run the build first)" >&2
  exit 1
fi
cp "$Z3_SRC" "$STAGE/bin/"

# ── 标准库目录 ───────────────────────────────────────────────────
# native 模块：刚构建的引擎生成的接口视图（实现在二进制内）
"$STAGE/bin/yaoxiang-rs$EXE_SUFFIX" gen-std --out-dir "$STAGE/lib/yaoxiang/std"
# .yx 层：仓库真实源码原样复制（编译权威仍是 include_str! 内嵌，RFC-036）
for f in src/std/*.yx; do
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

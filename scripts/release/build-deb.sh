#!/bin/bash
# build-deb.sh — 从重组发行包树打 .deb（RFC-037 傻瓜渠道：系统级平装）
# 用法: build-deb.sh <解压根目录> <version> <arch>
#   <解压根目录> = package-dist.sh 的 stage 树（bin/+lib/），arch: amd64|arm64
# 布局与解压包同构：/usr/lib/yaoxiang/{bin/,lib/yaoxiang/std/} + /usr/bin/yx 符号链接
# （$ORIGIN 按真实路径解析，符号链接后仍命中同目录 libz3——RFC-037 §安装器支持）
set -euo pipefail

SRC="$1"
VERSION="$2"
DEBARCH="$3"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

[ -f "$SRC/bin/yaoxiang-rs" ] || { echo "missing $SRC/bin/yaoxiang-rs" >&2; exit 1; }

# glibc 下限对齐 build.rs::detect_target() 的 Z3 发行包命名
case "$DEBARCH" in
  amd64) GLIBC="2.39" ;;
  arm64) GLIBC="2.38" ;;
  *) echo "unsupported deb arch: $DEBARCH" >&2; exit 1 ;;
esac

ROOT="target/distrib/deb/yaoxiang"
rm -rf "$ROOT"
mkdir -p "$ROOT/DEBIAN" "$ROOT/usr/lib" "$ROOT/usr/bin"

cp -R "$SRC" "$ROOT/usr/lib/yaoxiang"
ln -s /usr/lib/yaoxiang/bin/yx "$ROOT/usr/bin/yx"

cat > "$ROOT/DEBIAN/control" <<EOF
Package: yaoxiang
Version: $VERSION
Architecture: $DEBARCH
Maintainer: ChenXu233 <Woyerpa@outlook.com>
Depends: libc6 (>= $GLIBC), libstdc++6
Section: devel
Priority: optional
Homepage: https://github.com/ChenXu233/yaoxiang
Description: YaoXiang (爻象) programming language toolchain
 Compiler, runtime, package manager, formatter and LSP for the YaoXiang
 language, with the yx front door providing built-in toolchain version
 management. The standard library ships as readable .yx sources under
 /usr/lib/yaoxiang/lib/yaoxiang/std/.
EOF

cat > "$ROOT/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
# $ORIGIN 相对 libz3 的加载不需要 ldconfig 缓存；这里只刷新符号链接缓存
ldconfig >/dev/null 2>&1 || true
EOF
chmod 755 "$ROOT/DEBIAN/postinst"

mkdir -p target/distrib
dpkg-deb --build --root-owner-group "$ROOT" "target/distrib/yaoxiang-$VERSION-$DEBARCH.deb"
rm -rf "$ROOT"
echo "built: target/distrib/yaoxiang-$VERSION-$DEBARCH.deb"

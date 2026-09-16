#!/bin/sh
# install.sh — YaoXiang 一行命令安装（RFC-037 傻瓜渠道，Linux/macOS）
#
# curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
#
# 行为：下载最新重组发行包 → 解压进 $YAOXIANG_HOME/versions/<ver>/ →
# bin/yx 就位 $YAOXIANG_HOME/bin/ → 写 settings.toml 默认版本 → PATH 提示。
# 多版本管理由 yx 前门接管（rustup 引导分解）。
set -eu

REPO="ChenXu233/YaoXiang"
YX_HOME="${YAOXIANG_HOME:-$HOME/.yaoxiang}"

# ── 平台识别 → 发行包 target ─────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS/$ARCH" in
  Linux/x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
  Linux/aarch64|Linux/arm64) TARGET="aarch64-unknown-linux-gnu" ;;
  Darwin/x86_64)  TARGET="x86_64-apple-darwin" ;;
  Darwin/arm64)   TARGET="aarch64-apple-darwin" ;;
  *) echo "yx: unsupported platform $OS/$ARCH" >&2; exit 1 ;;
esac

# ── 版本号：环境变量优先，否则取 GitHub 最新 stable ─────────────
if [ -n "${YAOXIANG_VERSION:-}" ]; then
  VERSION="${YAOXIANG_VERSION#v}"
else
  VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name": *"v\([^"]*\)".*/\1/p')
fi
[ -n "$VERSION" ] || { echo "yx: cannot determine latest version" >&2; exit 1; }

# ── 下载 + 解压 ──────────────────────────────────────────────────
ASSET="yaoxiang-$VERSION-$TARGET.tar.gz"
URL="https://github.com/$REPO/releases/download/v$VERSION/$ASSET"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "yx: downloading $URL"
curl -fL "$URL" -o "$TMP/$ASSET"

# 校验 .sha256 旁证（缺失时降级为提示，与 yx 行为一致）
if curl -fsL "$URL.sha256" -o "$TMP/$ASSET.sha256"; then
  echo "yx: verifying checksum"
  EXPECTED="$(awk '{print $1}' "$TMP/$ASSET.sha256")"
  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL="$(sha256sum "$TMP/$ASSET" | awk '{print $1}')"
  else
    ACTUAL="$(shasum -a 256 "$TMP/$ASSET" | awk '{print $1}')"
  fi
  if [ "$ACTUAL" != "$EXPECTED" ]; then
    echo "yx: checksum mismatch" >&2
    exit 1
  fi
else
  echo "yx: warning: .sha256 not available, skipping verification"
fi

mkdir -p "$TMP/out"
tar xzf "$TMP/$ASSET" -C "$TMP/out"

# ── 就位：版本目录 = 发行包解压根；bin/yx 入口 ──────────────────
mkdir -p "$YX_HOME/versions/$VERSION" "$YX_HOME/bin"
cp -R "$TMP/out/yaoxiang-$VERSION-$TARGET/." "$YX_HOME/versions/$VERSION/"
cp "$YX_HOME/versions/$VERSION/bin/yx" "$YX_HOME/bin/yx"
chmod +x "$YX_HOME/bin/yx"

# 首次安装写默认版本，不覆盖既有设置
if [ ! -f "$YX_HOME/settings.toml" ]; then
  printf 'default = "%s"\n' "$VERSION" > "$YX_HOME/settings.toml"
fi

case ":$PATH:" in
  *":$YX_HOME/bin:"*) ;;
  *) echo "yx: add to PATH:  export PATH=\"$YX_HOME/bin:\$PATH\"" ;;
esac
echo "yx: installed $VERSION -> $YX_HOME/versions/$VERSION"
echo "yx: run 'yx' to use, 'yx toolchain update' to upgrade"

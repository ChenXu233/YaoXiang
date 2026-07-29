#!/usr/bin/env bash
# docs-freshness.sh - 检查文档时效性
#
# 遍历 docs/src/**/*.md，检查每个文件是否超过 90 天未修改。
# 只报告，不阻断构建。
#
# 用法:
#   ./scripts/ci/check-docs-freshness.sh [--days=90] [--dir=docs/src]
#
# 输出:
#   退出码 0 — 报告已输出，无阻断
#   环境变量 STALE_COUNT 和 STALE_FILES 供后续步骤使用

set -euo pipefail

# ── 默认参数 ──
STALE_THRESHOLD=90
DOCS_DIR="docs/src"
VERBOSE=false

# ── 解析参数 ──
for arg in "$@"; do
  case "$arg" in
    --days=*) STALE_THRESHOLD="${arg#*=}" ;;
    --dir=*)  DOCS_DIR="${arg#*=}" ;;
    --verbose) VERBOSE=true ;;
    --help)
      echo "用法: $0 [--days=90] [--dir=docs/src] [--verbose]"
      echo "  检查超过指定天数未修改的文档 (默认 90 天)"
      exit 0
      ;;
  esac
done

# ── 确保在 git 仓库根目录执行 ──
if [ ! -d ".git" ]; then
  echo "[ERROR] 必须在 git 仓库根目录执行" >&2
  exit 1
fi

if [ ! -d "$DOCS_DIR" ]; then
  echo "[ERROR] 文档目录不存在: $DOCS_DIR" >&2
  exit 1
fi

# ── 排除模式 ──
# 与 VitePress config.js 中 srcExclude 保持一致
EXCLUDE_PATTERNS=(
  "archive/"
  "old/"
  "node_modules/"
  ".vitepress/cache/"
  ".vitepress/dist/"
)

# ── 主逻辑 ──
echo "============================================"
echo " 文档时效性检查 (Stale ≥ ${STALE_THRESHOLD} 天)"
echo " 扫描目录: ${DOCS_DIR}"
echo "============================================"
echo ""

STALE_COUNT=0
STALE_FILES=""
TOTAL_COUNT=0
CURRENT_DATE=$(date +%s)

# 构建 find 排除参数
FIND_EXCLUDE=()
for pattern in "${EXCLUDE_PATTERNS[@]}"; do
  FIND_EXCLUDE+=( -not -path "*/${pattern}*" )
done

while IFS= read -r -d '' file; do
  # 跳过 .backup.md
  if [[ "$file" == *.backup.md ]]; then
    continue
  fi

  TOTAL_COUNT=$((TOTAL_COUNT + 1))

  # 获取最后修改时间 (git log)
  last_modified=$(git log -1 --format="%ai" -- "$file" 2>/dev/null || echo "")

  if [ -z "$last_modified" ]; then
    # 文件未被 git 跟踪（可能是新文件）
    if $VERBOSE; then
      echo "  [UNTRACKED] $file"
    fi
    continue
  fi

  # 计算天数差
  last_ts=$(date -d "$last_modified" +%s 2>/dev/null || date -j -f "%Y-%m-%d %H:%M:%S %z" "$last_modified" +%s 2>/dev/null || echo "")
  if [ -z "$last_ts" ]; then
    echo "  [WARN] 无法解析日期: $last_modified ($file)" >&2
    continue
  fi

  days_old=$(( (CURRENT_DATE - last_ts) / 86400 ))

  if [ "$days_old" -ge "$STALE_THRESHOLD" ]; then
    echo "  [STALE] ${days_old}d  ${file}"
    STALE_COUNT=$((STALE_COUNT + 1))
    STALE_FILES="${STALE_FILES}${file}"$'\n'
  else
    if $VERBOSE; then
      echo "  [OK]    ${days_old}d  ${file}"
    fi
  fi
done < <(find "$DOCS_DIR" -name "*.md" -type f "${FIND_EXCLUDE[@]}" -print0)

echo ""
echo "============================================"
echo " 检查完成"
echo " 总计: ${TOTAL_COUNT} 个文件"
echo " 过时: ${STALE_COUNT} 个文件 (≥${STALE_THRESHOLD} 天未更新)"
echo " 新鲜: $((TOTAL_COUNT - STALE_COUNT)) 个文件"
echo "============================================"

# 导出给 CI 环境
if [ -n "${GITHUB_ENV:-}" ]; then
  {
    echo "STALE_COUNT=${STALE_COUNT}"
    echo "STALE_FILES<<EOF"
    echo -n "$STALE_FILES"
    echo "EOF"
  } >> "$GITHUB_ENV"
fi

# 不过时文档列表 (用于后续步骤)
if [ "$STALE_COUNT" -gt 0 ]; then
  echo ""
  echo "=== 过时文档列表 ==="
  echo "$STALE_FILES"
fi

# 永不阻断构建
exit 0
#!/bin/bash

# 增量翻译脚本
# 只翻译 git 变更的中文文档

set -e

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== VitePress 增量翻译脚本 ===${NC}"

# 检查是否安装了 vitepress-auto-translate
if ! command -v npx &> /dev/null; then
    echo -e "${RED}错误: npx 未安装，请先安装 Node.js${NC}"
    exit 1
fi

# 检查 API 密钥
if [ -z "$API_KEY" ]; then
    echo -e "${YELLOW}警告: API_KEY 环境变量未设置${NC}"
    echo "请设置 API_KEY 环境变量，例如："
    echo "  export API_KEY=your_api_key"
    echo ""
    echo "或者使用 .env 文件："
    echo "  echo 'API_KEY=your_api_key' > .env"
    exit 1
fi

# 获取变更的文件
echo -e "${GREEN}正在检测变更的文件...${NC}"

# 获取暂存区的变更文件
CHANGED_FILES=$(git diff --cached --name-only -- 'docs/src/**/*.md' ':!docs/src/en/**' 2>/dev/null || true)

# 如果暂存区没有变更，获取工作区的变更
if [ -z "$CHANGED_FILES" ]; then
    CHANGED_FILES=$(git diff --name-only -- 'docs/src/**/*.md' ':!docs/src/en/**' 2>/dev/null || true)
fi

# 如果还是没有变更，获取最近一次提交的变更
if [ -z "$CHANGED_FILES" ]; then
    CHANGED_FILES=$(git diff --name-only HEAD~1 HEAD -- 'docs/src/**/*.md' ':!docs/src/en/**' 2>/dev/null || true)
fi

if [ -z "$CHANGED_FILES" ]; then
    echo -e "${YELLOW}没有检测到变更的中文文档${NC}"
    exit 0
fi

echo -e "${GREEN}检测到以下变更的文件:${NC}"
echo "$CHANGED_FILES"
echo ""

# 创建临时目录
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# 复制变更的文件到临时目录
echo -e "${GREEN}正在准备翻译文件...${NC}"
for file in $CHANGED_FILES; do
    if [ -f "$file" ]; then
        mkdir -p "$TEMP_DIR/$(dirname "$file")"
        cp "$file" "$TEMP_DIR/$file"
        echo "  - $file"
    fi
done

echo ""

# 翻译文件
echo -e "${GREEN}正在翻译文件...${NC}"
cd "$TEMP_DIR"
npx vitepress-auto-translate -s docs/src -l en

# 将翻译结果复制回原目录
echo -e "${GREEN}正在复制翻译结果...${NC}"
cd - > /dev/null
cp -r "$TEMP_DIR/docs/src/en/"* docs/src/en/ 2>/dev/null || true

echo ""
echo -e "${GREEN}翻译完成！${NC}"
echo ""
echo "翻译结果已保存到 docs/src/en/ 目录"
echo ""
echo "下一步："
echo "  1. 检查翻译结果"
echo "  2. git add docs/src/en/"
echo "  3. git commit -m 'docs: translate documentation'"

# 自动翻译

本项目使用 [vitepress-auto-translate](https://github.com/xing-junyang/vitepress-auto-translate) 自动翻译文档。

## 工作原理

### GitHub Actions（自动）

当 `docs/src/` 下的中文文档发生变更时，GitHub Actions 会自动：
1. 检测变更的文件
2. 使用 LLM 翻译成英文
3. 将翻译结果提交到 `docs/src/en/` 目录

### 本地手动翻译

#### 增量翻译（推荐）

只翻译 git 变更的文件：

```bash
# 设置 API 密钥
export API_KEY=your_api_key

# 运行增量翻译脚本
./scripts/translate-incremental.sh
```

#### 全量翻译

翻译整个目录：

```bash
# 设置 API 密钥
export API_KEY=your_api_key

# 安装依赖
npm install vitepress-auto-translate

# 翻译到英文
npx vitepress-auto-translate -s docs/src -l en
```

## 配置

### GitHub Secrets

在 GitHub 仓库的 Settings > Secrets and variables > Actions 中添加：

- `LLM_API_KEY` - LLM API 密钥

### 自定义模型

如果需要使用自定义模型（如本地 Ollama），可以修改 `.github/workflows/auto-translate.yml`：

```yaml
- name: Translate changed documents
  if: steps.changed-files.outputs.files != ''
  env:
    API_KEY: ${{ secrets.LLM_API_KEY }}
  run: |
    # 创建临时目录存放待翻译文件
    mkdir -p /tmp/to-translate

    # 复制变更的文件到临时目录
    for file in ${{ steps.changed-files.outputs.files }}; do
      if [ -f "$file" ]; then
        mkdir -p "/tmp/to-translate/$(dirname "$file")"
        cp "$file" "/tmp/to-translate/$file"
      fi
    done

    # 翻译临时目录中的文件（使用自定义模型）
    npx vitepress-auto-translate -s /tmp/to-translate/docs/src -l en -m custom -b http://your-model-url/v1

    # 将翻译结果复制回原目录
    cp -r /tmp/to-translate/docs/src/en/* docs/src/en/ 2>/dev/null || true
```

## 支持的 LLM 提供商

- **SiliconFlow**（默认）- 使用 deepseek-ai/DeepSeek-V3
- **OpenAI** - 使用 gpt-3.5-turbo
- **自定义** - 任何兼容的 API

## 支持的语言

en, es, fr, de, it, pt, nl, pl, ru, ja, ko, zh, tzh, ar, tr

## 注意事项

- 代码块和数学公式不会被翻译
- Markdown 格式会被保留
- 翻译质量取决于所选 LLM
- 建议人工审核关键文档的翻译结果
- 增量翻译只翻译 git 变更的文件，节省 API 调用

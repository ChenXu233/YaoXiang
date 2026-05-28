#!/usr/bin/env node

/**
 * 自动翻译脚本
 *
 * 使用大语言模型（LLM）将中文文档翻译成英文
 *
 * 支持的 LLM 提供商：
 *   - OpenAI（GPT-4, GPT-3.5-turbo）
 *   - Anthropic（Claude）
 *   - 本地模型（Ollama, LM Studio, vLLM 等）
 *   - 其他兼容 OpenAI API 的提供商
 *
 * 用法：
 *   node scripts/translate.js --files "file1.md file2.md file3.md"
 *
 * 环境变量：
 *   LLM_PROVIDER - LLM 提供商（openai | anthropic | ollama | local | custom）
 *   LLM_API_KEY - API 密钥（本地模型可留空）
 *   LLM_MODEL - 模型名称
 *   LLM_BASE_URL - API 基础 URL
 */

const fs = require('fs');
const path = require('path');

// 翻译提示词
const TRANSLATION_PROMPT = `You are a professional technical documentation translator. Your task is to translate Chinese (Simplified) documentation into English.

Rules:
1. Translate the text accurately while maintaining the original meaning
2. Keep all code blocks, inline code, and technical terms unchanged
3. Preserve the markdown formatting
4. Use natural, fluent English that reads well
5. Keep proper nouns and brand names as-is
6. Translate technical terms accurately, using standard terminology
7. Maintain the same tone and style as the original

Please translate the following Chinese text to English:`;

// 提供商配置
const PROVIDER_CONFIG = {
  openai: {
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4o-mini',
  },
  anthropic: {
    baseUrl: 'https://api.anthropic.com/v1',
    model: 'claude-3-haiku-20240307',
  },
  ollama: {
    baseUrl: 'http://localhost:11434/v1',
    model: 'llama3',
  },
  local: {
    baseUrl: 'http://localhost:1234/v1',
    model: 'default',
  },
  deepseek: {
    baseUrl: 'https://api.deepseek.com/v1',
    model: 'deepseek-chat',
  },
  moonshot: {
    baseUrl: 'https://api.moonshot.cn/v1',
    model: 'moonshot-v1-8k',
  },
  zhipu: {
    baseUrl: 'https://open.bigmodel.cn/api/paas/v4',
    model: 'glm-4-flash',
  },
  qwen: {
    baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    model: 'qwen-turbo',
  },
};

/**
 * 使用 OpenAI 兼容 API 进行翻译
 */
async function translateWithOpenAICompatible(text, apiKey, model, baseUrl) {
  const headers = {
    'Content-Type': 'application/json',
  };

  // 本地模型可能不需要 API 密钥
  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`;
  }

  const response = await fetch(`${baseUrl}/chat/completions`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      model,
      messages: [
        { role: 'system', content: TRANSLATION_PROMPT },
        { role: 'user', content: text },
      ],
      temperature: 0.3,
    }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`API error: ${response.status} - ${error}`);
  }

  const data = await response.json();
  return data.choices[0].message.content;
}

/**
 * 使用 Anthropic API 进行翻译
 */
async function translateWithAnthropic(text, apiKey, model) {
  const response = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': apiKey,
      'anthropic-version': '2023-06-01',
    },
    body: JSON.stringify({
      model,
      max_tokens: 4096,
      messages: [
        { role: 'user', content: `${TRANSLATION_PROMPT}\n\n${text}` },
      ],
    }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Anthropic API error: ${response.status} - ${error}`);
  }

  const data = await response.json();
  return data.content[0].text;
}

/**
 * 根据提供商选择翻译函数
 */
async function translateText(text, provider, apiKey, model, baseUrl) {
  // 获取提供商配置
  const config = PROVIDER_CONFIG[provider] || {};
  const finalModel = model || config.model;
  const finalBaseUrl = baseUrl || config.baseUrl;

  switch (provider) {
    case 'anthropic':
      return translateWithAnthropic(text, apiKey, finalModel);
    case 'openai':
    case 'ollama':
    case 'local':
    case 'deepseek':
    case 'moonshot':
    case 'zhipu':
    case 'qwen':
    case 'custom':
      return translateWithOpenAICompatible(text, apiKey, finalModel, finalBaseUrl);
    default:
      throw new Error(`Unsupported LLM provider: ${provider}`);
  }
}

/**
 * 翻译单个文件
 */
async function translateFile(filePath, provider, apiKey, model, baseUrl) {
  const content = fs.readFileSync(filePath, 'utf-8');

  // 跳过空文件
  if (!content.trim()) {
    console.log(`Skipping empty file: ${filePath}`);
    return null;
  }

  // 提取 frontmatter
  const frontmatterMatch = content.match(/^(---\n[\s\S]*?\n---\n)/);
  let frontmatter = '';
  let body = content;

  if (frontmatterMatch) {
    frontmatter = frontmatterMatch[1];
    body = content.slice(frontmatter.length);
  }

  // 提取代码块
  const codeBlocks = [];
  const bodyWithoutCode = body.replace(/```[\s\S]*?```/g, (match) => {
    codeBlocks.push(match);
    return `__CODE_BLOCK_${codeBlocks.length - 1}__`;
  });

  // 提取行内代码
  const inlineCodes = [];
  const bodyWithoutInlineCode = bodyWithoutCode.replace(/`[^`]+`/g, (match) => {
    inlineCodes.push(match);
    return `__INLINE_CODE_${inlineCodes.length - 1}__`;
  });

  // 翻译文本
  let translatedText;
  try {
    translatedText = await translateText(
      bodyWithoutInlineCode,
      provider,
      apiKey,
      model,
      baseUrl
    );
  } catch (error) {
    console.error(`Translation error for ${filePath}:`, error.message);
    return null;
  }

  // 恢复行内代码
  let result = translatedText;
  inlineCodes.forEach((code, index) => {
    result = result.replace(`__INLINE_CODE_${index}__`, code);
  });

  // 恢复代码块
  codeBlocks.forEach((block, index) => {
    result = result.replace(`__CODE_BLOCK_${index}__`, block);
  });

  // 更新 frontmatter 中的语言标签
  let translatedFrontmatter = frontmatter;
  if (frontmatter) {
    translatedFrontmatter = frontmatter.replace(/lang:\s*zh/, 'lang: en');
  }

  return translatedFrontmatter + result;
}

/**
 * 显示帮助信息
 */
function showHelp() {
  console.log(`
自动翻译脚本 - 使用 LLM 将中文文档翻译成英文

用法：
  node scripts/translate.js --files "file1.md file2.md file3.md"
  node scripts/translate.js --help

环境变量：
  LLM_PROVIDER    LLM 提供商 (默认: openai)
                  可选: openai, anthropic, ollama, local, deepseek, moonshot, zhipu, qwen, custom
  LLM_API_KEY     API 密钥 (本地模型可留空)
  LLM_MODEL       模型名称 (可选，有默认值)
  LLM_BASE_URL    API 基础 URL (可选，有默认值)

示例：
  # 使用 OpenAI
  LLM_PROVIDER=openai LLM_API_KEY=sk-xxx node scripts/translate.js --files "docs/src/tutorial/getting-started.md"

  # 使用本地 Ollama 模型
  LLM_PROVIDER=ollama node scripts/translate.js --files "docs/src/tutorial/getting-started.md"

  # 使用 DeepSeek
  LLM_PROVIDER=deepseek LLM_API_KEY=sk-xxx node scripts/translate.js --files "docs/src/tutorial/getting-started.md"

  # 使用自定义模型
  LLM_PROVIDER=custom LLM_BASE_URL=http://localhost:8080/v1 LLM_MODEL=my-model node scripts/translate.js --files "docs/src/tutorial/getting-started.md"

支持的提供商：
  openai     - OpenAI GPT 系列 (默认: gpt-4o-mini)
  anthropic  - Anthropic Claude 系列 (默认: claude-3-haiku-20240307)
  ollama     - 本地 Ollama 模型 (默认: llama3, 地址: http://localhost:11434/v1)
  local      - 本地 LM Studio 等 (默认: default, 地址: http://localhost:1234/v1)
  deepseek   - DeepSeek 模型 (默认: deepseek-chat)
  moonshot   - Moonshot 模型 (默认: moonshot-v1-8k)
  zhipu      - 智谱 GLM 模型 (默认: glm-4-flash)
  qwen       - 通义千问模型 (默认: qwen-turbo)
  custom     - 自定义 OpenAI 兼容 API
`);
}

/**
 * 主函数
 */
async function main() {
  // 解析命令行参数
  const args = process.argv.slice(2);
  let files = [];

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--help' || args[i] === '-h') {
      showHelp();
      process.exit(0);
    }
    if (args[i] === '--files' && args[i + 1]) {
      files = args[i + 1].split(/\s+/).filter(Boolean);
      break;
    }
  }

  if (files.length === 0) {
    console.log('No files to translate');
    console.log('Use --help for usage information');
    process.exit(0);
  }

  // 检查环境变量
  const provider = process.env.LLM_PROVIDER || 'openai';
  const apiKey = process.env.LLM_API_KEY;
  const model = process.env.LLM_MODEL;
  const baseUrl = process.env.LLM_BASE_URL;

  // 本地模型不需要 API 密钥
  const needsApiKey = !['ollama', 'local'].includes(provider);
  if (needsApiKey && !apiKey) {
    console.error('LLM_API_KEY environment variable is not set');
    console.error('Use --help for usage information');
    process.exit(1);
  }

  console.log(`Using LLM provider: ${provider}`);
  console.log(`Translating ${files.length} files...`);

  // 翻译每个文件
  for (const file of files) {
    // 只处理 docs/src/ 下的文件
    if (!file.startsWith('docs/src/') || file.startsWith('docs/src/en/')) {
      continue;
    }

    const sourcePath = file;
    const targetPath = file.replace('docs/src/', 'docs/src/en/');
    const targetDir = path.dirname(targetPath);

    // 检查源文件是否存在
    if (!fs.existsSync(sourcePath)) {
      console.log(`Source file not found: ${sourcePath}`);
      continue;
    }

    // 创建目标目录
    if (!fs.existsSync(targetDir)) {
      fs.mkdirSync(targetDir, { recursive: true });
    }

    // 翻译文件
    console.log(`Translating: ${sourcePath}`);
    const translated = await translateFile(sourcePath, provider, apiKey, model, baseUrl);

    if (translated) {
      fs.writeFileSync(targetPath, translated, 'utf-8');
      console.log(`  -> ${targetPath}`);
    }
  }

  console.log('Translation complete!');
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

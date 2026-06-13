#!/usr/bin/env node
// scripts/i18n/translate.mjs

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { loadConfig, loadGlossary } from './config.mjs';
import { loadCache, saveCache, getKeysToTranslate, updateCache } from './cache.mjs';
import { translateBatch } from './ai.mjs';
import { resolveLanguagePrompt } from './prompt.mjs';
import { getAdapter } from './adapters/index.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, '../..');

// 加载 .env 文件（如果存在）
function loadDotEnv() {
  const envPath = path.join(__dirname, '.env');
  if (fs.existsSync(envPath)) {
    const content = fs.readFileSync(envPath, 'utf-8');
    for (const line of content.split('\n')) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const [key, ...valueParts] = trimmed.split('=');
      const value = valueParts.join('=').trim();
      if (key && !process.env[key.trim()]) {
        process.env[key.trim()] = value;
      }
    }
  }
}

loadDotEnv();

// 解析命令行参数
const args = process.argv.slice(2);
const isFullTranslate = args.includes('--full');

/**
 * 翻译单个系统
 */
async function translateSystem(systemName, systemConfig, adapter, config, glossary, env) {
  console.log(`\n📦 Processing ${systemName}...`);

  // 读取源 JSON
  const sourceFullPath = path.join(ROOT, systemConfig.sourcePath);
  if (!fs.existsSync(sourceFullPath)) {
    console.log(`  ⚠️  Source file not found: ${systemConfig.sourcePath}`);
    return;
  }
  const sourceJson = JSON.parse(fs.readFileSync(sourceFullPath, 'utf-8'));
  const sourceKeys = adapter.extractKeys(sourceJson);
  const sourceKeyCount = Object.keys(sourceKeys).length;
  console.log(`  📄 Source: ${sourceKeyCount} keys`);

  // 加载 cache
  const cacheFullPath = path.join(ROOT, systemConfig.cachePath);
  let cache = isFullTranslate ? {} : loadCache(cacheFullPath);

  // 获取语言配置
  const targetLangs = Object.keys(config.languages);

  for (const lang of targetLangs) {
    console.log(`\n  🌐 Translating to ${lang} (${config.languages[lang].name})...`);

    // 获取需要翻译的 key
    const keysToTranslate = isFullTranslate
      ? Object.keys(sourceKeys)
      : getKeysToTranslate(sourceKeys, cache, lang);

    if (keysToTranslate.length === 0) {
      console.log(`    ✅ No keys to translate (up to date)`);
      continue;
    }

    console.log(`    📝 ${keysToTranslate.length} keys to translate`);

    // 读取目标 JSON
    const targetPath = path.join(ROOT, systemConfig.targetDir, `${lang}.json`);
    let targetJson = {};
    if (fs.existsSync(targetPath)) {
      targetJson = JSON.parse(fs.readFileSync(targetPath, 'utf-8'));
    }

    // 按 batchSize 分组翻译
    const batchSize = config.batchSize || 20;

    for (let i = 0; i < keysToTranslate.length; i += batchSize) {
      const batch = keysToTranslate.slice(i, i + batchSize);
      const batchKeys = {};
      for (const key of batch) {
        batchKeys[key] = sourceKeys[key];
      }

      console.log(`    🔄 Translating batch ${Math.floor(i / batchSize) + 1}/${Math.ceil(keysToTranslate.length / batchSize)}...`);

      try {
        const languagePrompt = resolveLanguagePrompt(lang, config);
        const translations = await translateBatch({
          keys: batchKeys,
          sourceLang: config.languages[config.source]?.name || config.source,
          targetLang: config.languages[lang]?.name || lang,
          languagePrompt,
          glossary,
          apiKey: env.AI_API_KEY,
          baseUrl: env.AI_BASE_URL,
          model: env.AI_MODEL || config.model
        });

        // 合并翻译结果
        targetJson = adapter.applyTranslations(targetJson, translations);

        // 立即写入文件（崩溃恢复）
        fs.writeFileSync(targetPath, JSON.stringify(targetJson, null, 2) + '\n', 'utf-8');

        // 立即更新 cache
        cache = updateCache(cache, sourceKeys, lang, batch);
        saveCache(cacheFullPath, cache);

        console.log(`    ✅ Batch translated successfully`);
      } catch (error) {
        console.error(`    ❌ Batch translation failed: ${error.message}`);
      }
    }

    console.log(`    💾 Saved ${targetPath}`);
  }

  console.log(`  💾 Cache saved`);
}

/**
 * 主函数
 */
async function main() {
  console.log('🚀 i18n Auto Translation System');
  console.log(`   Mode: ${isFullTranslate ? 'Full translate' : 'Incremental'}`);

  // 加载配置
  const config = loadConfig(ROOT);
  const glossary = loadGlossary(ROOT);

  // 读取环境变量
  const env = {
    AI_API_KEY: process.env.AI_API_KEY,
    AI_BASE_URL: process.env.AI_BASE_URL || 'http://101.132.38.216:3000/v1',
    AI_MODEL: process.env.AI_MODEL
  };

  if (!env.AI_API_KEY) {
    console.error('❌ AI_API_KEY environment variable is required');
    process.exit(1);
  }

  // 翻译每个系统
  for (const [systemName, systemConfig] of Object.entries(config.systems)) {
    const adapter = getAdapter(systemConfig.adapter);
    await translateSystem(systemName, systemConfig, adapter, config, glossary, env);
  }

  console.log('\n✅ Translation complete!');
}

main().catch(error => {
  console.error('❌ Fatal error:', error);
  process.exit(1);
});

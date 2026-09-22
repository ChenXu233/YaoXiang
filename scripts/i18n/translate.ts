#!/usr/bin/env node
// scripts/i18n/translate.ts

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { loadConfig, loadGlossary } from './config.ts';
import type { Glossary, I18nConfig, SystemConfig, TranslateEnv } from './config.ts';
import { loadCache, saveCache, getKeysToTranslate, updateCache } from './cache.ts';
import type { TranslationCache } from './cache.ts';
import { translateBatch } from './ai.ts';
import { resolveLanguagePrompt } from './prompt.ts';
import { getAdapter } from './adapters/index.ts';
import type { TranslateAdapter } from './adapters/index.ts';
import type { FlatKeys, LocaleJson } from './adapters/locales.ts';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, '../..');

/** 从 `.env` 读入缺失的环境变量（不覆盖已存在的进程环境） */
function loadDotEnv(): void {
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

/** `error.message` 的 `unknown` 安全读取 */
function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** 读取 JSON 文件；不存在返回空对象 */
function readJsonIfExists(filePath: string): LocaleJson {
  if (!fs.existsSync(filePath)) return {};
  return JSON.parse(fs.readFileSync(filePath, 'utf-8')) as LocaleJson;
}

/** 按 batchSize 把待翻键切成批次 */
function chunk(keys: string[], size: number): string[][] {
  const out: string[][] = [];
  for (let i = 0; i < keys.length; i += size) {
    out.push(keys.slice(i, i + size));
  }
  return out;
}

/**
 * 翻译单个系统
 */
async function translateSystem(
  systemName: string,
  systemConfig: SystemConfig,
  adapter: TranslateAdapter,
  config: I18nConfig,
  glossary: Glossary,
  env: TranslateEnv,
): Promise<void> {
  console.log(`\n📦 Processing ${systemName}...`);

  // 读取源 JSON
  const sourceFullPath = path.join(ROOT, systemConfig.sourcePath);
  if (!fs.existsSync(sourceFullPath)) {
    console.log(`  ⚠️  Source file not found: ${systemConfig.sourcePath}`);
    return;
  }
  const sourceJson = JSON.parse(fs.readFileSync(sourceFullPath, 'utf-8')) as LocaleJson;
  const sourceKeys = adapter.extractKeys(sourceJson);
  const sourceKeyCount = Object.keys(sourceKeys).length;
  console.log(`  📄 Source: ${sourceKeyCount} keys`);

  // 加载 cache
  const cacheFullPath = path.join(ROOT, systemConfig.cachePath);
  let cache: TranslationCache = isFullTranslate ? {} : loadCache(cacheFullPath);

  // 获取语言配置
  const targetLangs = Object.keys(config.languages);

  for (const lang of targetLangs) {
    const langConfig = config.languages[lang];
    if (!langConfig) continue;
    console.log(`\n  🌐 Translating to ${lang} (${langConfig.name})...`);

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
    let targetJson: LocaleJson = readJsonIfExists(targetPath);

    // 按 batchSize 分组翻译
    const batchSize = config.batchSize || 20;
    const batches = chunk(keysToTranslate, batchSize);

    for (const [index, batch] of batches.entries()) {
      const batchKeys: FlatKeys = {};
      for (const key of batch) {
        const source = sourceKeys[key];
        if (source !== undefined) {
          batchKeys[key] = source;
        }
      }

      console.log(`    🔄 Translating batch ${index + 1}/${batches.length}...`);

      try {
        const languagePrompt = resolveLanguagePrompt(lang, config);
        const translations = await translateBatch({
          keys: batchKeys,
          sourceLang: config.languages[config.source]?.name || config.source,
          targetLang: langConfig.name || lang,
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
        console.error(`    ❌ Batch translation failed: ${errorMessage(error)}`);
      }
    }

    console.log(`    💾 Saved ${targetPath}`);
  }

  console.log(`  💾 Cache saved`);
}

/**
 * 主函数
 */
async function main(): Promise<void> {
  console.log('🚀 i18n Auto Translation System');
  console.log(`   Mode: ${isFullTranslate ? 'Full translate' : 'Incremental'}`);

  // 加载配置
  const config = loadConfig(ROOT);
  const glossary = loadGlossary(ROOT);

  // 读取环境变量
  const env: TranslateEnv = {
    AI_API_KEY: process.env.AI_API_KEY ?? '',
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

main().catch((error: unknown) => {
  console.error('❌ Fatal error:', error);
  process.exit(1);
});

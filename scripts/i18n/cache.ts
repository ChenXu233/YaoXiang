import fs from 'fs';
import crypto from 'crypto';
import type { FlatKeys } from './adapters/locales.ts';

/** 翻译缓存：`<key>:<lang>` → 源文案的 MD5 */
export type TranslationCache = Record<string, string>;

/**
 * 计算字符串的 MD5 hash
 * @param content - 要计算 hash 的内容
 * @returns MD5 hash（十六进制）
 */
export function computeHash(content: string): string {
  return crypto.createHash('md5').update(content, 'utf-8').digest('hex');
}

/**
 * 加载 cache 文件
 * @param cachePath - cache 文件路径
 * @returns cache 对象（缺失或损坏时返回空表，视为「全部待翻译」）
 */
export function loadCache(cachePath: string): TranslationCache {
  try {
    const content = fs.readFileSync(cachePath, 'utf-8');
    return JSON.parse(content) as TranslationCache;
  } catch {
    return {};
  }
}

/**
 * 保存 cache 文件
 * @param cachePath - cache 文件路径
 * @param cache - cache 对象
 */
export function saveCache(cachePath: string, cache: TranslationCache): void {
  fs.writeFileSync(cachePath, JSON.stringify(cache, null, 2) + '\n', 'utf-8');
}

/**
 * 获取需要翻译的 key 列表
 * @param sourceKeys - 源 key-value 映射
 * @param cache - cache 对象
 * @param lang - 目标语言代码
 * @returns 需要翻译的 key 列表（源文案 hash 变了的）
 */
export function getKeysToTranslate(
  sourceKeys: FlatKeys,
  cache: TranslationCache,
  lang: string,
): string[] {
  const result: string[] = [];
  for (const [key, value] of Object.entries(sourceKeys)) {
    const cacheKey = `${key}:${lang}`;
    const currentHash = computeHash(value);
    if (cache[cacheKey] !== currentHash) {
      result.push(key);
    }
  }
  return result;
}

/**
 * 更新 cache 中的 hash
 * @param cache - cache 对象
 * @param sourceKeys - 源 key-value 映射
 * @param lang - 目标语言代码
 * @param translatedKeys - 已翻译的 key 列表
 * @returns 更新后的 cache
 */
export function updateCache(
  cache: TranslationCache,
  sourceKeys: FlatKeys,
  lang: string,
  translatedKeys: string[],
): TranslationCache {
  const result: TranslationCache = { ...cache };
  for (const key of translatedKeys) {
    const cacheKey = `${key}:${lang}`;
    const source = sourceKeys[key];
    if (source !== undefined) {
      result[cacheKey] = computeHash(source);
    }
  }
  return result;
}

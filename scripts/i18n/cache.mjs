import fs from 'fs';
import crypto from 'crypto';

/**
 * 计算字符串的 MD5 hash
 * @param {string} content - 要计算 hash 的内容
 * @returns {string} MD5 hash
 */
export function computeHash(content) {
  return crypto.createHash('md5').update(content, 'utf-8').digest('hex');
}

/**
 * 加载 cache 文件
 * @param {string} cachePath - cache 文件路径
 * @returns {object} cache 对象
 */
export function loadCache(cachePath) {
  try {
    const content = fs.readFileSync(cachePath, 'utf-8');
    return JSON.parse(content);
  } catch {
    return {};
  }
}

/**
 * 保存 cache 文件
 * @param {string} cachePath - cache 文件路径
 * @param {object} cache - cache 对象
 */
export function saveCache(cachePath, cache) {
  fs.writeFileSync(cachePath, JSON.stringify(cache, null, 2) + '\n', 'utf-8');
}

/**
 * 获取需要翻译的 key 列表
 * @param {Object<string, string>} sourceKeys - 源 key-value 映射
 * @param {object} cache - cache 对象
 * @param {string} lang - 目标语言代码
 * @returns {string[]} 需要翻译的 key 列表
 */
export function getKeysToTranslate(sourceKeys, cache, lang) {
  const result = [];
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
 * @param {object} cache - cache 对象
 * @param {Object<string, string>} sourceKeys - 源 key-value 映射
 * @param {string} lang - 目标语言代码
 * @param {string[]} translatedKeys - 已翻译的 key 列表
 * @returns {object} 更新后的 cache
 */
export function updateCache(cache, sourceKeys, lang, translatedKeys) {
  const result = { ...cache };
  for (const key of translatedKeys) {
    const cacheKey = `${key}:${lang}`;
    result[cacheKey] = computeHash(sourceKeys[key]);
  }
  return result;
}

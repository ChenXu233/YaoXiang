/**
 * 从扁平 JSON 中提取 key-value 对
 * @param {object} json - 源 JSON（可能包含 _meta）
 * @returns {Object<string, string>} key → value 映射
 */
export function extractKeys(json) {
  const result = {};
  for (const [key, value] of Object.entries(json)) {
    if (key !== '_meta' && typeof value === 'string') {
      result[key] = value;
    }
  }
  return result;
}

/**
 * 将翻译结果写回目标 JSON
 * @param {object} targetJson - 目标 JSON
 * @param {Object<string, string>} translations - key → 翻译值
 * @returns {object} 更新后的 JSON
 */
export function applyTranslations(targetJson, translations) {
  const result = { ...targetJson };
  for (const [key, value] of Object.entries(translations)) {
    result[key] = value;
  }
  return result;
}

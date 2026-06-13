/**
 * 从嵌套 JSON 中提取 key-value 对
 * @param {object} json - 源 JSON（可能包含 _meta）
 * @returns {Object<string, string>} key.subkey → value 映射
 */
export function extractKeys(json) {
  const result = {};
  for (const [code, info] of Object.entries(json)) {
    if (code === '_meta') continue;
    if (typeof info === 'object' && info !== null) {
      for (const [field, value] of Object.entries(info)) {
        if (typeof value === 'string') {
          result[`${code}.${field}`] = value;
        }
      }
    }
  }
  return result;
}

/**
 * 将翻译结果写回嵌套 JSON
 * @param {object} targetJson - 目标 JSON
 * @param {Object<string, string>} translations - key.subkey → 翻译值
 * @returns {object} 更新后的 JSON
 */
export function applyTranslations(targetJson, translations) {
  const result = JSON.parse(JSON.stringify(targetJson)); // deep clone
  for (const [compoundKey, value] of Object.entries(translations)) {
    const [code, field] = compoundKey.split('.');
    if (!result[code]) result[code] = {};
    result[code][field] = value;
  }
  return result;
}

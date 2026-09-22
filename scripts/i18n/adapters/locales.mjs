/**
 * 错误码键形状（`E0001` / `W1004`）。判定用形状而非「含点即嵌套」：
 * 万一将来引入普通带点键，不会被误当成错误码拍进对象。
 */
const CODE_KEY = /^[EW]\d{4}$/;

/**
 * 从 locale JSON 中提取 key-value 对（扁平字符串 + 嵌套错误码对象，如 E0001.title）
 * @param {object} json - 源 JSON（可能包含 _meta）
 * @returns {Object<string, string>} key → value 映射
 */
export function extractKeys(json) {
  const result = {};
  for (const [key, value] of Object.entries(json)) {
    if (key === '_meta') continue;
    if (typeof value === 'string') {
      result[key] = value;
    } else if (typeof value === 'object' && value !== null) {
      for (const [field, v] of Object.entries(value)) {
        if (typeof v === 'string') {
          result[`${key}.${field}`] = v;
        }
      }
    }
  }
  return result;
}

/**
 * 将翻译结果写回目标 JSON（点分 key 还原为嵌套结构）
 * @param {object} targetJson - 目标 JSON
 * @param {Object<string, string>} translations - key → 翻译值
 * @returns {object} 更新后的 JSON
 */
export function applyTranslations(targetJson, translations) {
  const result = { ...targetJson };
  for (const [key, value] of Object.entries(translations)) {
    const dot = key.indexOf('.');
    if (dot > 0 && CODE_KEY.test(key.slice(0, dot))) {
      // E0001.title → result.E0001.title（错误码对象字段）
      const head = key.slice(0, dot);
      const parent = result[head];
      result[head] = {
        ...(typeof parent === 'object' && parent !== null ? parent : {}),
        [key.slice(dot + 1)]: value
      };
      continue;
    }
    result[key] = value;
  }
  return result;
}

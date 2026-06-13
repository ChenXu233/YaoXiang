const DEFAULT_LANGUAGE_PROMPT = '请进行专业准确的技术文档翻译。';

/**
 * 解析语言特定的 prompt
 * @param {string} lang - 目标语言代码
 * @param {object} config - i18n.config.json 配置对象
 * @returns {string} 语言特定的 prompt
 */
export function resolveLanguagePrompt(lang, config) {
  const target = config.targets?.[lang];
  if (target?.prompt) {
    return target.prompt;
  }
  return DEFAULT_LANGUAGE_PROMPT;
}

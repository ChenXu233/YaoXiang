import type { LanguageConfig } from './config.ts';

const DEFAULT_LANGUAGE_PROMPT = '请进行专业准确的技术文档翻译。';

/** 含 `languages` 表的最小配置形状（只取本函数所需字段） */
interface PromptConfig {
  languages?: Record<string, LanguageConfig>;
}

/**
 * 解析语言特定的 prompt
 *
 * 读 `config.languages[lang].prompt`——`i18n.config.json` 里语言配置挂在
 * `languages` 下（`config.ts` 的校验门也只认该键）。曾误读 `config.targets`，
 * 导致 zh-x-miao/zh-classical 的风格指令长期未生效（该键从来不存在，
 * 旧测试又恰好 mock 了它，把这个错误锁进了绿灯）。
 *
 * @param lang - 目标语言代码
 * @param config - i18n.config.json 配置对象
 * @returns 语言特定的 prompt；该语言未定义风格指令时返回默认 prompt
 */
export function resolveLanguagePrompt(lang: string, config: PromptConfig): string {
  const target = config.languages?.[lang];
  if (target?.prompt) {
    return target.prompt;
  }
  return DEFAULT_LANGUAGE_PROMPT;
}

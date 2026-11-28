import type { LanguageConfig } from './config.ts';

// 稳定性条款（#421）：既有译文是既定产物，重译只发生在源文案语义变化时——
// 风格层面（大小写、措辞习惯、句式）保持克制，不做无谓改写
export const DEFAULT_LANGUAGE_PROMPT =
  '请进行专业准确的技术文档翻译。目标语言的既有译文是既定产物：' +
  '除非源文案语义发生变化，保持既有译法与风格（大小写、措辞习惯），不做无谓改写。';

/** 含 `languages` 表的最小配置形状（只取本函数所需字段） */
interface PromptConfig {
  languages?: Record<string, LanguageConfig>;
}

/**
 * 字段级规则：与语言风格无关，任何目标语言都必须遵守。
 *
 * 错误码条目的 `title` 是**诊断标题**——标签式短名，在终端里独占一行，
 * 用户按它检索。此前没有这条规则时，新增的码由模型自行决定大小写，于是同一张
 * 表里出现 `Invalid character` 与 `unterminated string` 两种写法。
 */
export const FIELD_RULES = [
  '字段级规则（对所有目标语言生效）：',
  '- 错误码条目的 title 是诊断标题（标签式短名）：有大小写的语言统一句首大写、其余小写（如 `Invalid character`），不要小写开头；',
  '- title 不带句末标点，保持简短（不超过 6 个词）。',
].join('\n');

/**
 * 把语言风格指令与字段级规则合成最终提示词。
 *
 * 分两层是刻意的：`resolveLanguagePrompt` 只管「译成什么风格」（喵语 / 文言文），
 * 字段级规则管「某些字段必须长什么样」，与语言无关，因此两者都要在。
 *
 * @param languagePrompt 语言风格指令
 * @returns 送进模型的完整提示词
 */
export function composePrompt(languagePrompt: string): string {
  return `${languagePrompt}\n\n${FIELD_RULES}`;
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

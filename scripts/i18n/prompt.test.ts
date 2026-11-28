import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolveLanguagePrompt, DEFAULT_LANGUAGE_PROMPT, composePrompt, FIELD_RULES } from './prompt.ts';
import type { I18nConfig } from './config.ts';

describe('resolveLanguagePrompt', () => {
  it('should return default prompt for standard languages', () => {
    const result = resolveLanguagePrompt('en', { languages: { en: { name: 'English' } } });
    expect(result).toBe(DEFAULT_LANGUAGE_PROMPT);
  });

  it('should return custom prompt from config', () => {
    const config = {
      languages: {
        'zh-x-miao': { name: '喵语', prompt: '翻译成猫娘风格' },
      },
    };
    const result = resolveLanguagePrompt('zh-x-miao', config);
    expect(result).toBe('翻译成猫娘风格');
  });

  it('should return default prompt if no custom prompt in config', () => {
    const config = { languages: { en: { name: 'English' } } };
    const result = resolveLanguagePrompt('en', config);
    expect(result).toBe(DEFAULT_LANGUAGE_PROMPT);
  });

  // 回归：本函数的键名必须与**仓库里真实的** i18n.config.json 对齐。
  // 曾经的失败模式是单测自造 `targets` 键（仓库里根本不存在）而绿，
  // 同时线上因读不到风格指令而静默退回默认 prompt。这里直接读真实配置，
  // 保证「定义在 languages 下的 prompt 必须真的被取到」。
  it('should read the style prompt from the real i18n.config.json', () => {
    const configPath = fileURLToPath(new URL('../../i18n.config.json', import.meta.url));
    const config = JSON.parse(readFileSync(configPath, 'utf-8')) as I18nConfig;

    // 先确认前提：真实配置里确实定义了风格 prompt
    const styled = Object.entries(config.languages).filter(([, v]) => v.prompt);
    expect(styled.length).toBeGreaterThan(0);

    for (const [lang, langConfig] of styled) {
      expect(resolveLanguagePrompt(lang, config)).toBe(langConfig.prompt);
    }

    // 未定义风格指令的语言应拿默认 prompt
    expect(resolveLanguagePrompt('en', config)).toBe(DEFAULT_LANGUAGE_PROMPT);
  });
});

describe('composePrompt', () => {
  it('should keep the language style prompt and append the field rules', () => {
    const result = composePrompt(DEFAULT_LANGUAGE_PROMPT);
    expect(result).toContain(DEFAULT_LANGUAGE_PROMPT);
    expect(result).toContain(FIELD_RULES);
  });

  it('should append the field rules for every configured language', () => {
    // 回归：字段级规则必须对每种语言都在——喵语/文言文有各自的风格指令，
    // 若规则只挂在默认 prompt 上，这两种语言的 title 仍会大小写混用。
    const configPath = fileURLToPath(new URL('../../i18n.config.json', import.meta.url));
    const config = JSON.parse(readFileSync(configPath, 'utf-8')) as I18nConfig;
    for (const lang of Object.keys(config.languages)) {
      const composed = composePrompt(resolveLanguagePrompt(lang, config));
      expect(composed).toContain(FIELD_RULES);
      expect(composed).toContain('title');
    }
  });

  it('should state the title casing rule explicitly', () => {
    // title 的句首大写规则必须写在提示词里——否则新增码的大小写仍由模型自行决定。
    expect(FIELD_RULES).toContain('句首大写');
    expect(FIELD_RULES).toContain('title');
  });
});

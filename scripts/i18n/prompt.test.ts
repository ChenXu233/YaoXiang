import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolveLanguagePrompt } from './prompt.ts';
import type { I18nConfig } from './config.ts';

describe('resolveLanguagePrompt', () => {
  it('should return default prompt for standard languages', () => {
    const result = resolveLanguagePrompt('en', { languages: { en: { name: 'English' } } });
    expect(result).toBe('请进行专业准确的技术文档翻译。');
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
    expect(result).toBe('请进行专业准确的技术文档翻译。');
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
    expect(resolveLanguagePrompt('en', config)).toBe('请进行专业准确的技术文档翻译。');
  });
});

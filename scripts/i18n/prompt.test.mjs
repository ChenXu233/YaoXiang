import { describe, it, expect } from 'vitest';
import { resolveLanguagePrompt } from './prompt.mjs';

describe('resolveLanguagePrompt', () => {
  it('should return default prompt for standard languages', () => {
    const result = resolveLanguagePrompt('en', {});
    expect(result).toBe('请进行专业准确的技术文档翻译。');
  });

  it('should return custom prompt from config', () => {
    const config = {
      targets: {
        'zh-x-miao': { prompt: '翻译成猫娘风格' }
      }
    };
    const result = resolveLanguagePrompt('zh-x-miao', config);
    expect(result).toBe('翻译成猫娘风格');
  });

  it('should return default prompt if no custom prompt in config', () => {
    const config = {
      targets: {
        en: { name: 'English' }
      }
    };
    const result = resolveLanguagePrompt('en', config);
    expect(result).toBe('请进行专业准确的技术文档翻译。');
  });
});

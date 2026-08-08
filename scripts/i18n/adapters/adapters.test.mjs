import { describe, it, expect } from 'vitest';
import * as locales from './locales.mjs';

describe('locales adapter', () => {
  const sampleJson = {
    _meta: { lang: 'zh', name: '中文' },
    cmd_received: '收到命令',
    run_file: '运行文件',
    reading_file: '正在读取：{0}'
  };

  describe('extractKeys', () => {
    it('should extract flat key-value pairs', () => {
      const result = locales.extractKeys(sampleJson);
      expect(result).toEqual({
        cmd_received: '收到命令',
        run_file: '运行文件',
        reading_file: '正在读取：{0}'
      });
    });

    it('should skip _meta field', () => {
      const result = locales.extractKeys(sampleJson);
      expect(result).not.toHaveProperty('_meta');
    });
  });

  describe('applyTranslations', () => {
    it('should apply translations to target json', () => {
      const target = { _meta: { lang: 'en' }, cmd_received: 'old' };
      const translations = { cmd_received: 'Command received', run_file: 'Run file' };
      const result = locales.applyTranslations(target, translations);
      expect(result.cmd_received).toBe('Command received');
      expect(result.run_file).toBe('Run file');
    });

    it('should preserve _meta and other fields', () => {
      const target = { _meta: { lang: 'en' }, existing: 'keep' };
      const translations = { new_key: 'new value' };
      const result = locales.applyTranslations(target, translations);
      expect(result._meta).toEqual({ lang: 'en' });
      expect(result.existing).toBe('keep');
    });
  });
});

describe('locales adapter with nested error codes', () => {
  const sampleJson = {
    _meta: { lang: 'zh' },
    E0001: {
      title: '无效字符',
      template: "无效字符：'{char}'",
      help: '删除非法字符'
    }
  };

  describe('extractKeys', () => {
    it('should extract nested key-value pairs with compound keys', () => {
      const result = locales.extractKeys(sampleJson);
      expect(result).toEqual({
        'E0001.title': '无效字符',
        'E0001.template': "无效字符：'{char}'",
        'E0001.help': '删除非法字符'
      });
    });

    it('should skip _meta field', () => {
      const result = locales.extractKeys(sampleJson);
      expect(Object.keys(result).every(k => !k.startsWith('_meta'))).toBe(true);
    });
  });

  describe('applyTranslations', () => {
    it('should apply translations to nested structure', () => {
      const target = { E0001: { title: 'old' } };
      const translations = { 'E0001.title': 'Invalid character', 'E0001.help': 'Remove it' };
      const result = locales.applyTranslations(target, translations);
      expect(result.E0001.title).toBe('Invalid character');
      expect(result.E0001.help).toBe('Remove it');
    });
  });
});

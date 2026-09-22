import { describe, it, expect } from 'vitest';
import * as locales from './locales.ts';
import type { LocaleJson } from './locales.ts';

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
      const target: LocaleJson = { E0001: { title: 'old' } };
      const translations = { 'E0001.title': 'Invalid character', 'E0001.help': 'Remove it' };
      const result = locales.applyTranslations(target, translations);
      expect(locales.codeFields(result, 'E0001')).toEqual({
        title: 'Invalid character',
        help: 'Remove it'
      });
    });

    // 回归：新增错误码在目标语言里**还没有父对象**。旧守卫额外要求父对象
    // 已存在，不满足就走 `result[key] = value` 兜底，落成扁平键
    // `"E9999.title"`；而运行时 src/util/i18n/mod.rs 只从对象值取条目，
    // 扁平键被静默丢弃——翻译存在于文件里，用户却看到英文兜底。
    it('should nest fields for a code the target has never translated', () => {
      const target: LocaleJson = { _meta: { lang: 'ja' }, cmd_received: '既存' };
      const result = locales.applyTranslations(target, { 'E9999.title': '新コード' });
      expect(result.E9999).toEqual({ title: '新コード' });
      // 注意用 Object.hasOwn 而非 toHaveProperty：后者按点分路径取值，
      // 会把 result.E9999.title 当作命中，验不出扁平键残留
      expect(Object.hasOwn(result, 'E9999.title')).toBe(false);
    });

    // 目标已有该码的父对象时，两个字段都要并入同一对象（不能互相覆盖）
    it('should merge sibling fields into one object', () => {
      const target: LocaleJson = { E9999: { title: '旧' } };
      const result = locales.applyTranslations(target, {
        'E9999.title': '新',
        'E9999.help': '助'
      });
      expect(result.E9999).toEqual({ title: '新', help: '助' });
    });

    // 普通（非错误码）带点键必须原样保留，不能被误嵌套
    it('should keep non-code dotted keys flat', () => {
      const result = locales.applyTranslations({}, { 'some.other.key': 'v' });
      expect(Object.hasOwn(result, 'some.other.key')).toBe(true);
      expect(result).not.toHaveProperty('some');
    });
  });
});

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { loadCache, saveCache, getKeysToTranslate, updateCache } from './cache.ts';
import * as localesAdapter from './adapters/locales.ts';

describe('E2E: Translation workflow', () => {
  const tmpDir = path.join(os.tmpdir(), 'i18n-e2e-test');

  beforeEach(() => {
    fs.mkdirSync(tmpDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('should handle incremental translation for locales', () => {
    // 模拟源文件
    const source = {
      cmd_received: '收到命令',
      run_file: '运行文件'
    };

    // 第一次翻译：所有 key 都需要翻译
    const cachePath = path.join(tmpDir, '.i18n-cache.json');
    let cache = loadCache(cachePath);
    const { translate: keysToTranslate } = getKeysToTranslate(source, cache, 'en');
    expect(keysToTranslate).toEqual(['cmd_received', 'run_file']);

    // 模拟翻译结果
    const translations = {
      cmd_received: 'Command received',
      run_file: 'Run file'
    };

    // 更新目标文件
    let target: localesAdapter.LocaleJson = { _meta: { lang: 'en' } };
    target = localesAdapter.applyTranslations(target, translations);

    // 更新 cache
    cache = updateCache(cache, source, 'en', keysToTranslate);
    saveCache(cachePath, cache);

    // 第二次翻译：没有 key 需要翻译
    cache = loadCache(cachePath);
    const { translate: keysToTranslate2 } = getKeysToTranslate(source, cache, 'en');
    expect(keysToTranslate2).toEqual([]);

    // 修改源文件
    const source2 = {
      cmd_received: '收到命令（修改）',
      run_file: '运行文件'
    };

    // 第三次翻译：只有修改的 key 需要翻译
    const { translate: keysToTranslate3 } = getKeysToTranslate(source2, cache, 'en');
    expect(keysToTranslate3).toEqual(['cmd_received']);
  });

  // #421：缓存冷启动 + 目标已有译文 → 收养进缓存而非重译，产物零改写
  it('should adopt existing translations on cold cache', () => {
    const source = {
      cmd_received: '收到命令',
      run_file: '运行文件'
    };

    // 目标文件已有上一轮产物；缓存丢失（冷启动）
    const target: localesAdapter.LocaleJson = {
      _meta: { lang: 'en' },
      cmd_received: 'Command received',
      run_file: 'Run file'
    };
    const cache = loadCache(path.join(os.tmpdir(), 'nonexistent-i18n-cache'));

    const { translate, adopt } = getKeysToTranslate(
      source,
      cache,
      'en',
      localesAdapter.extractKeys(target)
    );
    expect(adopt).toEqual(['cmd_received', 'run_file']);
    expect(translate).toEqual([]);

    // 收养 = 只登记 hash，译文不动
    cache['cmd_received:en'] = 'adopted';
    expect(target.cmd_received).toBe('Command received');
  });

  it('should handle incremental translation for diagnostic', () => {
    const source = {
      E0001: {
        title: '无效字符',
        template: "无效字符：'{char}'"
      }
    };

    const cachePath = path.join(tmpDir, '.i18n-cache.json');
    let cache = loadCache(cachePath);

    // 提取 keys
    const sourceKeys = localesAdapter.extractKeys(source);
    expect(sourceKeys).toEqual({
      'E0001.title': '无效字符',
      'E0001.template': "无效字符：'{char}'"
    });

    // 检查需要翻译的 key
    const { translate: keysToTranslate } = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(keysToTranslate).toEqual(['E0001.title', 'E0001.template']);

    // 模拟翻译
    const translations = {
      'E0001.title': 'Invalid character',
      'E0001.template': "Invalid character: '{char}'"
    };

    let target: localesAdapter.LocaleJson = {};
    target = localesAdapter.applyTranslations(target, translations);
    expect(localesAdapter.codeFields(target, 'E0001')).toEqual({
      title: 'Invalid character',
      template: "Invalid character: '{char}'"
    });

    // 更新 cache
    cache = updateCache(cache, sourceKeys, 'en', keysToTranslate);
    saveCache(cachePath, cache);

    // 再次检查
    cache = loadCache(cachePath);
    const { translate: keysToTranslate2 } = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(keysToTranslate2).toEqual([]);
  });
});

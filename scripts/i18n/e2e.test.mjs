import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { loadCache, saveCache, getKeysToTranslate, updateCache, computeHash } from './cache.mjs';
import * as localesAdapter from './adapters/locales.mjs';
import * as diagnosticAdapter from './adapters/diagnostic.mjs';

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
    const keysToTranslate = getKeysToTranslate(source, cache, 'en');
    expect(keysToTranslate).toEqual(['cmd_received', 'run_file']);

    // 模拟翻译结果
    const translations = {
      cmd_received: 'Command received',
      run_file: 'Run file'
    };

    // 更新目标文件
    let target = { _meta: { lang: 'en' } };
    target = localesAdapter.applyTranslations(target, translations);

    // 更新 cache
    cache = updateCache(cache, source, 'en', keysToTranslate);
    saveCache(cachePath, cache);

    // 第二次翻译：没有 key 需要翻译
    cache = loadCache(cachePath);
    const keysToTranslate2 = getKeysToTranslate(source, cache, 'en');
    expect(keysToTranslate2).toEqual([]);

    // 修改源文件
    const source2 = {
      cmd_received: '收到命令（修改）',
      run_file: '运行文件'
    };

    // 第三次翻译：只有修改的 key 需要翻译
    const keysToTranslate3 = getKeysToTranslate(source2, cache, 'en');
    expect(keysToTranslate3).toEqual(['cmd_received']);
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
    const sourceKeys = diagnosticAdapter.extractKeys(source);
    expect(sourceKeys).toEqual({
      'E0001.title': '无效字符',
      'E0001.template': "无效字符：'{char}'"
    });

    // 检查需要翻译的 key
    const keysToTranslate = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(keysToTranslate).toEqual(['E0001.title', 'E0001.template']);

    // 模拟翻译
    const translations = {
      'E0001.title': 'Invalid character',
      'E0001.template': "Invalid character: '{char}'"
    };

    let target = {};
    target = diagnosticAdapter.applyTranslations(target, translations);
    expect(target.E0001.title).toBe('Invalid character');
    expect(target.E0001.template).toBe("Invalid character: '{char}'");

    // 更新 cache
    cache = updateCache(cache, sourceKeys, 'en', keysToTranslate);
    saveCache(cachePath, cache);

    // 再次检查
    cache = loadCache(cachePath);
    const keysToTranslate2 = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(keysToTranslate2).toEqual([]);
  });
});

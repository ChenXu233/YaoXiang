import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { loadCache, saveCache, getKeysToTranslate, computeHash } from './cache.ts';
import fs from 'fs';
import path from 'path';
import os from 'os';

describe('computeHash', () => {
  it('should compute MD5 hash of string', () => {
    const hash = computeHash('hello world');
    expect(hash).toMatch(/^[a-f0-9]{32}$/);
  });

  it('should return same hash for same input', () => {
    expect(computeHash('test')).toBe(computeHash('test'));
  });

  it('should return different hash for different input', () => {
    expect(computeHash('test1')).not.toBe(computeHash('test2'));
  });
});

describe('loadCache / saveCache', () => {
  const tmpDir = path.join(os.tmpdir(), 'i18n-cache-test');
  const cachePath = path.join(tmpDir, '.i18n-cache.json');

  beforeEach(() => {
    fs.mkdirSync(tmpDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('should return empty object if cache file does not exist', () => {
    const cache = loadCache(cachePath);
    expect(cache).toEqual({});
  });

  it('should load existing cache file', () => {
    fs.writeFileSync(cachePath, JSON.stringify({ 'key:en': 'abc123' }));
    const cache = loadCache(cachePath);
    expect(cache).toEqual({ 'key:en': 'abc123' });
  });

  it('should save cache file', () => {
    const cache = { 'key:en': 'abc123' };
    saveCache(cachePath, cache);
    const loaded = JSON.parse(fs.readFileSync(cachePath, 'utf-8'));
    expect(loaded).toEqual(cache);
  });
});

describe('getKeysToTranslate', () => {
  it('should return keys not in cache', () => {
    const sourceKeys = { a: 'value1', b: 'value2' };
    const cache = {};
    const result = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(result.translate).toEqual(['a', 'b']);
    expect(result.adopt).toEqual([]);
  });

  it('should skip keys with matching hash', () => {
    const sourceKeys = { a: 'value1' };
    const hash = computeHash('value1');
    const cache = { 'a:en': hash };
    const result = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(result.translate).toEqual([]);
    expect(result.adopt).toEqual([]);
  });

  it('should return keys with different hash', () => {
    const sourceKeys = { a: 'new value' };
    const cache = { 'a:en': 'old_hash' };
    const result = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(result.translate).toEqual(['a']);
    expect(result.adopt).toEqual([]);
  });

  // #421：缓存冷启动（CI 缓存丢失/首次接入）时，既有译文必须收养进缓存
  // 而非重译——「无记录」曾被视为「全部待翻」，一轮重写全部产物
  //（9a1ccdbd 415 处、9bb4d257c 五语言全量）
  it('should adopt existing translations on cache miss', () => {
    const sourceKeys = { a: 'value1', b: 'value2' };
    const cache = {};
    const existingTarget = { a: 'Value 1 (existing product)' };
    const result = getKeysToTranslate(sourceKeys, cache, 'en', existingTarget);
    expect(result.translate).toEqual(['b']);
    expect(result.adopt).toEqual(['a']);
  });

  // 缓存有记录且 hash 已变：即使目标已有译文，也要跟随源重译
  it('should retranslate changed source even if target exists', () => {
    const sourceKeys = { a: 'new value' };
    const cache = { 'a:en': 'old_hash' };
    const existingTarget = { a: 'Old translation' };
    const result = getKeysToTranslate(sourceKeys, cache, 'en', existingTarget);
    expect(result.translate).toEqual(['a']);
    expect(result.adopt).toEqual([]);
  });

  // 空串译文视同缺失，照常翻译
  it('should treat empty target value as missing', () => {
    const sourceKeys = { a: 'value1' };
    const cache = {};
    const existingTarget = { a: '' };
    const result = getKeysToTranslate(sourceKeys, cache, 'en', existingTarget);
    expect(result.translate).toEqual(['a']);
    expect(result.adopt).toEqual([]);
  });
});

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { loadCache, saveCache, getKeysToTranslate, computeHash } from './cache.mjs';
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
    expect(result).toEqual(['a', 'b']);
  });

  it('should skip keys with matching hash', () => {
    const sourceKeys = { a: 'value1' };
    const hash = computeHash('value1');
    const cache = { 'a:en': hash };
    const result = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(result).toEqual([]);
  });

  it('should return keys with different hash', () => {
    const sourceKeys = { a: 'new value' };
    const cache = { 'a:en': 'old_hash' };
    const result = getKeysToTranslate(sourceKeys, cache, 'en');
    expect(result).toEqual(['a']);
  });
});

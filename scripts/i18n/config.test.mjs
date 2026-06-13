import { describe, it, expect } from 'vitest';
import { loadConfig, loadGlossary } from './config.mjs';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, '../..');

describe('loadConfig', () => {
  it('should load i18n.config.json', () => {
    const config = loadConfig(ROOT);
    expect(config).toHaveProperty('source', 'zh');
    expect(config).toHaveProperty('targets');
    expect(config.targets).toHaveProperty('en');
    expect(config.targets).toHaveProperty('zh-x-miao');
    expect(config).toHaveProperty('batchSize', 20);
  });

  it('should throw if config file not found', () => {
    expect(() => loadConfig('/nonexistent')).toThrow();
  });
});

describe('loadGlossary', () => {
  it('should load i18n-glossary.json', () => {
    const glossary = loadGlossary(ROOT);
    expect(glossary).toHaveProperty('trait', 'trait');
    expect(glossary).toHaveProperty('type inference', '类型推导');
  });

  it('should return empty object if glossary not found', () => {
    const glossary = loadGlossary('/nonexistent');
    expect(glossary).toEqual({});
  });
});

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
    expect(config).toHaveProperty('systems');
    expect(config).toHaveProperty('languages');
    expect(config).toHaveProperty('batchSize', 20);
  });

  it('should throw if config file not found', () => {
    expect(() => loadConfig('/nonexistent')).toThrow();
  });
});

describe('loadConfig with new structure', () => {
  it('should have systems definition', () => {
    const config = loadConfig(ROOT);
    expect(config).toHaveProperty('systems');
    expect(config.systems).toHaveProperty('locales');
    expect(config.systems).toHaveProperty('diagnostic');
    expect(config.systems.locales).toHaveProperty('adapter', 'flat');
    expect(config.systems.diagnostic).toHaveProperty('adapter', 'nested');
  });

  it('should have languages definition', () => {
    const config = loadConfig(ROOT);
    expect(config).toHaveProperty('languages');
    expect(config.languages).toHaveProperty('en');
    expect(config.languages).toHaveProperty('zh-x-miao');
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

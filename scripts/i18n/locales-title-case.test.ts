import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { normalizeCodeTitles } from './adapters/locales.ts';
import { loadConfig } from './config.ts';
import type { LocaleJson } from './adapters/locales.ts';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const CODE_KEY = /^[EW]\d{4}$/;

function loadLocale(lang: string): LocaleJson {
  return JSON.parse(readFileSync(`${ROOT}locales/${lang}.json`, 'utf-8')) as LocaleJson;
}

describe('错误码 title 的大小写不变量', () => {
  // 不变量本体：对声明了 capitalizeTitles 的语言（字母表语言），
  // 每个错误码 title 都句首大写——诊断标题在终端里独占一行、用户按它检索。
  it('en/ru 的每个错误码 title 都句首大写', () => {
    const config = loadConfig(ROOT);
    const langs = Object.entries(config.languages)
      .filter(([, cfg]) => cfg.capitalizeTitles)
      .map(([lang]) => lang);

    expect(langs.length).toBeGreaterThan(0);

    for (const lang of langs) {
      const json = loadLocale(lang);
      const offenders: string[] = [];
      for (const [code, entry] of Object.entries(json)) {
        if (!CODE_KEY.test(code) || typeof entry !== 'object' || entry === null) continue;
        const title = (entry as Record<string, unknown>).title;
        if (typeof title !== 'string' || title === '') continue;
        const first = title[0]!;
        const isLowerLetter = first.toLowerCase() === first && first.toUpperCase() !== first;
        if (isLowerLetter) offenders.push(`${code}=${title}`);
      }
      expect(offenders, `${lang} 有小写开头的错误码 title`).toEqual([]);
    }
  });

  it('normalizeCodeTitles 只改首字母，其余原样', () => {
    const input = {
      E0001: { title: 'invalid character', template: 'Invalid character: {c}', help: 'x' },
    } as unknown as LocaleJson;
    const out = normalizeCodeTitles(input) as unknown as Record<string, Record<string, string>>;
    expect(out.E0001!.title).toBe('Invalid character');
    expect(out.E0001!.template).toBe('Invalid character: {c}');
    expect(out.E0001!.help).toBe('x');
  });

  it('非错误码键与无 title 的条目不动', () => {
    const input = {
      cmd_received: '收到运行命令',
      E0002: { template: 'no title here' },
      _meta: { version: 1 },
    } as unknown as LocaleJson;
    const out = normalizeCodeTitles(input) as unknown as Record<string, unknown>;
    expect(out.cmd_received).toBe('收到运行命令');
    expect(out._meta).toEqual({ version: 1 });
    expect((out.E0002 as Record<string, unknown>).template).toBe('no title here');
  });

  it('已经是大写开头的条目原样返回（幂等）', () => {
    const input = { E0001: { title: 'Invalid character' } } as unknown as LocaleJson;
    expect(normalizeCodeTitles(input)).toEqual(input);
  });
});

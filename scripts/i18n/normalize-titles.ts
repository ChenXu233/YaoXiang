#!/usr/bin/env node
// scripts/i18n/normalize-titles.ts

/**
 * 把目标语言 locale 文件里**错误码 title** 的首字母统一成大写。
 *
 * 用途：修正存量译文。翻译管线每次翻译后会自动做同样的规范化
 * （见 translate.ts 的 capitalizeTitles 分支），但已经入库的文件不会自动回溯，
 * 本脚本用于一次性对齐——诊断标题在终端里独占一行、用户按它检索，大小写必须一致。
 *
 * 只处理 i18n.config.json 里声明了 `capitalizeTitles` 的语言（有大小写的语言）。
 *
 * 用法：node scripts/i18n/normalize-titles.ts [--lang en,ru]
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { loadConfig } from './config.ts';
import { normalizeCodeTitles } from './adapters/locales.ts';
import type { LocaleJson } from './adapters/locales.ts';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

function readJson(filePath: string): LocaleJson {
  return JSON.parse(fs.readFileSync(filePath, 'utf-8')) as LocaleJson;
}

function main(): void {
  const config = loadConfig(ROOT);
  const arg = process.argv.slice(2).find((a) => a.startsWith('--lang='));
  const only = arg ? arg.slice('--lang='.length).split(',').map((s) => s.trim()) : null;

  const languages = Object.entries(config.languages).filter(
    ([lang, cfg]) => cfg.capitalizeTitles && (!only || only.includes(lang))
  );

  if (languages.length === 0) {
    console.log('没有需要规范化的语言（i18n.config.json 里没有 capitalizeTitles 的语言）');
    return;
  }

  for (const [lang] of languages) {
    const filePath = path.join(ROOT, 'locales', `${lang}.json`);
    if (!fs.existsSync(filePath)) {
      console.log(`  ⚠️  跳过 ${lang}：locales/${lang}.json 不存在`);
      continue;
    }
    const before = readJson(filePath);
    const after = normalizeCodeTitles(before);
    const changed = Object.keys(after).filter(
      (code) =>
        typeof before[code] === 'object' &&
        before[code] !== null &&
        (before[code] as Record<string, unknown>).title !==
          (after[code] as Record<string, unknown>).title
    );
    if (changed.length === 0) {
      console.log(`  ✅ ${lang}：已是句首大写，无需改动（${Object.keys(after).length} 个条目）`);
      continue;
    }
    fs.writeFileSync(filePath, JSON.stringify(after, null, 2) + '\n', 'utf-8');
    console.log(`  ✏️  ${lang}：规范化了 ${changed.length} 个 title（${changed.join(', ')}）`);
  }
}

main();

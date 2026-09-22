// scripts/i18n/migrate-flat-keys.ts
//
// 一次性数据迁移：把 locale JSON 里的扁平错误码键折叠回嵌套对象。
//
// 背景：scripts/i18n/adapters/locales.ts 的写回逻辑曾要求「目标语言已有该码的
// 父对象」才嵌套，否则兜底成扁平键 `"E1103.title"`。而运行时
// src/util/i18n/mod.rs 只从 JSON **对象值**取错误码条目，扁平键被静默丢弃——
// 译文存在于文件里，用户却看到英文兜底（如 `YAOXIANG_LANG=ja` 跑 E1103）。
//
// bot 是 hash 增量缓存（scripts/i18n/cache.ts），键没变不会重翻，
// 所以存量扁平键不会自愈，必须迁移一次。
//
// 冲突处理：若某码同时有嵌套与扁平两份，以**嵌套为准**（来自更晚的权威注册表
// 机制 #320），扁平作废。
//
// 用法：node scripts/i18n/migrate-flat-keys.ts [--dry-run]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { LocaleEntry, LocaleJson } from './adapters/locales.ts';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const LOCALES = path.join(ROOT, 'locales');
const CODE_KEY = /^[EW]\d{4}$/;
const DRY_RUN = process.argv.includes('--dry-run');

/** `foldFlatCodeKeys` 的结果：新对象 + 改动清单 */
export interface FoldResult {
  json: LocaleJson;
  /** 被折进嵌套对象的扁平键 */
  folded: string[];
  /** 因字段已存在嵌套版而被作废的扁平键 */
  dropped: string[];
}

/** 把顶层扁平码键折叠进嵌套对象；返回新对象与改动清单 */
export function foldFlatCodeKeys(json: LocaleJson): FoldResult {
  const result: LocaleJson = {};
  const folded: string[] = [];
  const dropped: string[] = [];

  for (const [key, value] of Object.entries(json)) {
    const dot = key.indexOf('.');
    if (dot > 0 && CODE_KEY.test(key.slice(0, dot)) && typeof value === 'string') {
      const head = key.slice(0, dot);
      const field = key.slice(dot + 1);
      const existing: LocaleEntry | undefined = result[head];
      const existingObj = typeof existing === 'object' && existing !== null ? existing : undefined;
      // 该码已有嵌套条目且该字段已存在 → 扁平的那份是旧产物，作废
      if (existingObj !== undefined && Object.hasOwn(existingObj, field)) {
        dropped.push(key);
        continue;
      }
      result[head] = { ...(existingObj ?? {}), [field]: value };
      folded.push(key);
      continue;
    }
    result[key] = value;
  }

  return { json: result, folded, dropped };
}

function main(): void {
  let totalFolded = 0;
  let totalDropped = 0;

  for (const file of fs.readdirSync(LOCALES).sort()) {
    if (!file.endsWith('.json')) continue;
    const p = path.join(LOCALES, file);
    const raw = fs.readFileSync(p, 'utf-8');
    const { json, folded, dropped } = foldFlatCodeKeys(JSON.parse(raw) as LocaleJson);

    if (folded.length === 0) {
      console.log(`${file}: 无需迁移`);
      continue;
    }

    const out = JSON.stringify(json, null, 2) + '\n';
    if (!DRY_RUN) fs.writeFileSync(p, out, 'utf-8');

    const codes = [...new Set(folded.map((k) => k.slice(0, k.indexOf('.'))))].sort();
    console.log(
      `${file}: 折叠 ${folded.length} 个扁平键 → ${codes.length} 个码` +
        `${dropped.length ? `（作废 ${dropped.length} 个重复字段）` : ''}` +
        `${DRY_RUN ? ' [dry-run]' : ''}`
    );
    console.log(`  码: ${codes.join(' ')}`);
    totalFolded += folded.length;
    totalDropped += dropped.length;
  }

  console.log(
    `\n合计：折叠 ${totalFolded} 个，作废 ${totalDropped} 个` +
      `${DRY_RUN ? '（未写盘）' : ''}`
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();

// scripts/i18n/adapters/index.ts

import * as flat from './locales.ts';
import type { FlatKeys, LocaleJson } from './locales.ts';

/**
 * 翻译管线适配器：把「某系统的 locale 文件」抽象成
 * 「抽出待翻键 → 回写译文」两步。
 *
 * 这是 `translate.ts` 与具体 locale 格式之间的唯一边界——加新格式
 * （如嵌套更深的错误码树）只需实现本接口，翻译管线不动。
 */
export interface TranslateAdapter {
  /** 抽出待翻译的键（点分形式，如 `E0001.title`） */
  extractKeys(json: LocaleJson): FlatKeys;
  /** 把译文写回目标 JSON */
  applyTranslations(targetJson: LocaleJson, translations: FlatKeys): LocaleJson;
}

const ADAPTERS = {
  flat
} satisfies Record<string, TranslateAdapter>;

/** 已注册的适配器类型（`i18n.config.json` 的 `adapter` 字段取值） */
export type AdapterType = keyof typeof ADAPTERS;

/**
 * 根据类型获取适配器
 * @param type - 适配器类型（见 `AdapterType`）
 * @returns 适配器对象
 */
export function getAdapter(type: string): TranslateAdapter {
  const adapter = (ADAPTERS as Record<string, TranslateAdapter | undefined>)[type];
  if (!adapter) {
    throw new Error(
      `Unknown adapter type: ${type}. Available: ${Object.keys(ADAPTERS).join(', ')}`,
    );
  }
  return adapter;
}

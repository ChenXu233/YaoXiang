// scripts/i18n/adapters/index.mjs

import * as flat from './locales.mjs';
import * as nested from './diagnostic.mjs';

const ADAPTERS = {
  flat,
  nested
};

/**
 * 根据类型获取适配器
 * @param {string} type - 适配器类型（'flat' 或 'nested'）
 * @returns {object} 适配器对象
 */
export function getAdapter(type) {
  const adapter = ADAPTERS[type];
  if (!adapter) {
    throw new Error(`Unknown adapter type: ${type}. Available: ${Object.keys(ADAPTERS).join(', ')}`);
  }
  return adapter;
}

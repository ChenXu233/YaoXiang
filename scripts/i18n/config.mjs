import fs from 'fs';
import path from 'path';

/**
 * 加载 i18n.config.json
 * @param {string} root - 项目根目录
 * @returns {object} 配置对象
 */
export function loadConfig(root) {
  const configPath = path.join(root, 'i18n.config.json');
  const content = fs.readFileSync(configPath, 'utf-8');
  const config = JSON.parse(content);

  // 验证配置结构
  if (!config.source) throw new Error('Missing source language');
  if (!config.systems) throw new Error('Missing systems definition');
  if (!config.languages) throw new Error('Missing languages definition');

  return config;
}

/**
 * 加载 i18n-glossary.json
 * @param {string} root - 项目根目录
 * @returns {object} 术语表对象
 */
export function loadGlossary(root) {
  const glossaryPath = path.join(root, 'i18n-glossary.json');
  try {
    const content = fs.readFileSync(glossaryPath, 'utf-8');
    return JSON.parse(content);
  } catch {
    return {};
  }
}

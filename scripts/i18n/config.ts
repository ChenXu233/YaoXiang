import fs from 'fs';
import path from 'path';
import type { FlatKeys, LocaleJson } from './adapters/locales.ts';

/**
 * `i18n.config.json` 里单个目标语言的配置。
 *
 * 注意 `prompt` 是「语言风格指令」（如喵语/文言文），会拼进翻译 prompt 的
 * 语言指令位；没有则用通用技术文档 prompt。
 */
export interface LanguageConfig {
  /** 该语言在 prompt 里呈现给模型的名称（如「日本語」） */
  name: string;
  /** 语言风格指令；省略则用默认技术文档 prompt */
  prompt?: string;
}

/** 单个翻译系统的配置（决定源文件、目标目录、适配器） */
export interface SystemConfig {
  sourcePath: string;
  targetDir: string;
  cachePath: string;
  adapter: string;
}

/** `i18n.config.json` 的整体形态 */
export interface I18nConfig {
  /** 源语言代码（通常是 zh） */
  source: string;
  batchSize: number;
  model: string;
  systems: Record<string, SystemConfig>;
  languages: Record<string, LanguageConfig>;
  cachePath?: string;
}

/** 术语表：术语 → 指定译法 */
export type Glossary = FlatKeys;

/** 翻译系统运行所需的环境变量 */
export interface TranslateEnv {
  AI_API_KEY: string;
  AI_BASE_URL: string;
  AI_MODEL?: string;
}

/**
 * 加载 i18n.config.json
 * @param root - 项目根目录
 * @returns 配置对象
 */
export function loadConfig(root: string): I18nConfig {
  const configPath = path.join(root, 'i18n.config.json');
  const content = fs.readFileSync(configPath, 'utf-8');
  const config = JSON.parse(content) as I18nConfig;

  // 验证配置结构
  if (!config.source) throw new Error('Missing source language');
  if (!config.systems) throw new Error('Missing systems definition');
  if (!config.languages) throw new Error('Missing languages definition');

  return config;
}

/**
 * 加载 i18n-glossary.json
 * @param root - 项目根目录
 * @returns 术语表对象（缺失或损坏时返回空表）
 */
export function loadGlossary(root: string): Glossary {
  const glossaryPath = path.join(root, 'i18n-glossary.json');
  try {
    const content = fs.readFileSync(glossaryPath, 'utf-8');
    return JSON.parse(content) as Glossary;
  } catch {
    return {};
  }
}

/** 读取目标语言的 locale 文件（不存在返回空对象） */
export function loadTargetJson(targetPath: string): LocaleJson {
  try {
    return JSON.parse(fs.readFileSync(targetPath, 'utf-8')) as LocaleJson;
  } catch {
    return {};
  }
}

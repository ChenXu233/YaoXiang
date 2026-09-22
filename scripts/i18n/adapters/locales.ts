/**
 * locale JSON 的顶层条目：普通 MSG 是字符串，错误码是嵌套对象。
 * `_meta` 也落在这一层。
 */
export type LocaleEntry = string | Record<string, unknown>;

/** locale JSON 的整体形态（`locales/<lang>.json`） */
export type LocaleJson = Record<string, LocaleEntry>;

/** 点分键 → 译文（`extractKeys` 的输出，也是翻译 API 的交换格式） */
export type FlatKeys = Record<string, string>;

/** 错误码条目：字段名 → 文案（`title`/`template`/`help`…） */
export type CodeFields = Record<string, unknown>;

/**
 * 取某个错误码的字段表。
 *
 * `LocaleJson` 的值是联合类型（MSG 是字符串、错误码是对象、`_meta` 是对象），
 * 读字段前必须收窄——本函数就是那个收窄点（例如断言某码是否真的被写成
 * 嵌套对象而非扁平键）。缺失或非对象时返回 `undefined`。
 */
export function codeFields(json: LocaleJson, code: string): CodeFields | undefined {
  const entry = json[code];
  return typeof entry === 'object' && entry !== null ? entry : undefined;
}

/**
 * 错误码键形状（`E0001` / `W1004`）。判定用形状而非「含点即嵌套」：
 * 万一将来引入普通带点键，不会被误当成错误码拍进对象。
 */
const CODE_KEY = /^[EW]\d{4}$/;

/**
 * 从 locale JSON 中提取 key-value 对（扁平字符串 + 嵌套错误码对象，如 E0001.title）
 * @param json - 源 JSON（可能包含 _meta）
 * @returns key → value 映射
 */
export function extractKeys(json: LocaleJson): FlatKeys {
  const result: FlatKeys = {};
  for (const [key, value] of Object.entries(json)) {
    if (key === '_meta') continue;
    if (typeof value === 'string') {
      result[key] = value;
    } else if (typeof value === 'object' && value !== null) {
      for (const [field, v] of Object.entries(value)) {
        if (typeof v === 'string') {
          result[`${key}.${field}`] = v;
        }
      }
    }
  }
  return result;
}

/**
 * 将翻译结果写回目标 JSON（点分 key 还原为嵌套结构）
 *
 * 码键（`E0001.title`）**一律**嵌套成对象，与目标语言是否已有该码无关——
 * 运行时 `src/util/i18n/mod.rs` 只从对象值取条目，扁平键会被静默丢弃。
 *
 * @param targetJson - 目标 JSON
 * @param translations - key → 翻译值
 * @returns 更新后的 JSON
 */
export function applyTranslations(
  targetJson: LocaleJson,
  translations: FlatKeys,
): LocaleJson {
  const result: LocaleJson = { ...targetJson };
  for (const [key, value] of Object.entries(translations)) {
    const dot = key.indexOf('.');
    if (dot > 0 && CODE_KEY.test(key.slice(0, dot))) {
      // E0001.title → result.E0001.title（错误码对象字段）
      const head = key.slice(0, dot);
      const parent = result[head];
      result[head] = {
        ...(typeof parent === 'object' && parent !== null ? parent : {}),
        [key.slice(dot + 1)]: value,
      };
      continue;
    }
    result[key] = value;
  }
  return result;
}

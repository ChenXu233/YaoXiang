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
 * 把错误码条目的 title 统一成句首大写。
 *
 * 诊断标题在终端里独占一行、用户按它检索，大小写必须一致。此前 title 由模型
 * 自行决定大小写（提示词只要求「保持既有译法与风格」，新增的码没有既定译法），
 * 于是同一张表里出现 `Invalid character` 与 `unterminated string` 两种写法。
 *
 * 只改**首字母**：其余部分原样保留——标题是既定产物，不在这里改写措辞。
 * 以技术标识符开头（`const` / `unsafe` / `return`）的标题同样处理：
 * 它们在中文侧也是标识符开头，句首大写后仍是同一个词。
 *
 * 只对有大小写的语言调用（CJK 标题无大小写；ja/文言文/喵语里以标识符开头的
 * 标题是既有写法，不动）。
 *
 * @param json 目标语言 JSON（原地不改，返回新对象）
 * @returns title 已规范化的新对象
 */
export function normalizeCodeTitles(json: LocaleJson): LocaleJson {
  const result: LocaleJson = { ...json };
  for (const [code, entry] of Object.entries(result)) {
    if (!CODE_KEY.test(code) || typeof entry !== "object" || entry === null) {
      continue;
    }
    const title = (entry as CodeFields).title;
    if (typeof title !== "string" || title === "") {
      continue;
    }
    // JavaScript 没有 isLowerCase()——用大小写比较判定「首字符是有大小写的小写字母」。
    // CJK 字符 toLowerCase() === toUpperCase() === 自身，第二个条件为假 ⇒ 不改。
    const first = title[0]!;
    const isLowerLetter = first.toLowerCase() === first && first.toUpperCase() !== first;
    if (!isLowerLetter) {
      continue;
    }
    result[code] = {
      ...(entry as CodeFields),
      title: first.toUpperCase() + title.slice(1),
    };
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

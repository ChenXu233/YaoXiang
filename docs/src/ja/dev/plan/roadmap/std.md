```markdown
---
title: "標準ライブラリステータス"
---

# 標準ライブラリ（Std）

> **モジュールステータス**：ギャップあり（4 項目改善待ち）
> **位置**：`src/std/`
> **最終更新**：2026-06-01

---

## モジュール概要

標準ライブラリは YaoXiang 言語のコア機能モジュールを提供します。IO、数学、文字列、リスト、辞書、ファイルシステム、ネットワーク、并发などのモジュールが含まれています。

**コード量**：5,071 行（14 のサブモジュール）

---

## 機能一覧

### std.io（379 行）- ✅ 完了

| 関数 | 署名 | ステータス |
|------|------|------|
| `print` | `(...args) -> ()` | ✅ |
| `println` | `(...args) -> ()` | ✅ |
| `read_line` | `() -> String` | ✅ |
| `read_file` | `(path: String) -> String` | ✅ |
| `write_file` | `(path: String, content: String) -> Bool` | ✅ |
| `append_file` | `(path: String, content: String) -> Bool` | ✅ |
| `format_fallback` | `(value, type_name: String) -> String` | ✅ |

### std.math（301 行）- ✅ 完了

| 関数 | 署名 | ステータス |
|------|------|------|
| `abs` | `(n: Int) -> Int` | ✅ |
| `max/min` | `(a: Int, b: Int) -> Int` | ✅ |
| `clamp` | `(value: Int, min: Int, max: Int) -> Int` | ✅ |
| `fabs/fmax/fmin` | Float バージョン | ✅ |
| `pow` | `(base: Float, exp: Float) -> Float` | ✅ |
| `sqrt` | `(n: Float) -> Float` | ✅ |
| `floor/ceil/round` | `(n: Float) -> Float` | ✅ |
| `sin/cos/tan` | `(n: Float) -> Float` | ✅ |
| `PI/E/TAU` | 定数 | ✅ |

### std.string（523 行）- ✅ 完了

| 関数 | 署名 | ステータス |
|------|------|------|
| `split` | `(s: String, sep: String) -> List` | ✅ |
| `trim` | `(s: String) -> String` | ✅ |
| `upper/lower` | `(s: String) -> String` | ✅ |
| `replace` | `(s: String, old: String, new: String) -> String` | ✅ |
| `contains/starts_with/ends_with` | `(s: String, sub: String) -> Bool` | ✅ |
| `index_of` | `(s: String, sub: String) -> Int` | ✅ |
| `substring` | `(s: String, start: Int, end: Int) -> String` | ✅ |
| `is_empty/len` | `(s: String) -> Bool/Int` | ✅ |
| `chars` | `(s: String) -> List` | ✅ |
| `concat/repeat/reverse` | 文字列操作 | ✅ |
| `format` | `(format: String, ...args) -> String` | ✅ |

### std.list（784 行）- ✅ 完了

| 関数 | 署名 | ステータス |
|------|------|------|
| `push/pop/append/prepend` | リスト変更 | ✅ |
| `remove_at` | `(list: List, index: Int) -> Any` | ✅ |
| `reverse/concat` | リスト操作 | ✅ |
| `map/filter/reduce` | 高階関数 | ✅ |
| `len/is_empty` | リスト情報 | ✅ |
| `get/set` | インデックスアクセス | ✅ |
| `first/last` | 境界要素 | ✅ |
| `slice` | `(list: List, start: Int, end: Int) -> List` | ✅ |
| `contains/find_index` | 検索 | ✅ |
| `iter/next/has_next` | イテレータプロトコル | ✅ |

### std.dict（335 行）- ✅ 完了

| 関数 | 署名 | ステータス |
|------|------|------|
| `get/set` | 辞書アクセス | ✅ |
| `has` | `(dict: Dict, key: Any) -> Bool` | ✅ |
| `keys/values/entries` | 集合取得 | ✅ |
| `delete` | `(dict: Dict, key: Any) -> Dict` | ✅ |
| `len/is_empty` | 辞書情報 | ✅ |
| `merge` | `(a: Dict, b: Dict) -> Dict` | ✅ |

### std.convert（149 行）- ✅ 完了

- ✅ `to_string` — 通用タイプから文字列への変換
- ✅ 各タイプの `to_string` メソッド：int, float, bool, char, string, list, dict, tuple, set, range

### std.os（1,023 行）- ✅ 完了

- ✅ ファイル操作：open, close, read, write, seek, tell, flush
- ✅ ディレクトリ操作：mkdir, rmdir, read_dir
- ✅ パスチェック：remove, exists, is_file, is_dir
- ✅ ファイル操作：copy, rename
- ✅ 環境変数：get_env, set_env
- ✅ プロセス情報：args, chdir, getcwd

### std.time（507 行）- ✅ 完了

- ✅ 時間取得：now, timestamp, timestamp_ms
- ✅ `sleep` — `(seconds: Float) -> Void`
- ✅ フォーマット：format_time, parse_time（strftime スタイル）
- ✅ DateTime メソッド：year, month, day, hour, minute, second, weekday, to_string

### std.net（177 行）- ⚠️ スタブ実装

| 関数 | 署名 | ステータス |
|------|------|------|
| `http_get` | `(url: String) -> String` | ⚠️ スタブ - `"GET: {url}"` を返す |
| `http_post` | `(url: String, body: String) -> String` | ⚠️ スタブ - `"POST {url}: {body}"` を返す |
| `url_encode` | `(s: String) -> String` | ✅ |
| `url_decode` | `(s: String) -> String` | ✅ |

### std.concurrent（85 行）- ✅ 基本完了

- ✅ `sleep` — `(millis: Int) -> Void`
- ✅ `thread_id` — `() -> String`
- ✅ `yield_now` — `() -> Void`

### std.ffi（265 行）- ✅ 完了

- ✅ `native` — `(symbol: String) -> Never`（コンパイル時傍受）

### std.weak（45 行）- ⚠️ 基礎実装

- ✅ `weak_new` — `(arc) -> Weak`
- ✅ `weak_upgrade` — `(weak) -> Option`
- ⚠️ `StdModule` trait 实现缺失。`use std.weak` でインポート不可

### gen_interfaces（208 行）- ✅ 完了

- ✅ `.yx` インターフェースファイルの自動生成
- ✅ 書き込みディレクトリ、インターフェースファイルの検索をサポート

---

## テストカバレッジ

**ユニットテストはわずか 8 個**。深刻な不足：

| モジュール | ユニットテスト数 | ステータス |
|------|-----------|------|
| io | 0 | ❌ 欠落 |
| math | 0 | ❌ 欠落 |
| string | 0 | ❌ 欠落 |
| list | 0 | ❌ 欠落 |
| dict | 0 | ❌ 欠落 |
| convert | 0 | ❌ 欠落 |
| os | 0 | ❌ 欠落 |
| time | 0 | ❌ 欠落 |
| net | 0 | ❌ 欠落 |
| concurrent | 0 | ❌ 欠落 |
| ffi | 2 | ✅ 基礎カバー |
| gen_interfaces | 6 | ✅ 良好 |

**間接テストカバレッジ**：
- `tests/yx_runner.rs` は E2E テストで一部機能をカバー
- `tests/integration/execution.rs` に基礎統合テストあり

---

## 発見された問題

1. **net モジュールはスタブ実装**：`http_get` と `http_post` はモック文字列を返す
2. **weak モジュールは不完全**：`StdModule` trait 实现缺失。`use std.weak` でインポート不可
3. **os.chdir は実際にはディレクトリを切り替えしない**：ディレクトリ是否存在のみチェックし、`std::env::set_current_dir()` を呼び出していない
4. **string.len はバイト数を返す**：`native_len` は `s.len()` を使用して文字数ではなくバイト数を返す

---

## コード品質評価

| ディメンション | スコア | 説明 |
|------|------|------|
| 未完了事項 | 4 | テスト追加、バグ修正、weak モジュール、HTTP スタブ |
| テストカバレッジ | 深刻な不足 | ユニットテストはわずか 8 個 |
| ドキュメント品質 | 良好 | 各モジュールにモジュールレベルの `//!` ドキュメントコメントあり |
| コードアーキテクチャ | 良好 | モジュール分割が明確 |

---

## 改善待ちら項目

1. **各モジュールにユニットテストを追加**（最高優先度）
2. **`os.chdir` と `string.len` の問題を修正**
3. **weak モジュールの `StdModule` 实现を完善**
4. **実際の HTTP 機能を実装するか、スタブとして明確にマーク**
```
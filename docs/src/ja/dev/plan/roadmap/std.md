---
title: "標準ライブラリの状態"
---

# 標準ライブラリ（Std）

> **モジュール状態**：基本完成（14モジュールのうち13が利用可能、netはスタブ）
> **位置**：`src/std/`
> **最終更新**：2026-06-01

---

## モジュールの概要

標準ライブラリは、YaoXiang言語の中核機能モジュールを提供します。IO、数学、文字列、リスト、辞書、ファイルシステム、ネットワーク、並行処理などのモジュールが含まれています。

**コード量**：5,071行（14サブモジュール）

---

## 機能一覧

### std.io（379行）- ✅ 完成済み

| 関数 | 署名 | 状態 |
|------|------|------|
| `print` | `(...args) -> ()` | ✅ |
| `println` | `(...args) -> ()` | ✅ |
| `read_line` | `() -> String` | ✅ |
| `read_file` | `(path: String) -> String` | ✅ |
| `write_file` | `(path: String, content: String) -> Bool` | ✅ |
| `append_file` | `(path: String, content: String) -> Bool` | ✅ |
| `format_fallback` | `(value, type_name: String) -> String` | ✅ |

### std.math（301行）- ✅ 完成済み

| 関数 | 署名 | 状態 |
|------|------|------|
| `abs` | `(n: Int) -> Int` | ✅ |
| `max/min` | `(a: Int, b: Int) -> Int` | ✅ |
| `clamp` | `(value: Int, min: Int, max: Int) -> Int` | ✅ |
| `fabs/fmax/fmin` | Floatバージョン | ✅ |
| `pow` | `(base: Float, exp: Float) -> Float` | ✅ |
| `sqrt` | `(n: Float) -> Float` | ✅ |
| `floor/ceil/round` | `(n: Float) -> Float` | ✅ |
| `sin/cos/tan` | `(n: Float) -> Float` | ✅ |
| `PI/E/TAU` | 定数 | ✅ |

### std.string（523行）- ✅ 完成済み

| 関数 | 署名 | 状態 |
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

### std.list（784行）- ✅ 完成済み

| 関数 | 署名 | 状態 |
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

### std.dict（335行）- ✅ 完成済み

| 関数 | 署名 | 状態 |
|------|------|------|
| `get/set` | 辞書アクセス | ✅ |
| `has` | `(dict: Dict, key: Any) -> Bool` | ✅ |
| `keys/values/entries` | 集合の取得 | ✅ |
| `delete` | `(dict: Dict, key: Any) -> Dict` | ✅ |
| `len/is_empty` | 辞書情報 | ✅ |
| `merge` | `(a: Dict, b: Dict) -> Dict` | ✅ |

### std.convert（149行）- ✅ 完成済み

- ✅ `to_string` — 汎用 型 から文字列への変換
- ✅ 各型 `to_string` メソッド：int, float, bool, char, string, list, dict, tuple, set, range

### std.os（1,023行）- ✅ 完成済み

- ✅ ファイル操作：open, close, read, write, seek, tell, flush
- ✅ ディレクトリ操作：mkdir, rmdir, read_dir
- ✅ パスチェック：remove, exists, is_file, is_dir
- ✅ ファイル操作：copy, rename
- ✅ 環境変数：get_env, set_env
- ✅ プロセス情報：args, chdir, getcwd

### std.time（507行）- ✅ 完成済み

- ✅ 時刻取得：now, timestamp, timestamp_ms
- ✅ `sleep` — `(seconds: Float) -> Void`
- ✅ フォーマット：format_time, parse_time（strftimeスタイル）
- ✅ DateTimeメソッド：year, month, day, hour, minute, second, weekday, to_string

### std.net（177行）- ⚠️ スタブ実装

| 関数 | 署名 | 状態 |
|------|------|------|
| `http_get` | `(url: String) -> String` | ⚠️ スタブ - `"GET: {url}"` を返す |
| `http_post` | `(url: String, body: String) -> String` | ⚠️ スタブ - `"POST {url}: {body}"` を返す |
| `url_encode` | `(s: String) -> String` | ✅ |
| `url_decode` | `(s: String) -> String` | ✅ |

### std.concurrent（85行）- ✅ 基本完成

- ✅ `sleep` — `(millis: Int) -> Void`
- ✅ `thread_id` — `() -> String`
- ✅ `yield_now` — `() -> Void`

### std.ffi（265行）- ✅ 完成済み

- ✅ `native` — `(symbol: String) -> Never`（コンパイル時傍受）

### std.weak（45行）- ⚠️ 基礎実装

- ✅ `weak_new` — `(arc) -> Weak`
- ✅ `weak_upgrade` — `(weak) -> Option`
- ⚠️ `StdModule` traitの実装が不足しており、`use std.weak` でインポート不可

### gen_interfaces（208行）- ✅ 完成済み

- ✅ `.yx` インターフェースファイルの自動生成
- ✅ 書き込みディレクトリ、インターフェースファイルの検索をサポート

---

## テストカバレッジ

**ユニットテストはわずか8つのみ**、極めて不十分です：

| モジュール | ユニットテスト数 | 状態 |
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
- `tests/yx_runner.rs` がE2Eテストで一部機能をカバー
- `tests/integration/execution.rs` に基本的な統合テストあり

---

## 発見された問題

1. **netモジュールはスタブ実装**： `http_get` と `http_post` はモック文字列を返す
2. **weakモジュールは不完全**：`StdModule` traitの実装が不足しており、`use std.weak` でインポート不可
3. **os.chdir は実際にはディレクトリを切り替えらない**：ディレクトリが存在するかどうかのみチェックし、`std::env::set_current_dir()` を呼び出していない
4. **string.len はバイト数を返す**：`native_len` が `s.len()` を使用して文字数ではなくバイト数を返す

---

## コード品質評価

| 次元 | スコア | 説明 |
|------|------|------|
| 機能完成度 | 85% | 中核機能は完整、高级機能（HTTP）は未実装 |
| テストカバレッジ | 極めて不十分 | ユニットテストはわずか8つのみ |
| ドキュメント品質 | 良好 | 各モジュールにモジュールレベルの `//!` ドキュメントコメントあり |
| コードアーキテクチャ | 良好 | モジュール区分が明確 |

---

## 改善項目

1. **各モジュールにユニットテストを追加する**（最優先）
2. **`os.chdir` と `string.len` の問題を修正する**
3. **weakモジュールの `StdModule` 実装を完成させる**
4. **実際のHTTP機能を実装するか、スタブとして明示的にマークする**
# Native 関数シグネチャ解析とクロージャ呼び出し問題修正計画

> **状態**：✅ 完了
> **日付**：2026-02-19
> **完了日**：2026-02-19

---

## 概要

### 問題背景

現在 YaoXiang 言語では、高階関数（`list.map`、`list.filter`、`list.reduce`）の使用時に2つの関連する問題が存在します：

1. **シグネチャ解析エラー**：`src/std/list.rs` の `map`/`filter`/`reduce` 関数のシグネチャ文字列に無効な型 `Fn` が使用されている
2. **エラーメッセージが误导的**：シグネチャ解析が失敗した際、"Invalid signature 'Float': missing '->'" と表示され、より適切なエラーメッセージが表示されない

### 問題コード

`src/std/list.rs` 72-87行目：

```rust
NativeExport::new(
    "map",
    "std.list.map",
    "(list: List, fn: Fn) -> List",  // ❌ Fn は無効な型（(T) -> T であるべき）
    native_map as NativeHandler,
),
NativeExport::new(
    "filter",
    "std.list.filter",
    "(list: List, fn: Fn) -> List",   // ❌ Fn は無効な型
    native_filter as NativeHandler,
),
NativeExport::new(
    "reduce",
    "std.list.reduce",
    "(list: List, fn: Fn, init: Any) -> Any",  // ❌ Fn は無効な型
    native_reduce as NativeHandler,
),
```

**現在のシグネチャ形式**（正しい）：

```
(list: List, fn: Fn) -> List
  ↑       ↑     ↑
  │       │     └── 引数型（無効）
  │       └── 引数名
  └── 引数型
```

**問題**：`fn` は引数名、`Fn` は型名です。`Fn` は有効な基本型名ではありません。

### ランタイムエラー

テストコード実行時：

```yaoxiang
main = {
    doubled = list.map([1, 2, 3], x => x * 2);
    io.println(doubled);
}
```

出力（現在の動作 - エラー）：

```
[Warning] Invalid signature 'Float': missing '->'
[Warning] Invalid signature 'Float': missing '->'
[Warning] Invalid signature 'Float': missing '->'
Error: Runtime error: Type error: Expected function value
```

修正後の出力（期待する動作）：

```
[Error] Invalid signature: unknown type 'Fn'
Error: (compilation failed)
```

---

## 実装目標

### 目標 1：シグネチャ定義の修正

RFC-010 統一型構文に従い、ジェネリック関数の正しい形式は：

```
関数名: [ジェネリック引数リスト](引数リスト) -> 戻り値型
```

ここでジェネリック引数 `[T]` は関数レベルで宣言され、関数シグネチャ全体に対して作用します。

`map`/`filter`/`reduce` のシグネチャを以下のように修正（ジェネリック引数 `[T]` は関数名の前）：

```rust
// map: ジェネリック [T] のスコープは関数全体
"[T](list: List<T>, fn: (item: T) -> T) -> List<T>"

// filter: ジェネリック [T] のスコープは関数全体
"[T](list: List<T>, fn: (item: T) -> Bool) -> List<T>"

// reduce: ジェネリック [T] のスコープは関数全体
"[T](list: List<T>, fn: (acc: Any, item: T) -> Any, init: Any) -> Any"
```

**シグネチャ構造の説明**：

```
[T](list: List<T>, fn: (item: T) -> T) -> List<T>
│  │         │      │    │        │
│  │         │      │    │        └── 戻り値型（T を使用）
│  │         │      │    └── 引数型（T を使用）
│  │         │      └── 引数名
│  │         └── 引数型（関数型）
│  └── 引数型（List ジェネリック、T を使用）
└── ジェネリック引数宣言（スコープは関数全体）
```

### 目標 2：ジェネリック引数のスコープルール

**ジェネリック引数宣言のシャドウイング禁止**（No Shadowing）：

シグネチャには複数のスコープレベルが存在します：

```
[T](list: List[T], fn: [T](item: T) -> T) -> List[T]
│                      │
│                      └── 内側関数型スコープ（fn の型引数）
└── 外側関数スコープ（関数のジェネリック引数）
```

**ルール**：

1. **同レベルでのシャドウイング禁止**：同一スコープ内のジェネリック引数は同名を名はできない
2. **内側から外側へのシャドウイング禁止**：内側スコープのジェネリック引数は外側と同名を名はできない
3. **関数引数のシャドウィング禁止**：関数引数名はどのジェネリック引数とも同名を名はできない

**有効な例**：

```yaoxiang
// ✅ 有効：ジェネリック引数 T のスコープは関数全体
map: [T](list: List[T], fn: (item: T) -> T) -> List<T>

// ✅ 有効：内側関数型にジェネリック引数なし
map: [T](list: List[T], fn: (item: T) -> T) -> List<T>

// ✅ 有効：複数のジェネリック引数
zip: [T, U](a: List[T], b: List[U]) -> List<(T, U)>

// ✅ 有効：関数引数名はジェネリック引数と異なる
foo: [T](x: Int, y: T) -> T
```

**無効な例**：

```yaoxiang
// ❌ 無効：内側関数のジェネリックが外側のジェネリックをシャドウинг（シャドウинг禁止）
"[T](list: List[T], fn: [T](item: T) -> T) -> List[T]"
# エラー：Generic parameter 'T' in function type shadows outer generic parameter 'T'

// ❌ 無効：ジェネリック引数が同名（同级シャドウинг禁止）
"[T, T](x: T, y: T) -> T"
# エラー：Duplicate generic parameter 'T'

// ❌ 無効：関数引数名がジェネリック引数と同名（シャドウинг禁止）
"[T](T: Int) -> T"
# エラー：Parameter 'T' shadows generic parameter 'T'
```

### 目標 3：シグネチャ引数名チェック

シグネチャ解析時に、引数名の妥当性を検証する必要があります：
1. **引数名は重複できない**（E2093）
2. **ジェネリック引数のシャドウィング禁止**（E2095, E2096）

> **注意**：引数名がキーワードの場合は、パーサーがシグネチャを解析する際に自動的に構文エラーを報告するため、单独の検証は不要です。

例：
```
// 有効なシグネチャ（RFC-010 に準拠）
"[T](list: List<T>, fn: (item: T) -> T) -> List<T>"

// 無効なシグネチャ - 関数引数名がジェネリック引数と同名（シャドウィング禁止）
"[T](list: List<T>, fn: (T: T) -> T) -> List<T>"
# エラー: Parameter 'T' shadows generic parameter 'T'

// 無効なシグネチャ - 重複した引数名
"[T](x: Int, x: Int) -> Int"
# エラー: Invalid signature: duplicate parameter name 'x'

// 注意：引数名がキーワードの場合は、パーサーがシグネチャを解析する際に自動的に構文エラーを報告
```

### 目標 4：错误メッセージの修正

シグネチャ解析でエラーが発生した場合、エラーコードシステム（E2xxx - セマンティック分析段階）で報告する必要があります：

**新規追加が必要なエラーコード**：

| エラーコード | メッセージテンプレート | 説明 |
|--------|----------|------|
| E2090 | Invalid signature: {reason} | シグネチャ解析失敗（汎用） |
| E2091 | Invalid signature: unknown type '{type_name}' | 未知の型 |
| E2092 | Invalid signature: missing '->' | 矢印欠落 |
| E2093 | Invalid signature: duplicate parameter '{name}' | 重複した引数名 |
| E2094 | Invalid signature: generic '{name}' shadows outer generic | ジェネリック引数のシャドウィング |
| E2095 | Invalid signature: parameter '{name}' shadows generic | 引数名がジェネリックをシャドウィング |

> **注意**：引数名がキーワードの場合は、パーサーがシグネチャを解析する際に自動的に構文エラーを報告するため、单独の検証は不要です。

**エラーメッセージ形式**（RFC-013 に準拠）：

```
[Error] E2091: Invalid signature: unknown type 'Fn'
 --> std/list.yx:1:1
  |
1 | "(list: List, fn: Fn) -> List"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: Use a valid type like '(T) -> T' for function parameters
```

---

## 受け入れ方案

### 受け入れ条件 1：コンパイル成功

シグネチャ修正後、テストコードはコンパイルに成功し、"Invalid signature" 警告が表示されなくなります：

```bash
$ cargo run -- run tests/closure_test2.yx
# 以下を出力するはず：
# [Test map:]
# [2, 4, 6]
# [Test filter:]
# [3, 4, 5]
# [Test reduce:]
# 10
# [All tests passed!]
```

### 受け入れ条件 2：错误メッセージが正しい

無効なシグネチャを使用した場合、**Warning** ではなく **Error** を表示し、エラーコードシステム（E2xxx）を使用する必要があります：

```bash
# 無効なシグネチャをテスト
# 期待する出力：
[Error] E2091: Invalid signature: unknown type 'Fn'
[Error] E2092: Invalid signature: missing '->'
[Error] E2093: Invalid signature: duplicate parameter 'x'
```

### 受け入れ条件 3：Lambda 引数名のマッチ

渡される lambda の引数名は、シグネチャで定義された関数引数名と一致する必要があります：

```yaoxiang
// シグネチャ定義：fn: (item: T) -> T
// 渡される lambda：x => x * 2

// ❌ エラー - 引数名が一致しない
list.map([1, 2, 3], x => x * 2)
# 期待するエラー：Parameter name mismatch: expected 'item', got 'x'

// ✅ 正しい - 引数名が一致する
list.map([1, 2, 3], item => item * 2)

// reduce の場合、シグネチャは fn: (acc: Any, item: T) -> Any
list.reduce([1, 2, 3], (accumulator, item) => accumulator + item, 0)
# ✅ 正しい - 引数名が一致する
```

### 受け入れ条件 4：引数名チェック（シグネチャ解析）

シグネチャ解析時に無効な引数名を検出する必要があります：
- 重複した引数名はエラーを報告（E2093）
- ジェネリック引数のシャドウィングはエラーを報告（E2095, E2096）

> **注意**：引数名がキーワードの場合は、パーサーがシグネチャを解析する際に自動的に構文エラーを報告するため、单独の検証は不要です。

```bash
# 期待するエラー：
[Error] E2093: Invalid signature: duplicate parameter 'x'
[Error] E2095: Invalid signature: generic 'T' shadows outer generic
```

### 受け入れ条件 5：高階関数の機能が正常

- `list.map`：リストの各要素に関数を適用し、新しいリストを返す
- `list.filter`：条件を満たす要素を保持し、新しいリストを返す
- `list.reduce`：要素の累積計算を行う

---

## テスト方案

### テストケース 1：基本 map 機能

```yaoxiang
main = {
    // シグネチャ定義: fn: (item: T) -> T、引数名は item
    doubled = list.map([1, 2, 3], item => item * 2);
    io.println(doubled);  // 期待: [2, 4, 6]
}
```

### テストケース 2：基本 filter 機能

```yaoxiang
main = {
    // シグネチャ定義: fn: (item: T) -> Bool、引数名は item
    filtered = list.filter([1, 2, 3, 4, 5], item => item > 2);
    io.println(filtered);  // 期待: [3, 4, 5]
}
```

### テストケース 3：基本 reduce 機能

```yaoxiang
main = {
    // シグネチャ定義: fn: (acc: Any, item: T) -> Any、引数名は acc, item
    sum = list.reduce([1, 2, 3, 4], (acc, item) => acc + item, 0);
    io.println(sum);  // 期待: 10
}
```

### テストケース 4：Lambda 引数名の不一致

```yaoxiang
main = {
    // ❌ エラー - 引数名が一致しない
    // シグネチャ: fn: (item: T) -> T、だが渡される引数名は x
    doubled = list.map([1, 2, 3], x => x * 2);
}
# 期待するコンパイルエラー：Parameter name mismatch: expected 'item', got 'x'
```

### テストケース 5：複雑な lambda

```yaoxiang
main = {
    // 複数引数 lambda
    result = list.reduce([1, 2, 3], (acc, x) => acc * x, 1);
    io.println(result);  // 期待: 6

    // ネスト呼び出し
    data = list.map([1, 2, 3], x => {
        y = x + 1;
        y * 2
    });
    io.println(data);  // 期待: [4, 6, 8]
}
```

### テストケース 6：無効なシグネチャエラーメッセージ

エラーメッセージが正しく表示されるか検証（Error でありエラーコードを使用すること）：

```bash
# 期待する出力：
[Error] E2091: Invalid signature: unknown type 'Fn'
# 以下ではなく：
# [Warning] Invalid signature 'Float': missing '->'
```

### テストケース 7：重複した引数名

```rust
// 無効なシグネチャを想定
// "(x: Int, x: Int) -> Int"
// 期待するコンパイルエラー：
// [Error] Invalid signature: duplicate parameter name 'x'
```

---

## 技術詳細

### 関連コードファイル

| ファイル | 役割 |
|------|------|
| `src/std/list.rs` | Native 関数エクスポート定義（✅ シグネチャ修正済み） |
| `src/frontend/typecheck/mod.rs` | シグネチャ解析ロジック（✅ parse_signature 再実装済み） |
| `src/backends/interpreter/executor.rs` | ランタイムクロージャ呼び出し（✅ MakeClosure 検索修正済み） |
| `src/middle/core/bytecode.rs` | バイトコードデコード（✅ MakeClosure デコーダー追加済み） |
| `src/std/io.rs` | IO モジュール（✅ リスト表示形式修正済み） |
| `src/util/diagnostic/codes/e2xxx.rs` | エラーコード定義（✅ E2090-E2095 追加済み） |
| `src/util/diagnostic/codes/i18n/zh.json` | 中国語 i18n（✅ 追加済み） |
| `src/util/diagnostic/codes/i18n/en.json` | 英語 i18n（✅ 追加済み） |

### シグネチャ解析フロー

1. `TypeCheckResult::new()` が `register_std_native_signatures()` を呼び出す
2. `register_std_native_signatures()` が std モジュールのエクスポートを走査する
3. 各 `Export` に対して `parse_signature(&export.signature, env)` を呼び出す
4. `parse_signature` がシグネチャ文字列を `MonoType::Fn` として解析する
5. 解析失敗時にエラーコードシステムで報告（E2090-E2095）

### 実際のコード修正

1. **`src/std/list.rs:71-88`**：3つの関数のシグネチャ文字列を修正（RFC-010 ジェネリック関数構文）
   ```rust
   "[T](list: List<T>, fn: (item: T) -> T) -> List<T>"
   "[T](list: List<T>, fn: (item: T) -> Bool) -> List<T>"
   "[T](list: List<T>, fn: (acc: Any, item: T) -> Any, init: Any) -> Any"
   ```

2. **`src/frontend/typecheck/mod.rs`**（parse_signature 再実装）：
   - `[T]` ジェネリック引数プレフィックスの解析をサポート
   - 関数型引数 `(item: T) -> T` をサポート
   - 括弧のマッチングを正しく処理（find_matching_close）
   - 引数名の重複チェック（E2093）
   - ジェネリック引数のシャドウィングチェック（E2094、E2095）
   - 定数型シグネチャの処理（`"Float"` など）
   - エラーメッセージを Warning から Error + エラーコードにアップグレード

3. **`src/middle/core/bytecode.rs`**（バイトコードデコーダー）：
   - `Opcode::MakeClosure` デコーダーを追加（以前 catch-all ブランチに飲み込まれて Nop になっていた）

4. **`src/backends/interpreter/executor.rs`**（クロージャ実行）：
   - `MakeClosure` ハンドラーを修正：`FunctionRef::Index` はインデックスを直接使用（名前 "fn_N" を構築するのではなく）

5. **`src/std/io.rs`**（IO モジュール）：
   - print/println がリスト/辞書の読みやすいフォーマット出力をサポート（heap 解析経由）

6. **エラーコード定義**：
   - `src/util/diagnostic/codes/e2xxx.rs`：E2090-E2095 定義とショートカットメソッドを追加
   - `src/util/diagnostic/codes/i18n/zh.json`：中国語の錯誤情報を追加
   - `src/util/diagnostic/codes/i18n/en.json`：英語の錯誤情報を追加

---

## 依存関係

本タスクは他のタスクに依存せず、独立して完了しています。

---

## リスクと注意事項

1. **ジェネリックサポート**：✅ 型システムはジェネリック `List<T>` の解析をサポートしており、ジェネリック引数は TypeRef として解析される
2. **クロージャ環境のキャプチャ**：現在の実装ではクロージャの外部変数キャプチャを処理せず、map/filter/reduce のユースケースにはこの機能は不要
3. **追加で発見・修正された問題**：
   - MakeClosure バイトコードデコーダーが欠落（クロージャが Nop になる原因）
   - MakeClosure 実行時の `FunctionRef::Index` 処理エラー（インデックスを直接使用するのではなく "fn_N" という名前を構築していた）
   - io.println がリスト内容をフォーマット表示できない（ハンドルアドレスのみ表示）
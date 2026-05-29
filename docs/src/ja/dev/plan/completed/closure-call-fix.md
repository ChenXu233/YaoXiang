# ブロック内関数定義とクロージャ関連の問題

> **状態**: ✅ 完了
>
> **作成日**: 2026-02-19
> **完了日**: 2026-02-19

## 一、問題の要約

### 1.1 問題 1：ブロック内関数定義で変数が見つからない

**症状**：

```yaoxiang
main = {
  add = (a, b) => a + b;      //  型注釈なしの関数定義
  result = add(1, 2);         // ❌ Unknown variable: 'add'
}
```

**根本原因**：`src/frontend/typecheck/checking/mod.rs` 第 548-563 行

```rust
// 型注釈がある場合にのみに関数をスコープに追加！
if let Some(crate::frontend::core::parser::ast::Type::Fn { ... }) = type_annotation {
    // ... 関数タイプを構築
    self.add_var(name.to_string(), PolyType::mono(fn_type));  // ❌ 実行されない
}
```

最も簡略な形式 `add = (a, b) => ...` を使用する場合、`type_annotation = None` となり、関数名がスコープに追加されない。

### 1.2 問題 2：モジュールレベル関数は正常に動作する

モジュールレベル関数（例：`main = { ... }`）が正常に動作するのは、異なるコードパスを辿るためである：

```
check_module
  → collect_function_signature (第379-382行)
    → 型注釈がない関数にも型変数を追加
```

これは型推論の基礎インフラは存在しており、`check_fn_stmt` がそれを正しく使用していないことを示している。

### 1.3 問題 3：use std.{io} フィールドアクセスエラー

**症状**：

```yaoxiang
use std.{io}
add: (a: Int, b: Int) -> Int = (a, b) => a + b;
main = {
  result = add(1, 2);
  io.println(result);  // ❌ Cannot access field on non-struct type 'fn(t113) -> void'
}
```

**関連するが異なる問題**：`io` がモジュールではなく関数タイプとして認識される。

### 1.4 関数定義の4つの形式テスト結果

| 形式 | コード | モジュールレベル | ブロック内部 |
|------|------|--------|--------|
| 完全形式 | `add: (a: Int, b: Int) -> Int = (a, b) => a + b` | ✅ | ✅ |
| 簡略形（Lambdaヘッダー省略） | `add: (a: Int, b: Int) -> Int = { return a + b }` | ✅ | ✅ |
| 簡略形（引数型省略） | `add: (a, b) -> Int = (a, b) => { return a + b }` | ✅ | ❌ |
| 最も簡略な形式 | `add = (a, b) => { return a + b }` | ✅ | ❌ |

---

## 二、修正方案

### 2.1 問題 1 の修正：ブロック内関数定義

**状態**: ✅ 修正済み

修正は2つの部分に分かれる：

#### 2.1.1 型検査の修正

`src/frontend/typecheck/checking/mod.rs` の `check_fn_stmt` 関数を修正（第 546-583 行）：
- 型注釈があるかどうかにかかわらず、関数をスコープに追加
- 型注釈がある場合は、その型を使用
- それ以外の場合は、引数から型変数を作成

#### 2.1.2 IR 生成の修正

`src/middle/core/ir_gen.rs` を修正：
1. ネストした関数を格納するための `nested_functions` フィールドを追加（第 152 行）
2. `generate_local_stmt_ir` を修正してネストした関数の IR を生成（第 1013-1032 行）
3. `generate_module_ir` を修正してネストした関数をモジュール関数リストに追加（第 416-417 行）

**検証結果**：
- ✅ コンパイル段階通過（`Unknown variable` エラーが発生しない）
- ✅ 実行時正常に動作

### 2.3 問題 3 の修正：use std.{io}

これは別途調査が必要であり、以下の可能性がある：
1. `use` 文の解析後にモジュールの型が正しく設定されていない
2. フィールドアクセス検査時にモジュールが正しく処理されていない

---

## 三、検収基準

### 3.1 コンパイル検収

- [x] `cargo check` 通過
- [x] ブロック内完全形式関数呼び出し正常（コンパイル段階）
- [x] ブロック内最も簡略な形式関数呼び出し正常（コンパイル段階）

### 3.2 機能検収

- [x] `main = { add = (a,b) => a + b; add(1,2) }` 正常実行
- [x] `main = { add: (a:Int,b:Int)->Int = (a,b)=>a+b; add(1,2) }` 正常実行

### 3.3 現在の状態

| 段階 | モジュールレベル関数 | ブロック内関数 |
|------|-----------|----------|
| 字句解析/構文解析 | ✅ | ✅ |
| 型検査 | ✅ | ✅ 修正済み |
| コード生成 | ✅ | ✅ 修正済み |

---

## 四、未処理問題

### 4.1 use std.{io} フィールドアクセスエラー

**状態**: ✅ 修正済み

**修正内容**：

`src/frontend/typecheck/mod.rs` の `collect_use_statement` 関数を修正（第 645-678 行）：
- サブモジュールに対して誤った Fn 型ではなく、エクスポート関数を含む StructType を作成
- モジュールレジストリからサブモジュールのエクスポート情報を取得

**検証結果**：

```yaoxiang
use std.{io}
main = {
  add = (a, b) => a + b;
  result = add(100, 200);
  io.println(result)  // ✅ 正常に動作
}
```

---

## 四、テストケース

### 4.1 ブロック内関数定義テスト

```yaoxiang
// test_block_fn.yx
main = {
  // 完全形式
  add1: (a: Int, b: Int) -> Int = (a, b) => a + b;

  // 最も簡略な形式（型注釈なし）
  add2 = (a, b) => a + b;

  result1 = add1(1, 2);
  result2 = add2(3, 4);
  result1 + result2  // 10 を返す
}
```

---

## 五、関連ファイル

| ファイル | 行番号 | 説明 |
|------|------|------|
| `src/frontend/typecheck/checking/mod.rs` | 548-584 | `check_fn_stmt` 関数 |
| `src/frontend/typecheck/mod.rs` | 379-382 | `collect_function_signature` |
| `src/frontend/typecheck/mod.rs` | 546-590 | モジュールレベル関数シグネチャ収集ロジック |
# 関数パラメータの mut 構文サポート実装計画

> **ステータス**：実装済み
> **日付**：2026-02-19

---

## 概要

### 問題の背景

現在 YaoXiang 言語の関数パラメータは `mut` キーワードをサポートしておらず、パラメータはデフォルトで不変です。これにより、以下の問題が発生しています：

1. **クロージャパラメータが不変**：例えば `list.map([1,2,3], x => x * 2)` では、クロージャパラメータ `x` が不変であり、クロージャ本体内で `x` を修正できません
2. **関数パラメータが不変**：通常の関数パラメータも修正できず、「パラメータをその場で修正する」パターンを実装できません

### 目標

関数パラメータの `mut` 構文サポートを実装し、関数パラメータを変更可能として宣言できるようにする。

### 構文設計

```yaoxiang
// 通常関数のパラメータ
fn foo(mut x: Int) -> Int {
    x = x + 1  // 有効、修正可能
    x
}

// Lambda パラメータ
f = (mut x) => x + 1

// 高階関数呼び出し
list.map([1, 2, 3], (mut x) => x * 2)  // 有効
```

---

## エラーコード設計

### E2010 vs E2011 の区別

| エラーコード | 説明 | シナリオ |
|--------|------|------|
| E2010 | cannot assign to immutable variable | `x = 1; x = 2`（ユーザーが明示的に変数を修正） |
| **E2011** | **closure parameter requires mut** | `list.map([..., x => ...])`（クロージャパラメータに mut が必要） |

### E2011 の設計

**エラーメッセージ**：
```
error[E2011]: closure parameter '{param_name}' requires 'mut' to be modified
 --> example.yx:1:20
  |
1 | list.map([1,2,3], x => x * 2);
  |                    ^ consider adding 'mut' to parameter: (mut x) => ...
```

**トリガー条件**：
1. クロージャが高階関数にパラメータとして渡される
2. 高階関数内部でクロージャパラメータを修正しようとする
3. クロージャパラメータが `mut` を宣言していない

**修復のヒント**：
- `x => ...` を `(mut x) => ...` に変更することを提案

---

## 受け入れ基準

### パーサー層

- [x] パーサーが `(mut x: Type)` 形式のパラメータをパース可能
- [x] パーサーが `(mut x)` 形式のパラメータをパース可能（型を省略）
- [x] パーサーが Lambda の `(mut x) => body` 構文をサポート

### AST 層

- [x] `Param` 構造体に `is_mut: bool` フィールドを追加
- [x] 型チェックが可変パラメータを正しく識別

### 意味解析層

- [x] 型チェックが可変パラメータを正しく処理
- [x] 可変パラメータは関数本体内で修正可能

### IR 生成層

- [x] IR 生成器が可変パラメータを正しく処理（可変ローカル変数として登録）
- [x] クロージャの可変パラメータがクロージャ関数に正しく渡される

### テスト要件

- [x] テストケース：mut パラメータを持つ通常関数
- [x] テストケース：mut パラメータを持つ Lambda
- [x] テストケース：高階関数で mut パラメータクロージャを使用
- [x] テストケース：エラーシナリオ - 不変パラメータが修正される

---

## 実装手順

### Phase 1: AST の変更

#### 1.1 Param 構造体の変更

**ファイル**：`src/frontend/core/parser/ast.rs`

```rust
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub is_mut: bool,  // 新規追加
    pub span: Span,
}
```

**受け入れ**：
- [x] AST の Param に is_mut フィールドが含まれる

---

### Phase 2: パーサーの変更

#### 2.1 パラメータ解析ロジックの変更

**ファイル**：`src/frontend/core/parser/statements/declarations.rs`

**関数**：`parse_fn_params`

パラメータ名をパースする前に `mut` キーワードを検出：

```rust
// mut キーワードを検出
let is_mut = state.skip(&TokenKind::KwMut);

let name = match state.current().map(|t| &t.kind) {
    Some(TokenKind::Identifier(n)) => n.clone(),
    _ => break,
};
state.bump();

// 型注釈をパース
let ty = if state.skip(&TokenKind::Colon) {
    parse_type_annotation(state)
} else {
    None
};

params.push(Param {
    name,
    ty,
    is_mut,  // 新規追加
    span: param_span,
});
```

**受け入れ**：
- [x] `(mut x: Int)` が正しくパースされる
- [x] `(mut x)` が正しくパースされる（型注釈なし）
- [x] `(x: Int)` が不変としてパースされる（is_mut = false）

---

### Phase 3: 型チェック

#### 3.1 型チェッカーが is_mut 情報を伝達

**ファイル**：`src/frontend/typecheck/`

型チェック段階でパラメータの変更可能性情報を伝達する必要がある。

**受け入れ**：
- [x] 型チェックが可変パラメータを持つ関数定義を通す
- [x] 型チェックが不変パラメータが修正されるコードを拒否する

---

### Phase 4: IR 生成

#### 4.1 generate_function_ir の変更

**ファイル**：`src/middle/core/ir_gen.rs`

パラメータ登録ロジックを変更し、`is_mut` に基づいて可変かどうかを判断：

```rust
for (i, param) in params.iter().enumerate() {
    // パラメータを登録
    self.register_local(&param.name, i);
    // mut パラメータのみ可変として登録
    if param.is_mut {
        self.current_mut_locals.insert(i);
    }
}
```

#### 4.2 generate_lambda_body_ir の変更

**ファイル**：`src/middle/core/ir_gen.rs`

クロージャパラメータの処理も同様に変更：

```rust
for (i, param) in params.iter().enumerate() {
    self.register_local(&param.name, i);
    // mut パラメータのみ可変として登録
    if param.is_mut {
        self.current_mut_locals.insert(i);
    }
}
```

**受け入れ**：
- [x] 可変パラメータが IR で正しく可変としてマークされる
- [x] クロージャの可変パラメータが正しく伝達される

---

## 関連ファイル

| モジュール | ファイル | 変更内容 |
|------|------|----------|
| AST | `src/frontend/core/parser/ast.rs` | Param 構造体に is_mut フィールドを追加 |
| パーサー | `src/frontend/core/parser/statements/declarations.rs` | パラメータ解析で mut キーワードをサポート、関数型注釈で mut パラメータを識別 |
| パーサー | `src/frontend/core/parser/statements/bindings.rs` | binding パラメータ解析で mut キーワードをサポート |
| パーサー | `src/frontend/core/parser/pratt/nud.rs` | 型付きパラメータリストで mut 接頭辞をサポート |
| パーサー | `src/frontend/core/parser/pratt/led.rs` | Lambda パラメータ変換で Expr::Lambda をサポート |
| パーサー | `src/frontend/core/parser/pratt/mod.rs` | Lambda パラメータに is_mut: false のデフォルト値を追加 |
| IR 生成 | `src/middle/core/ir_gen.rs` | is_mut に基づいて可変ローカル変数を登録、lambda 状態隔离を修正 |
| テスト | `src/frontend/typecheck/tests/*.rs` | Param 構築に is_mut フィールドを追加するように更新 |
| テスト | `tests/mut_param_test.yx` | mut パラメータの成功シナリオテスト |
| テスト | `tests/mut_param_error_test.yx` | 不変パラメータ修正エラーシナリオテスト |

---

## テストケース

### 成功シナリオ

```yaoxiang
// 1. 通常関数の可変パラメータ
fn increment(mut x: Int) -> Int {
    x = x + 1
}
main = {
    result = increment(5);
}

// 2. Lambda の可変パラメータ
main = {
    f = (mut x) => {
        x = x + 1
        x
    };
    result = f(5);
}

// 3. 高階関数で可変パラメータクロージャを使用
use std.{io, list}
main = {
    result = list.map([1, 2, 3], (mut x) => {
        x = x * 2
        x
    });
}
```

### エラーシナリオ

#### E2010 - 通常不変変数の修正

```yaoxiang
// 不変パラメータが修正される - E2010
fn foo(x: Int) -> Int {
    x = x + 1  // E2010: cannot assign to immutable variable
}
```

#### E2011 - クロージャパラメータに mut が必要（新規追加）

```yaoxiang
// クロージャパラメータに mut を宣言していない - E2011
list.map([1, 2, 3], x => x * 2)
// E2011: closure parameter 'x' requires 'mut' to be modified
// help: consider adding 'mut' to parameter: (mut x) => ...

list.filter([1, 2, 3], x => x > 2)
// E2011: closure parameter 'x' requires 'mut' to be modified

list.reduce([1, 2, 3], (acc, x) => acc + x, 0)
// E2011: closure parameter 'acc' requires 'mut' to be modified
// E2011: closure parameter 'x' requires 'mut' to be modified
```

---

## リスクと注意事項

1. **後方互換性**：既存のコードで `mut` パラメータを書かない場合、不変動作を維持
2. **型推論**：型注釈を省略した場合 `(mut x)` は型を自動推論できる必要がある
3. **クロージャシナリオ**：可変パラメータがクロージャ関数本体に正しく伝達されることを確認

---

## 実装中に発見・修正した追加の問題

### 1. Lambda IR 生成の状態隔離

**問題**：`generate_lambda_body_ir` がクロージャ関数本体を生成する際、`current_mut_locals` と `current_local_names` をクリアするため、親関数の可変性情報が失われる。

**修正**：lambda body IR 生成開始前に親関数の状態（`current_mut_locals`、`current_local_names`、`next_temp`）を保存し、終了後に復元する。

### 2. Lambda 戻り値レジスタの競合

**問題**：`generate_lambda_body_ir` が固定レジスタ 0を戻り値レジスタとして使用するため、パラメータレジスタ 0と競合し、MutChecker が誤った警告を発生させる。

**修正**：`self.next_temp_reg()` を使用して独立した戻り値レジスタを割り当てるように変更。

### 3. リストリテラルの StoreIndex の可変性

**問題**：リストリテラル `[1, 2, 3]` が複数の `StoreIndex` で要素を書き込むと、2回目以降の書き込みが MutChecker によって不変変数への書き込みとみなされる。

**修正**：`AllocArray` の後、リストの一時レジスタを変更可能として登録（`current_mut_locals.insert(result_reg)`）。

### 4. 関数型注釈での `mut` パラメータの識別

**問題**：RFC-010 関数型注釈 `(mut x: Int) -> Int` をパースする際、`looks_like_named_params` 検出で `KwMut` で始まるパラメータリストを識別できず、旧構文として誤って処理される。

**修正**：2箇所の `looks_like_named_params` 検出に `state.at(&TokenKind::KwMut)` 判断を追加。
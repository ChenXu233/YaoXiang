# For ループとシャドウイングチェック実装ドキュメント

## 概要

本文書は YaoXiang 言語における for ループ変数の可変性設計とシャドウイングチェックの実装方案を記述ものである。

**背景問題**：

- 現在の `for i in 1..5` 構文は IR 段階でエラーが発生する。ループ変数が可変としてマークされていないためである
- 言語にはシャドウイング（shadowing）の禁止ルールを実装する必要がある
- ループ変数のループ内部での可変性は明示的に宣言する必要があり、`let mut` の構文ルールと一貫性を保つ必要がある。しかし、言語には `let` 構文がないため、シャドウイングを禁止する必要がある。

## 設計原則

1. **シャドウイングの禁止**：任意の名前空間（for、if、{}コードブロックなど）で新規に宣言された変数は、外部の既存の変数をシャドウィング（覆い隠す）ことはできない
2. **明示的な可変性**：可変性は `mut` キーワードによって明示的に宣言する必要がある。ループ変数はデフォルトで不変である
3. **各イテレーションでの新規バインディング**：for ループの変数は各イテレーションで新しいバインディングを作成する。同一の変数を変更するわけではない

## 実装内容

### 1. シャドウイング禁止チェック

#### 1.1 型検査段階

新規変数の作成箇所すべてでシャドウィングを検出する：

- **for ループ**：ループ変数名が現在または外側のスコープで既に宣言されているかを検出する
- **let 宣言**：変数名が現在のスコープで既に宣言されているかを検出する
- **if/while などのブロック文**：内部で宣言された変数が外部をシャドウィングしていないかを検出する

#### 1.2 実装箇所

以下のファイルを修正する：

- `src/frontend/typecheck/checking/mod.rs` - シャドウィング検出ロジックを追加

#### 1.3 エラーコード

新しいエラーコード `E2xxx`（未定）を追加する：

```
[E2xxx] 変数シャドウィングエラー
error: cannot shadow existing variable 'x'
 --> example.yx:3:5
  |
3 |     for x in 1..5 {
  |     ^ variable 'x' is already declared in outer scope
```

### 2. for ループ変数の可変性

#### 2.1 構文設計

```yaoxiang
for i in 1..5 {      // i は不変（デフォルト）
    print(i)         // OK
    i = i + 1        // エラー：cannot assign to immutable variable
}

for mut i in 1..5 {  // i は可変
    i = i + 1        // OK
}
```

#### 2.2 構文解析

`src/frontend/core/parser/statements/control_flow.rs` を修正する：

- for 文を解析する際、`for` キーワードの後に `mut` があるかどうかを確認する
- `mut` がある場合は、AST ノードに記録する

AST 構造の変更：

```rust
StmtKind::For {
    var,           // 変数名
    var_mut: bool, // 新規：変数が可変かどうか
    iterable,
    body,
    label,
}
```

#### 2.3 IR 生成

`src/middle/core/ir_gen.rs` の `generate_for_loop_ir` を修正する：

- `var_mut` が true の場合、ループ変数を `current_mut_locals` に追加する
- これにより、MutChecker がその変数への再代入を許可する

```rust
// generate_for_loop_ir 内
if var_mut {
    self.current_mut_locals.insert(var_reg);
}
```

## 実装効果（例）

### 例 1：基本的な for ループ

```yaoxiang
// 入力
for i in 1..5 {
    print(i)
}

// 出力
1
2
3
4
```

### 例 2：for ループ変数の変更

```yaoxiang
// 入力
for mut i in 1..3 {
    i = i + 10
    print(i)
}

// 出力
11
12
13
```

### 例 3：シャドウィング禁止 - for ループ

```yaoxiang
// 入力
i = 10
for i in 1..5 {
    print(i)
}

// エラー出力
error [E2xxx] cannot shadow existing variable 'i'
 --> example.yx:2:5
  |
2 |     for i in 1..5 {
  |         ^ variable 'i' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### 例 4：{}コードブロックでのシャドウィング禁止

```yaoxiang
// 入力
x = 1
{
    x = 2  // エラー！
    print(x)
}

// エラー出力
error [E2xxx] cannot shadow existing variable 'x'
 --> example.yx:3:1
  |
3 |     x = 2
  |     ^ variable 'x' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### 例 5：if ブロック内のシャドウィング

```yaoxiang
// 入力
x = 1
if true {
    x = 2  // エラー！
    print(x)
}

// エラー出力
error [E2xxx] cannot shadow existing variable 'x'
 --> example.yx:4:5
  |
4 |         x = 2
  |         ^ variable 'x' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### 例 6：ループ体内での不変変数の変更

```yaoxiang
// 入力
for i in 1..5 {
    i = i + 1
}

// エラー出力
error [E2010] Cannot assign to immutable variable 'i'
 --> example.yx:2:5
  |
2 |     i = i + 1
  |     ^ cannot assign to immutable variable 'i'
help: Use 'mut' to declare a mutable variable
```

## 2. for ループ変数は各イテレーションで新規バインディング

```yaoxiang
// 入力
for i in 1..3 {
    print(i)
}

// ループ本体内の i は各イテレーションで新しいバインディングになる。これは、各ループ繰り返しの終了時にループ体内的 i が破棄され、次のイテレーションで新しい i のバインディングが作成されるためです。同一の変数を変更するのではありません。
```

## 検収方案

### 機能検収

1. **for ループ基礎機能**
   - [ ] `for i in 1..5 { print(i) }` が正常に 1-4 を出力する
   - [ ] `for mut i in 1..5 { i = i + 1; print(i) }` が正常に 2-5 を出力する

2. **シャドウィング禁止**
   - [ ] 外部に変数が存在する場合、for ループで同名変数を使用するとエラーになる
   - [ ] 外部に変数が存在する場合、同名変数を宣言するとエラーになる
   - [ ] if/while ブロック内でシャドウィングするとエラーになる
   - [ ] 関数間スコープを跨いだシャドウィング検出

3. **可変性チェック**
   - [ ] for ループ変数はデフォルトで不変。変更するとエラーになる
   - [ ] for mut 変数は変更可能で、正常に動作する

### エラーメッセージ検収

- [ ] シャドウィングエラーメッセージが明確であり、覆い隠された変数と位置を示している
- [ ] 可変性エラーメッセージが既存の E2010 スタイルと一貫している

## テスト方案

### ユニットテスト

`src/frontend/typecheck/tests/` に以下を追加する：

```rust
#[test]
fn test_for_loop_basic() {
    // 基本 for ループのテスト
}

#[test]
fn test_for_loop_mut() {
    // for mut のテスト
}

#[test]
fn test_shadowing_for_loop() {
    // for ループシャドウィング検出のテスト
}

#[test]
fn test_shadowing_block() {
    // シャドウィング検出のテスト
}

#[test]
fn test_shadowing_if_block() {
    // if ブロックシャドウィング検出のテスト
}
```

### 統合テスト

`docs/src/tutorial/examples/` にテストケースを追加する：

1. `test_for_loop.yx` - for ループ基本テスト
2. `test_shadowing.yx` - シャドウィング検出テスト

### 手動テスト

```bash
# 基本機能のテスト
cargo run -- run docs/src/tutorial/examples/std_io_examples.yx

# シャドウィングエラーのテスト
echo 'i = 10; for i in 1..5 { print(i) }' | cargo run -- run -
```

### 回帰テスト

既存のテストが本次の修正により失敗しないことを確認する：

```bash
cargo test
```
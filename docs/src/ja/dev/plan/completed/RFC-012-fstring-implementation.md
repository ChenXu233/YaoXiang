# RFC-012 F-String テンプレート文字列実装計画

> **状態**: ✅ 完了
> **RFC ベース**: RFC-012 F-String Template Strings
> **変換戦略**: `format()` 呼び出しへの統一変換
> **完了日**: 2025-07

---

## 実装目標

YaoXiang 言語に f-string テンプレート文字列の糖衣構文サポートを追加する：

```yaoxiang
// 変数補間
name = "Alice"
greeting = f"Hello {name}"        // → format("Hello {}", name)

// 式補間
x = 10
y = 20
result = f"Sum: {x + y}"         // → format("Sum: {}", x + y)

// フォーマット指定子
pi = 3.14159
s = f"Pi: {pi:.2f}"              // → format("Pi: {:.2f}", pi)

// 複数補間
items = [1, 2, 3]
s = f"Count: {len(items)}, sum: {sum(items)}"
```

---

## アーキテクチャ設計

### 基本原則

1. **統一変換戦略** - すべての f-string を `format()` 呼び出しに統一変換
2. **コンパイル時糖衣構文** - 新規ランタイム能力の追加なし、純粋な前方処理
3. **定数評価の拡張** - IR 層で定数評価を拡張し、コンパイル時計算をサポート

### データフロー

```
ソースコード (f"...")
    ↓
Lexer: f" 接頭辞の識別
    ↓
Parser: 補間式の解析
    ↓
AST: FString ノードの新規追加
    ↓
TypeCheck: 式の型の検証
    ↓
Codegen: format() 呼び出しへの変換
    ↓
IR/ターゲットコード
```

---

## 実装手順

### Phase 1: Lexer  字句解析

**目標**: f-string 構文の識別

**ファイル**: `src/frontend/core/lexer/`

**変更内容**:

1. **tokens.rs** - 新規トークン種別の追加
   ```rust
   // 新規 FStringLiteral token（元の f-string 内容を保持）
   FStringLiteral(String),
   ```

2. **tokenizer.rs** - f" 接頭辞の識別
   ```rust
   // next_token() に追加
   '"' => {
       // 前の文字が 'f' かどうかを確認
       // f なら scan_fstring() を呼び出し
       // それ以外は scan_string() を呼び出し
   }
   ```

3. **literals.rs** - f-string スキャンの実装
   ```rust
   pub(lexer: &mut Lexer<'_>) -> Option<Token> {
       fn scan_fstring
       // f"..." の内容をスキャン
       // {expression} 補間を解析
       // FStringLiteral(String) を返す
   }
   ```

**受入基準**:
- [x] `f"hello"` が FStringLiteral token として識別される
- [x] `f"Hello {name}"` が補間境界を正しく解析する
- [x] エラー: 閉じられていない `{` に対して明確なエラーメッセージ（`UnterminatedFStringInterpolation`）

---

### Phase 2: Parser 構文解析

**目標**: f-string を AST ノードとして解析

**ファイル**: `src/frontend/core/parser/`

**変更内容**:

1. **ast.rs** - 新規 AST ノードの追加
   ```rust
   pub enum Expr {
       // ... 既存 ...
       /// F-string テンプレート文字列
       FString {
           segments: Vec<FStringSegment>,  // テキスト段と補間式
           span: Span,
       },
   }

   pub enum FStringSegment {
       /// テキスト断片
       Text(String),
       /// 補間式
       Interpolation {
           expr: Box<Expr>,
           format_spec: Option<String>,  // オプションのフォーマット指定子
       },
   }
   ```

2. **pratt/nud.rs** - f-string 字句解析の解析
   ```rust
   // nud テーブルに追加
   TokenKind::FStringLiteral(_) => Some((BP_HIGHEST, Self::parse_fstring)),

   fn parse_fstring(&mut self) -> Option<Expr> {
       // FStringLiteral 文字列を FString AST ノードに解析
   }
   ```

**受入基準**:
- [x] `f"hello"` が `Expr::FString { segments: [Text("hello")] }` として解析される
- [x] `f"hello {x}"` が補間式を正しく解析する
- [x] `f"Pi: {pi:.2f}"` がフォーマット指定子を正しく解析する

---

### Phase 3: TypeCheck 型検査

**目標**: 補間式の型の検証

**ファイル**: `src/frontend/typecheck/inference/`

**変更内容**:

1. **expressions.rs** - 型推論
   ```rust
   // 新規 f-string 型推論
   fn infer_fstring(&mut self, segments: &[FStringSegment]) -> Result<MonoType> {
       // f-string は常に String 型を返す
       // 各補間式の型が Stringable trait を実装しているかを検証
   }
   ```

2. **制約生成**（必要に応じて）
   ```rust
   // 補間式に対して、Stringable 制約を追加
   ```

**受入基準**:
- [x] `f"{42}"` の型は String
- [x] `f"{some_int}"` が Int → Stringable を正しく検証
- [ ] エラー: Stringable をサポートしない型に対して明確なエラー（trait システム整備後に実装）

---

### Phase 4: Codegen コード生成

**目標**: `format()` 呼び出しへの変換

**ファイル**: `src/middle/core/ir_gen.rs` または新規 `fstring.rs`

**変更内容**:

1. **`format()` 呼び出しへの変換**
   ```rust
   // 変換例
   f"Hello {name}" → format("Hello {}", name)
   f"Pi: {pi:.2f}" → format("Pi: {:.2f}", pi)
   ```

2. **IR 生成**
   ```rust
   fn gen_fstring(&mut self, segments: &[FStringSegment]) -> Operand {
       // format 呼び出しの構築
       // format_str: "Hello {}"
       // args: [name]
   }
   ```

**受入基準**:
- [x] `f"hello"` が正しい format 呼び出しを生成
- [x] `f"x = {x}"` が引数を正しく渡す
- [x] `f"Pi: {pi:.2f}"` がフォーマット指定子を正しく渡す

---

### Phase 5: 定数評価最適化

**目標**: コンパイル時定数計算

**ファイル**: `src/middle/core/ir_gen.rs`

**変更内容**:

1. **eval_const_expr の拡張**
   ```rust
   fn eval_const_expr(&self, expr: &Expr) -> Option<ConstValue> {
       match expr {
           // 既存
           Expr::Lit(lit) => eval_literal(lit),

           // 新規: f-string の再帰的評価
           Expr::FString { segments } => {
               let mut result = String::new();
               for seg in segments {
                   match seg {
                       FStringSegment::Text(s) => result.push_str(s),
                       FStringSegment::Interpolation { expr, .. } => {
                           // 式の再帰的評価
                           let val = self.eval_const_expr(expr)?;
                           result.push_str(&val.to_string());
                       }
                   }
               }
               Some(ConstValue::String(result))
           }

           // 既存: format() 定数呼び出しのサポート
           Expr::Call { func, args } if is_const_format(func) => {
               self.eval_const_format(args)
           }
       }
   }
   ```

2. **定数注入**
   ```rust
   // gen_expr 内
   if let Some(const_val) = self.eval_const_expr(expr) {
       // 定数値を直接使用、ランタイム呼び出しの生成不要
       return Operand::Const(const_val);
   }
   ```

**受入基準**:
- [x] `f"hello"` がコンパイル時に定数 "hello" として評価される
- [x] `f"x = {1+2}"` がコンパイル時に "x = 3" として評価される
- [x] 非定数補間は正しくランタイム呼び出しを生成

---

## テスト設計

### ユニットテスト

#### 1. Lexer テスト

**ファイル**: `src/frontend/core/lexer/tests/fstring.rs`（新規）

```rust
#[test]
fn test_fstring_basic() {
    let mut lexer = Lexer::new(r#"f"hello""#);
    let token = lexer.next_token().unwrap();
    assert!(matches!(token.kind, TokenKind::FStringLiteral(_)));
}

#[test]
fn test_fstring_with_interpolation() {
    let mut lexer = Lexer::new(r#"f"hello {name}""#);
    let token = lexer.next_token().unwrap();
    // token 内容が補間マークを含むことを検証
}

#[test]
fn test_fstring_unclosed_brace_error() {
    let mut lexer = Lexer::new(r#"f"hello {name""#);
    // エラーメッセージを検証
}
```

#### 2. Parser テスト

**ファイル**: `src/frontend/core/parser/tests/fstring.rs`（新規）

```rust
#[test]
fn test_parse_fstring_text() {
    let tokens = tokenize(r#"f"hello""#);
    let ast = parse(tokens);
    assert_matches!(ast, Expr::FString { segments, .. }
        if segments.len() == 1
    );
}

#[test]
fn test_parse_fstring_interpolation() {
    let tokens = tokenize(r#"f"hello {name}""#);
    let ast = parse(tokens);
    // segments = [Text("hello "), Interpolation(Var("name"))] を検証
}

#[test]
fn test_parse_fstring_format_spec() {
    let tokens = tokenize(r#"f"Pi: {pi:.2f}""#);
    let ast = parse(tokens);
    // format_spec = Some(".2f") を検証
}
```

#### 3. TypeCheck テスト

**ファイル**: `src/frontend/typecheck/tests/fstring.rs`（新規）

```rust
#[test]
fn test_fstring_type_int() {
    let code = r#"
        x = 10
        s = f"value: {x}"
    "#;
    check_types(code);
}

#[test]
fn test_fstring_type_not_stringable() {
    let code = r#"
        struct NotStringable
        x = NotStringable()
        s = f"value: {x}"  // エラーを出すべき
    "#;
    check_type_error(code, "does not implement Stringable");
}
```

#### 4. Codegen テスト

**ファイル**: `tests/integration/fstring.rs`（新規）

```rust
#[test]
fn test_fstring_basic() {
    let result = run(r#"
        print(f"hello world")
    "#);
    assert_eq!(result, "hello world");
}

#[test]
fn test_fstring_interpolation() {
    let result = run(r#"
        name = "Alice"
        print(f"Hello {name}")
    "#);
    assert_eq!(result, "Hello Alice");
}

#[test]
fn test_fstring_format_spec() {
    let result = run(r#"
        pi = 3.14159
        print(f"Pi: {pi:.2f}")
    "#);
    assert_eq!(result, "Pi: 3.14");
}

#[test]
fn test_fstring_expression() {
    let result = run(r#"
        x = 10
        y = 20
        print(f"{x} + {y} = {x + y}")
    "#);
    assert_eq!(result, "10 + 20 = 30");
}

#[test]
fn test_fstring_const_eval() {
    let result = run(r#"
        x = f"hello {1+2}"
        print(x)
    "#);
    // 定数評価の結果
    assert_eq!(result, "hello 3");
}
```

### 統合テスト

```rust
// 実際のシナリオをテスト
#[test]
fn test_fstring_logging() {
    let code = r#"
        log(level: String, msg: String) = () => {
            timestamp = "2024-01-01"
            print(f"[{timestamp}] {level}: {msg}")
        }
        log("INFO", "system started")
    "#;
    // 期待出力: [2024-01-01] INFO: system started
}

#[test]
fn test_fstring_json_like() {
    let code = r#"
        name = "Alice"
        age = 30
        print(f"{ '{name}': '{name}', 'age': {age} }")
    "#;
    // 期待出力: { "name": "Alice", "age": 30 }
}
```

---

## 主要ファイル一覧

| ファイル | 変更種別 | 説明 |
|------|---------|------|
| `src/frontend/core/lexer/tokens.rs` | 変更 | FStringLiteral の新規追加 |
| `src/frontend/core/lexer/tokenizer.rs` | 変更 | f" 接頭辞の識別 |
| `src/frontend/core/lexer/literals.rs` | 変更 | f-string のスキャン |
| `src/frontend/core/parser/ast.rs` | 変更 | FString ノードの新規追加 |
| `src/frontend/core/parser/pratt/nud.rs` | 変更 | f-string の解析 |
| `src/frontend/typecheck/inference/expressions.rs` | 変更 | 型推論 |
| `src/middle/core/ir_gen.rs` | 変更 | コード生成 + 定数評価 |
| `src/frontend/core/lexer/tests/fstring.rs` | 新規追加 | Lexer テスト |
| `src/frontend/core/parser/tests/fstring.rs` | 新規追加 | Parser テスト |
| `src/frontend/typecheck/tests/fstring.rs` | 新規追加 | TypeCheck テスト |
| `tests/integration/fstring.rs` | 新規追加 | 統合テスト |

---

## 依存関係とリスク

### 依存関係

- **既存**: `format()` 関数 (`src/std/string.rs`)
- **既存**: 定数評価フレームワーク (`ir_gen.rs::eval_const_expr`)
- **不要**: 新規外部依存関係

### リスク

1. **ネスト波括弧の解析**: `{ { x } }` シナリオ
   - 解決: RFC 規定でネスト使用を制限

2. **フォーマット指定子の複雑さ**
   - 解決: 既存の format 関数の解析ロジックを再利用

---

## マイルストーン

- [x] Phase 1: Lexer が f-string を識別
- [x] Phase 2: Parser が AST に解析
- [x] Phase 3: TypeCheck が型の検証
- [x] Phase 4: Codegen が `format()` に変換
- [x] Phase 5: 定数評価最適化
- [x] 完全テストカバレッジ（27 テスト: 10 Lexer + 6 Parser + 4 TypeCheck + 7 統合）

---

## 付録

### 参考実装

- Python f-strings: https://docs.python.org/3/tutorial/inputoutput.html
- Rust format!: https://doc.rust-lang.org/std/macro.format.html

### 関連 RFC

- RFC-012: F-String Template Strings（本文書はこれをベースとしている）

---

## 実装記録

### 実際の変更ファイル

| ファイル | 変更種別 | 具体的な変更 |
|------|---------|-----------|
| `src/frontend/core/lexer/tokens.rs` | 変更 | `FStringLiteral(String)` token および `UnterminatedFStringInterpolation` エラーの新規追加 |
| `src/frontend/core/lexer/tokenizer.rs` | 変更 | `scan_identifier()` 内で `f"` 接頭辞を検出し `scan_fstring()` を呼び出し |
| `src/frontend/core/lexer/literals.rs` | 変更 | `scan_fstring()` 関数（约180行）の新規追加。`{}` 補間、`{ }` エスケープ、ネスト波括弧深度追跡をサポート |
| `src/frontend/core/lexer/mod.rs` | 変更 | `log_token()` に FStringLiteral 分岐を新規追加；fstring テストモジュールの導入 |
| `src/frontend/core/parser/ast.rs` | 変更 | `FString` AST ノードおよび `FStringSegment` 列挙型の新規追加 |
| `src/frontend/core/parser/pratt/nud.rs` | 変更 | `parse_fstring()`、`parse_fstring_segments()`、`split_format_spec()` の新規追加 |
| `src/frontend/typecheck/inference/expressions.rs` | 変更 | `infer_expr()` に `Expr::FString` 分岐を新規追加、`MonoType::String` を返す |
| `src/middle/core/ir_gen.rs` | 変更 | `get_expr_span()`、`eval_const_expr()`、`generate_expr_ir()` の3箇所に FString 処理を追加 |
| `src/frontend/core/lexer/tests/fstring.rs` | 新規追加 | 10 件の Lexer テスト |
| `src/frontend/core/parser/tests/fstring.rs` | 新規追加 | 6 件の Parser テスト |
| `src/frontend/typecheck/tests/fstring.rs` | 新規追加 | 4 件の TypeCheck テスト |
| `tests/integration/fstring.rs` | 新規追加 | 7 件のエンドツーエンド統合テスト |
| `tests/integration.rs` | 変更 | fstring 統合テストモジュールの登録 |

### 実装のポイント

1. **Lexer**: f-string を単一の `FStringLiteral` token として保持し、`{}` 補間マークは文字列内容内に保持
2. **Parser**: `parse_fstring_segments()` が元の内容を `Text`/`Interpolation` セグメントに分割し、補間式は完全に再度 lexer+parser で解析
3. **コード生成**: `std.string.format()` 呼び出しに変換し、位置プレースホルダー `{0}`, `{1}` などを使用；フォーマット指定子如 `{0:.2f}` はそのまま传递
4. **定数最適化**: すべての補間式がコンパイル時定数（かつフォーマット指定子なし）の場合、f-string 全体をコンパイル時に定数文字列として畳み込み
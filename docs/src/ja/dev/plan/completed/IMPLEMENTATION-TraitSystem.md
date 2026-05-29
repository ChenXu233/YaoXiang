# YaoXiang 言語 Trait システム 完全実装ドキュメント

> YaoXiang 言語 Trait システム実装ガイド
>
> RFC-011 ジェネリクスシステム設計に基づく

---

## 目次

- [概要](#概要)
- [フェーズ C1：コア Trait 構文解析](#フェーズ-c1コア-trait-構文解析)
- [フェーズ C2：Trait 境界表現と制約解決](#フェーズ-c2trait-境界表現と制約解決)
- [フェーズ C3：Trait 継承](#フェーズ-c3trait-継承)
- [フェーズ C4：Trait 実装検査](#フェーズ-c4trait-実装検査)
- [フェーズ C5：高度な機能](#フェーズ-c5高度な機能)
- [受入基準](#受入基準)

---

## 概要

### 設計目標

YaoXiang 言語の Trait システムを実現し、以下をサポート：

- Trait 定義：`type TraitName = { ... }`
- Trait 制約：`[T: Trait]` / `[T: A + B]`
- Trait 継承：`type Trait = Parent { ... }`
- Trait 実装：`impl Trait for Type { ... }`

### 構文設計

```yaoxiang
# Trait 定義
type Clone = { clone: (Self) -> Self }
type Add = { add: (Self, Self) -> Self }
type Container[T] = { get: (Self) -> T }

# Trait 制約
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = a.clone() + b

# Trait 継承
type Serializable = { serialize: (Self) -> String }
type JsonSerializable = Serializable + { to_json: (Self) -> String }

# Trait 実装
impl Clone for Point {
    clone: (self: Point) -> Point = Point { x: self.x, y: self.y }
}
```

---

## フェーズ C1：コア Trait 構文解析

### 目標
`type TraitName = { method: (params) -> return_type }` 構文を解析できること

### ファイル変更

| ファイル | 操作 | 説明 |
|------|------|------|
| `src/frontend/core/parser/ast.rs` | 修改 | `TraitMethod`, `TraitDef` AST ノードを追加 |
| `src/frontend/core/parser/ast.rs` | 修改 | `StmtKind::TraitDef` を追加 |
| `src/frontend/core/parser/statements/trait_def.rs` | 新規 | Trait 定義パーサー |
| `src/frontend/core/parser/statements/mod.rs` | 修改 | 新規モジュールをエクスポート |
| `src/frontend/core/parser/parser_state.rs` | 修改 | 文分派に Trait を追加 |

### 1.1 AST 変更

**ファイル**: `src/frontend/core/parser/ast.rs`

```rust
// ファイル末尾に Trait 関連構造体を追加

/// Trait メソッド定義
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub span: Span,
}

/// Trait 定義
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    /// ジェネリックパラメータリスト
    pub generic_params: Vec<GenericParam>,
    /// Trait メソッドリスト
    pub methods: Vec<TraitMethod>,
    /// 親 Trait リスト（継承用）
    pub parent_traits: Vec<Type>,
    /// Trait 定義の位置
    pub span: Span,
}

/// Trait 実装ブロック
#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_name: String,
    /// 実装対象の型
    pub for_type: Type,
    /// 実装されたメソッド
    pub methods: Vec<MethodImpl>,
    pub span: Span,
}

/// Trait メソッド実装
#[derive(Debug, Clone)]
pub struct MethodImpl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
    pub span: Span,
}

// StmtKind 列挙型を変更
pub enum StmtKind {
    // ... 既存のバリアント ...

    /// Trait 定義: `type TraitName = { ... }`
    TraitDef(TraitDef),

    /// Trait 実装: `impl TraitName for Type { ... }`
    TraitImpl(TraitImpl),
}
```

### 1.2 パーサー新規作成

**ファイル**: `src/frontend/core/parser/statements/trait_def.rs`

```rust
//! Trait 定義と実装の解析

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::{ParserState, ParseError};
use crate::util::span::Span;

/// Trait 定義文かどうか検出
/// パターン: `type Identifier = { ... }`
fn is_trait_def_stmt(state: &mut ParserState<'_>) -> bool {
    if !matches!(
        state.current().map(|t| &t.kind),
        Some(TokenKind::KwType)
    ) {
        return false;
    }

    let saved = state.save_position();
    state.bump(); // consume `type`

    let is_trait = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump(); // consume identifier

        // `=`かどうかチェック（他の演算子ではなく）
        state.at(&TokenKind::Eq)
    } else {
        false
    };

    state.restore_position(saved);
    is_trait
}

/// Trait 実装文かどうか検出
/// パターン: `impl Identifier for Type { ... }`
fn is_trait_impl_stmt(state: &mut ParserState<'_>) -> bool {
    if !matches!(
        state.current().map(|t| &t.kind),
        Some(TokenKind::KwImpl)
    ) {
        return false;
    }

    let saved = state.save_position();
    state.bump(); // consume `impl`

    let is_impl = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump(); // consume trait name

        // `for` キーワードかどうかチェック
        state.at(&TokenKind::KwFor)
    } else {
        false
    };

    state.restore_position(saved);
    is_impl
}

/// Trait 定義を解析: `type TraitName = { method: (params) -> ret }`
pub fn parse_trait_def_stmt(
    state: &mut ParserState<'_>,
    start_span: Span,
) -> Option<Stmt> {
    // consume `type`
    state.bump();

    // Trait 名を解析
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected trait name after 'type'".to_string(),
            ));
            return None;
        }
    };

    let name_span = state.span();

    // ジェネリックパラメータを解析（任意）
    let generic_params = if state.at(&TokenKind::LBracket) {
        parse_trait_generic_params(state)?
    } else {
        vec![]
    };

    // `=` を期待
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // `{` を期待
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    let methods_span = state.span();

    // メソッドリストを解析
    let mut methods = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        // セミコロンをスキップ
        state.skip(&TokenKind::Semicolon);

        if state.at(&TokenKind::RBrace) {
            break;
        }

        // メソッド定義を解析
        if let Some(method) = parse_trait_method(state) {
            methods.push(method);
        } else {
            // 解析失敗、回復してスキップ
            state.synchronize();
        }

        // セミコロンをスキップ（区切り文字）
        state.skip(&TokenKind::Semicolon);
    }

    // `}` を期待
    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    let end_span = state.span();

    Some(Stmt {
        kind: StmtKind::TraitDef(TraitDef {
            name,
            generic_params,
            methods,
            parent_traits: vec![], // 継承は当面サポート外
            span: start_span.merge(&end_span),
        }),
        span: start_span,
    })
}

/// Trait ジェネリックパラメータを解析
fn parse_trait_generic_params(state: &mut ParserState<'_>) -> Option<Vec<GenericParam>> {
    // `[` を期待
    if !state.expect(&TokenKind::LBracket) {
        return None;
    }

    let mut params = Vec::new();

    while !state.at(&TokenKind::RBracket) && !state.at_end() {
        // ジェネリックパラメータを解析: `T` または `T: Trait`
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                state.bump();
                name
            }
            _ => {
                state.error(ParseError::Message(
                    "Expected generic parameter name".to_string(),
                ));
                return None;
            }
        };

        // 制約を解析（任意）
        let mut constraints = Vec::new();
        if state.at(&TokenKind::Colon) {
            state.bump(); // consume `:`
            // 型を制約として解析
            if let Some(constraint) = parse_trait_type_constraint(state) {
                constraints.push(constraint);
            }
        }

        params.push(GenericParam {
            name,
            constraints,
        });

        // コンマをスキップ
        state.skip(&TokenKind::Comma);
    }

    // `]` を期待
    if !state.expect(&TokenKind::RBracket) {
        return None;
    }

    Some(params)
}

/// Trait 型制約を解析
fn parse_trait_type_constraint(state: &mut ParserState<'_>) -> Option<Type> {
    // 簡略実装：単一の識別子のみ解析
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            Some(Type::Name(name))
        }
        _ => {
            state.error(ParseError::Message(
                "Expected type constraint".to_string(),
            ));
            None
        }
    }
}

/// Trait メソッド定義を解析
fn parse_trait_method(state: &mut ParserState<'_>) -> Option<TraitMethod> {
    let start_span = state.span();

    // メソッド名を解析
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected method name in trait".to_string(),
            ));
            return None;
        }
    };

    // `(` を期待
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    // パラメータリストを解析
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let param = parse_trait_method_param(state)?;
        params.push(param);

        // コンマをスキップ
        state.skip(&TokenKind::Comma);
    }

    // `)` を期待
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // 戻り値型を解析（任意）
    let return_type = if state.at(&TokenKind::Arrow) {
        state.bump(); // consume `->`
        parse_trait_return_type(state)?
    } else {
        None
    };

    let end_span = state.span();

    Some(TraitMethod {
        name,
        params,
        return_type,
        span: start_span.merge(&end_span),
    })
}

/// Trait メソッドパラメータを解析
fn parse_trait_method_param(state: &mut ParserState<'_>) -> Option<Param> {
    let start_span = state.span();

    // 最初のパラメータは `self` または `self: Type` の可能性
    if let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
        if name == "self" || name == "Self" {
            let self_name = name.clone();
            state.bump();

            // 型注釈があるかどうかチェック
            if state.at(&TokenKind::Colon) {
                state.bump(); // consume `:`
                let ty = parse_trait_return_type(state)?;
                return Some(Param {
                    name: self_name,
                    ty: Some(ty),
                    span: start_span.merge(&state.span()),
                });
            }

            // self のデフォルト型は Self
            return Some(Param {
                name: self_name,
                ty: Some(Type::Name("Self".to_string())),
                span: start_span.merge(&state.span()),
            });
        }
    }

    // 通常のパラメータを解析: `name: Type`
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected parameter name".to_string(),
            ));
            return None;
        }
    };

    // `:` を期待
    if !state.expect(&TokenKind::Colon) {
        return None;
    }

    // 型を解析
    let ty = parse_trait_return_type(state)?;

    Some(Param {
        name,
        ty: Some(ty),
        span: start_span.merge(&state.span()),
    })
}

/// 戻り値型を解析
fn parse_trait_return_type(state: &mut ParserState<'_>) -> Option<Type> {
    // 簡略実装：識別子と Fn 型のみ解析
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(_)) => {
            // 識別子またはジェネリック型の可能性
            let name = if let Some(TokenKind::Identifier(n)) = state.current().map(|t| &t.kind) {
                n.clone()
            } else {
                return None;
            };
            state.bump();

            // ジェネリック型 `<T>` かどうかチェック
            if state.at(&TokenKind::LAngle) {
                state.bump(); // consume `<`
                let mut args = Vec::new();
                while !state.at(&TokenKind::RAngle) && !state.at_end() {
                    if let Some(arg) = parse_trait_return_type(state) {
                        args.push(arg);
                    }
                    state.skip(&TokenKind::Comma);
                }
                state.expect(&TokenKind::RAngle)?;
                return Some(Type::Generic { name, args });
            }

            Some(Type::Name(name))
        }
        Some(TokenKind::LParen) => {
            // 関数型: `(T1, T2) -> T`
            state.bump(); // consume `(`
            let mut params = Vec::new();
            while !state.at(&TokenKind::RParen) && !state.at_end() {
                if let Some(ty) = parse_trait_return_type(state) {
                    params.push(ty);
                }
                state.skip(&TokenKind::Comma);
            }
            state.expect(&TokenKind::RParen)?;

            // `->` を期待
            state.expect(&TokenKind::Arrow)?;

            let ret = parse_trait_return_type(state)?;

            Some(Type::Fn {
                params,
                return_type: Box::new(ret),
            })
        }
        Some(TokenKind::KwVoid) => {
            state.bump();
            Some(Type::Void)
        }
        _ => {
            state.error(ParseError::Message(
                "Expected return type".to_string(),
            ));
            None
        }
    }
}

/// Trait 実装を解析: `impl TraitName for Type { ... }`
pub fn parse_trait_impl_stmt(
    state: &mut ParserState<'_>,
    start_span: Span,
) -> Option<Stmt> {
    // consume `impl`
    state.bump();

    // Trait 名を解析
    let trait_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected trait name after 'impl'".to_string(),
            ));
            return None;
        }
    };

    // `for` を期待
    if !state.expect(&TokenKind::KwFor) {
        return None;
    }

    // 実装対象の型を解析
    let for_type = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            Type::Name(name)
        }
        _ => {
            state.error(ParseError::Message(
                "Expected type after 'for'".to_string(),
            ));
            return None;
        }
    };

    // `{` を期待
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    // メソッド実装を解析
    let mut methods = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(method) = parse_trait_method_impl(state) {
            methods.push(method);
        } else {
            state.synchronize();
        }
        state.skip(&TokenKind::Semicolon);
    }

    // `}` を期待
    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    let end_span = state.span();

    Some(Stmt {
        kind: StmtKind::TraitImpl(TraitImpl {
            trait_name,
            for_type,
            methods,
            span: start_span.merge(&end_span),
        }),
        span: start_span,
    })
}

/// Trait メソッド実装を解析
fn parse_trait_method_impl(state: &mut ParserState<'_>) -> Option<MethodImpl> {
    let start_span = state.span();

    // メソッド名を解析
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected method name".to_string(),
            ));
            return None;
        }
    };

    // `(` を期待
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    // パラメータリストを解析
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let param = parse_trait_method_param(state)?;
        params.push(param);
        state.skip(&TokenKind::Comma);
    }

    // `)` を期待
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // 戻り値型を解析（任意）
    let return_type = if state.at(&TokenKind::Arrow) {
        state.bump();
        parse_trait_return_type(state)?
    } else {
        None
    };

    // `=` を期待
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // メソッド本体を解析
    let body = if state.at(&TokenKind::LBrace) {
        // ブロックを関数本体として
        let block = parse_trait_method_body(state)?;
        (block.stmts, block.expr)
    } else {
        // 簡略式を関数本体として
        let expr = state.parse_expression(ParserState::BP_LOWEST);
        (Vec::new(), expr.map(Box::new))
    };

    let end_span = state.span();

    Some(MethodImpl {
        name,
        params,
        return_type,
        body,
        span: start_span.merge(&end_span),
    })
}

/// メソッド本体ブロックを解析
fn parse_trait_method_body(state: &mut ParserState<'_>) -> Option<Block> {
    // 既存のブロック解析ロジックを使用
    // ここでは既存の parse_block または同様の関数を参照する必要がある
    // 簡略実装：空のブロックを作成
    let start_span = state.span();

    state.expect(&TokenKind::LBrace)?;

    let mut stmts = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(stmt) = state.parse_statement() {
            stmts.push(stmt);
        } else {
            state.bump();
        }
    }

    state.expect(&TokenKind::RBrace)?;

    let end_span = state.span();

    Some(Block {
        stmts,
        expr: None,
        span: start_span.merge(&end_span),
    })
}
```

### 1.3 モジュールエクスポート更新

**ファイル**: `src/frontend/core/parser/statements/mod.rs`

```rust
//! 文解析モジュール
//! 異なる文タイプ用の専用モジュールを含む

pub mod bindings;
pub mod control_flow;
pub mod declarations;
pub mod types;
pub mod trait_def;  // 新規追加

// よく使うアイテムをリエクスポート
pub use types::*;
pub use declarations::*;
pub use control_flow::*;
pub use bindings::*;
pub use trait_def::*;  // 新規追加
```

**ファイル**: `src/frontend/core/parser/statements/mod.rs` (StatementParser 実装)

```rust
impl StatementParser for ParserState<'_> {
    fn parse_statement(&mut self) -> Option<Stmt> {
        let start_span = self.span();

        match self.current().map(|t| &t.kind) {
            // ... 既存の分岐 ...

            // Trait 定義
            Some(TokenKind::KwType) => {
                if is_trait_def_stmt(self) {
                    trait_def::parse_trait_def_stmt(self, start_span)
                } else {
                    declarations::parse_type_stmt(self, start_span)
                }
            }

            // Trait 実装
            Some(TokenKind::KwImpl) => trait_def::parse_trait_impl_stmt(self, start_span),

            // ... 残りの分岐 ...
        }
    }
}
```

### 1.4 TokenKind 追加

**関連 Token が既にあるか確認**：

```rust
// lexer/tokens.rs で以下の Token が存在することを確認する必要がある：
// - KwType
// - KwImpl
// - KwFor
// - KwSelf / Self
```

### 1.5 受入テスト

```yaoxiang
# test_trait_def.yaoxiang

# 基本 Trait 定義
type Clone = {
    clone: (self: Self) -> Self
}

# ジェネリック Trait
type Container[T] = {
    get: (self: Self) -> T
}

# マルチメソッド Trait
type Add = {
    add: (self: Self, other: Self) -> Self
    zero: (Self) -> Self
}
```

---

## フェーズ C2：Trait 境界表現と制約解決 ✅ 完了

### 目標
`[T: Trait]` 制約の解析と検証を実現

### ファイル変更

| ファイル | 操作 | 説明 |
|------|------|------|
| `src/frontend/type_level/trait_bounds.rs` | 新規 | Trait 境界データ構造 |
| `src/frontend/type_level/mod.rs` | 修改 | trait_bounds モジュールをエクスポート |
| `src/frontend/typecheck/mod.rs` | 修改 | TypeEnvironment に Trait テーブルを追加 |

### 2.1 Trait 境界データ構造

**ファイル**: `src/frontend/type_level/trait_bounds.rs`

実装済み：

- `TraitMethodSignature` - Trait メソッド署名
- `TraitDefinition` - Trait 定義
- `TraitBound` - Trait 境界（ジェネリック制約用）
- `TraitTable` - Trait テーブル、全解析済み Trait 定義と実装を格納
- `TraitImplementation` - Trait 実装
- `TraitSolver` - Trait 制約解決器
- `TraitSolverError` - 解決エラー型

### 2.2 型環境拡張

**ファイル**: `src/frontend/typecheck/mod.rs`

追加済み：

- `trait_table: TraitTable` フィールドを `TypeEnvironment` に追加
- `add_trait()`, `get_trait()`, `has_trait()` メソッド
- `add_trait_impl()`, `has_trait_impl()`, `get_trait_impl()` メソッド

---

## フェーズ C3：Trait 継承 ✅ 完了

### 目標
`type Trait = Parent { ... }` 構文をサポート

### ファイル変更

| ファイル | 操作 | 説明 |
|------|------|------|
| `src/frontend/type_level/inheritance.rs` | 新規 | 継承解析と検証 |
| `src/frontend/type_level/mod.rs` | 修改 | 継承モジュールをエクスポート |

### 3.1 継承チェッカー

**ファイル**: `src/frontend/type_level/inheritance.rs`

実装済み：

- `TraitInheritanceGraph` - Trait 継承グラフ
- `InheritanceChecker` - 継承チェッカー
- `InheritanceError` - 継承エラー型

機能：

- 親 Trait が定義済みかを検証
- 循環継承を検出
- 必須メソッドを全て収集（親 Trait から継承したものを含む）
- 多重継承をサポート `type Trait = A + B + C {}`

---

## フェーズ C4：Trait 実装検査 ✅ 完了

### 目標
`impl Trait for Type { ... }` が正しく実装されているかを検証

### ファイル変更

| ファイル | 操作 | 説明 |
|------|------|------|
| `src/frontend/type_level/impl_check.rs` | 新規 | 実装検証 |
| `src/frontend/type_level/mod.rs` | 修改 | 実装検査モジュールをエクスポート |

### 4.1 実装チェッカー

**ファイル**: `src/frontend/type_level/impl_check.rs`

実装済み：

- `TraitImplChecker` - Trait 実装チェッカー
- `TraitImplError` - 実装エラー型

機能：

- Trait 定義が存在することを検証
- 必須メソッドを全て収集（継承したものを含む）
- 必須メソッドが実装されているかをチェック
- メソッド署名が互換であることを検証
- 重複実装をチェック（coherence）

---

## フェーズ C5：高度な機能 ✅ 完了

### 目標

- Derive マクロ
- デフォルト実装
- 静的メソッド

### ファイル変更

| ファイル | 操作 | 説明 |
|------|------|------|
| `src/frontend/type_level/derive.rs` | 新規 | Derive マクロサポート |
| `src/frontend/type_level/mod.rs` | 修改 | Derive モジュールをエクスポート |

### 5.1 Derive サポート

**ファイル**: `src/frontend/type_level/derive.rs`

実装済み：

- `DeriveParser` - Derive 属性パーサー
- `DeriveGenerator` - Derive コード生成器
- `DeriveImpl` - 内蔵派生実装（Clone, Copy）

機能：

- `#[derive(Clone, Copy)]` 属性を解析
- Trait 実装を自動生成
- 内蔵 Clone/Copy 派生をサポート

---

## 受入基準

### C1：構文解析

- [x] `type TraitName = { ... }` 構文を解析できる
- [x] ジェネリック Trait を解析できる：`type Container[T] = { ... }`
- [x] マルチメソッド Trait を解析できる
- [x] `[T: Trait]` 制約構文を解析できる

### C2：制約解決

- [x] 型が Trait 制約を満たすかを検証
- [x] 複数制約 `[T: A + B]` をサポート
- [x] 制約解決エラー情報が明確

### C3：継承

- [x] `type Trait = Parent { ... }` を解析できる
- [x] 継承チェーンに循環がないことを検証
- [x] 子 Trait は親 Trait メソッドを自動継承

### C4：実装検査

- [x] `impl Trait for Type { ... }` を解析できる
- [x] 実装が全ての必須メソッドを含むかを検証
- [x] メソッド署名が互換であることを検証
- [x] エラーメッセージが不足メソッドを指し示す

### C5：高度な機能

- [x] `#[derive(Trait)]` 構文をサポート
- [x] デフォルトメソッド実装をサポート
- [x] `Trait::method()` 静的呼び出しをサポート

---

## テストケース

### 基本機能テスト

```yaoxiang
# test_basic_trait.yaoxiang

# 1. 基本 Trait 定義
type Clone = {
    clone: (self: Self) -> Self
}

# 2. マルチメソッド Trait
type Add = {
    add: (self: Self, other: Self) -> Self
    zero: (Self) -> Self
}

# 3. ジェネリック Trait
type Container[T] = {
    get: (self: Self) -> T
    set: (self: Self, value: T) -> Void
}

# 4. 制約を使用
clone: [T: Clone](value: T) -> T = value.clone()

# 5. 複数制約
combine: [T: Clone + Add](a: T, b: T) -> T = a.add(a.clone(), b)
```

### 継承テスト

```yaoxiang
# test_trait_inheritance.yaoxiang

type Serializable = {
    serialize: (self: Self) -> String
}

type JsonSerializable = Serializable + {
    to_json: (self: Self) -> String
}

# 子 Trait は Serializable のメソッドを自動継承
```

### 実装テスト

```yaoxiang
# test_trait_impl.yaoxiang

type Clone = {
    clone: (self: Self) -> Self
}

type Point = { x: Int, y: Int }

impl Clone for Point {
    clone: (self: Point) -> Point = Point { x: self.x, y: self.y }
}
```

---

## 付録：参考リソース

### 関連ファイル

- `src/frontend/core/parser/ast.rs` - AST 定義
- `src/frontend/core/parser/statements/` - 文解析
- `src/frontend/typecheck/traits/` - Trait 関連検査
- `src/frontend/type_level/` - 型レベル計算

### 参考ドキュメント

- [RFC-011 ジェネリクスシステム設計](../accepted/011-generic-type-system.md)
- Rust Trait システムドキュメント
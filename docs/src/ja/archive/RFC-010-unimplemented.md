# RFC-010 統一型構文 - 未実装機能ドキュメント

> **作成日**: 2026-02-03
> **ステータス**: 未実装
> **RFC ベース**: RFC-010 統一型構文

## 概要

本文書は、RFC-010 統一型構文设计中尚未実装または実装が完了していない部分について记载しており、今後の開発参考资料として 제공한다。

---

## 1. メソッドバインディング構文解析

### 1.1 問題説明

RFC-010 では `Type.method: (Type, ...) -> ReturnType = ...` メソッド定義構文を設計しているが、パーサーは現在この構文をサポートしていない。

**期待される構文**：
```yaoxiang
# 型メソッド定義
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}
```

**現在の状態**：
- AST には `MethodBind` ノード定義が存在 (`src/frontend/core/parser/ast.rs:184-195`)
- パーサー `declarations.rs` に対応する構文解析ロジックが欠落

### 1.2 必要な修正

#### 1.2.1 `parse_type_annotation` の修正または新規解析関数の追加

`src/frontend/core/parser/statements/declarations.rs` にメソッドバインディング構文認識を追加する：

```rust
/// メソッドバインディング構文かを検出: `Type.method: (Params) -> ReturnType`
fn is_method_bind_syntax(state: &mut ParserState<'_>) -> bool {
    let saved = state.save_position();

    // 型名とメソッド名をドットで区切ったパターンをチェック
    // 例: Point.draw: (Point, Surface) -> Void = ...
    let has_dot_method = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump();
        state.at(&TokenKind::Dot)
    } else {
        false
    };

    state.restore_position(saved);
    has_dot_method
}
```

#### 1.2.2 メソッドバインディング解析関数の新規追加

```rust
/// メソッドバインディングを解析: `Type.method: (Params) -> ReturnType = (params) => body`
pub fn parse_method_bind_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // 型名を解析
    let type_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    // ドットを消費
    state.expect(&TokenKind::Dot)?;

    // メソッド名を解析
    let method_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    // コロンを消費
    state.expect(&TokenKind::Colon)?;

    // メソッド型を解析
    let method_type = parse_type_annotation(state)?;

    // 等号を消費
    state.expect(&TokenKind::Eq)?;

    // メソッド本体を解析
    let body = parse_fn_body(state)?;

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::MethodBind {
            type_name,
            method_name,
            method_type,
            params: body.0,
            body: body.1,
        },
        span,
    })
}
```

### 1.3 テストケース

```rust
#[test]
fn test_method_bind_parsing() {
    let code = r#"
        Point.draw: (Point, Surface) -> Void = (self, surface) => {
            surface.plot(self.x, self.y)
        }
    "#;

    let ast = parse(code).unwrap();
    assert!(matches!(
        ast.items[0].kind,
        StmtKind::MethodBind {
            type_name: ref n,
            method_name: ref m,
            ..
        } if n == "Point" && m == "draw"
    ));
}
```

---

## 2. pub 自動バインディング機構

### 2.1 問題説明

RFC-010 では `pub` 自動バインディング機構を設計している：関数が `pub` で宣言された場合、コンパイラーは自動的に同じファイルに定義された型にバインドする必要がある。

**期待される動作**：
```yaoxiang
# pub を使用して宣言すると、コンパイラーが Point.distance に自動バインド
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 以下と同等：
Point.distance = distance[0]

# 呼び出し方法
d1 = distance(p1, p2)      # 関数型
d2 = p1.distance(p2)       # OOP 糖衣構文
```

**現在の状態**：関連実装なし

### 2.2 必要な修正

#### 2.2.1 パーサーで pub 関数を認識させる修正

`src/frontend/core/parser/statements/declarations.rs` の `parse_identifier_stmt` 関数で：

```rust
/// 識別子で始まるステートメントを解析
pub fn parse_identifier_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // pub 宣言かをチェック
    let is_pub = state.skip(&TokenKind::KwPub);

    // 今後のロジック...

    // 返り時に pub ステータスをマーク
    Some(Stmt {
        kind: StmtKind::Fn {
            name,
            type_annotation,
            params,
            body,
            is_pub,  // 新規フィールド
        },
        span,
    })
}
```

#### 2.2.2 新規 AST フィールドの追加

`src/frontend/core/parser/ast.rs` の `StmtKind::Fn` を修正：

```rust
/// 関数定義: `name: Type = (params) => body`
pub struct FnStmt {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub params: Vec<Param>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
    pub is_pub: bool,  // 新規：型に自動バインドするかどうか
    pub auto_bind_type: Option<String>,  // 新規：自動バインドのターゲット型
}
```

#### 2.2.3 型チェック段階で自動バインディングを実装

`src/frontend/typecheck/inference/statements.rs` で：

```rust
/// 関数定義を処理し、pub 自動バインディングをサポート
fn infer_fn_stmt(
    &mut self,
    stmt: &Stmt,
    env: &mut TypeEnvironment,
) -> TypeResult<MonoType> {
    match &stmt.kind {
        StmtKind::Fn { name, params, return_type, body, is_pub, .. } => {
            // 関数型を構築
            let fn_type = self.infer_fn_type(params, return_type.as_ref())?;

            if *is_pub {
                // 同じファイルに定義された型への自動バインドを試行
                if let Some(target_type) = self.find_target_type_for_pub(name, params) {
                    self.bind_method_to_type(&target_type, name, &fn_type)?;
                }
            }

            // 環境に登録
            env.add_var(name.clone(), PolyType::mono(fn_type));

            Ok(MonoType::Void)
        }
        _ => unreachable!(),
    }
}

/// pub 関数がバインドされるターゲット型を查找
fn find_target_type_for_pub(
    &self,
    fn_name: &str,
    params: &[Param],
) -> Option<String> {
    // ルール：最初のパラメータの型名をバインド先に使用
    // 例：distance: (Point, Point) -> Float は Point にバインド
    if let Some(first_param) = params.first() {
        if let Some(ref ty) = first_param.ty {
            return Some(self.type_to_string(ty));
        }
    }
    None
}
```

### 2.3 テストケース

```rust
#[test]
fn test_pub_auto_bind() {
    let code = r#"
        type Point = {
            x: Float,
            y: Float
        }

        pub distance: (Point, Point) -> Float = (p1, p2) => {
            dx = p1.x - p2.x
            dy = p1.y - p2.y
            (dx * dx + dy * dy).sqrt()
        }
    "#;

    let type_env = typecheck(code).unwrap();

    // Point.distance メソッドがバインドされたかをチェック
    let point_type = type_env.get_type("Point").unwrap();
    assert!(point_type.methods.contains_key("distance"));
}
```

---

## 3. ジェネリクス制約構文解析

### 3.1 問題説明

RFC-010 は RFC-011 ジェネリクスシステムとの統合を設計しており、`[T: Constraint]` 制約構文をサポートする。

**期待される構文**：
```yaoxiang
# 制約付きジェネリック関数
clone: [T: Clone](value: T) -> T = value.clone()

# 複数制約（当面 & 構文不支持）
# process: [T: Drawable & Serializable](item: T) -> String = { ... }

# 山括弧構文
identity: <T: Clone>(value: T) -> T = value
```

**現在の状態**：✅ 実装済み

### 3.2 必要な修正

#### 3.2.1 ジェネリックパラメータ解析の修正

`src/frontend/core/parser/statements/declarations.rs` で：

```rust
/// ジェネリックパラメータ構造体
pub struct GenericParam {
    pub name: String,
    pub constraints: Vec<MonoType>,  // 制約リスト
}

/// 制約付きジェネリックパラメータを解析: `[T, U]` or `[T: Clone, U: Serializable]`
pub fn parse_generic_params_with_constraints(
    state: &mut ParserState<'_>,
) -> Option<Vec<GenericParam>> {
    let open = if state.at(&TokenKind::LBracket) {
        state.bump();
        TokenKind::RBracket
    } else if state.at(&TokenKind::Lt) {
        state.bump();
        TokenKind::Gt
    } else {
        return Some(Vec::new());
    };

    let mut params = Vec::new();

    while !state.at(&open) && !state.at_end() {
        // パラメータ名を解析
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        // 制約を解析
        let mut constraints = Vec::new();
        if state.skip(&TokenKind::Colon) {
            loop {
                let constraint = parse_type_annotation(state)?;
                constraints.push(constraint);

                if !state.skip(&TokenKind::Amp) {
                    break;
                }
            }
        }

        params.push(GenericParam { name, constraints });
        state.skip(&TokenKind::Comma);
    }

    if !state.expect(&open) {
        return None;
    }

    Some(params)
}
```

#### 3.2.2 型定義と関数定義の修正

`StmtKind::Fn` にジェネリックパラメータを追加：

```rust
/// ジェネリックパラメータ付き関数定義
pub struct FnStmt {
    pub name: String,
    pub generic_params: Vec<GenericParam>,  // 新規追加
    pub type_annotation: Option<Type>,
    pub params: Vec<Param>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
}
```

#### 3.2.3 型チェックでの制約検証実装

`src/frontend/typecheck/checking/bounds.rs` に追加：

```rust
/// ジェネリックパラメータが制約を満たすかをチェック
pub fn check_generic_param_constraints(
    &self,
    param: &GenericParam,
    arg_type: &MonoType,
) -> Result<(), TypeError> {
    for constraint in &param.constraints {
        if !self.check_constraint(arg_type, constraint)? {
            return Err(TypeError::ConstraintNotSatisfied {
                param_name: param.name.clone(),
                constraint_name: constraint.type_name(),
                arg_type: arg_type.type_name(),
            });
        }
    }
    Ok(())
}
```

### 3.3 テストケース

```rust
#[test]
fn test_generic_constraint_parsing() {
    let code = r#"
        clone: [T: Clone](value: T) -> T = value.clone()
    "#;

    let ast = parse(code).unwrap();
    match &ast.items[0].kind {
        StmtKind::Fn { generic_params, .. } => {
            assert_eq!(generic_params.len(), 1);
            assert_eq!(generic_params[0].name, "T");
            assert_eq!(generic_params[0].constraints.len(), 1);
        }
        _ => panic!("Expected function definition"),
    }
}

#[test]
fn test_generic_constraint_checking() {
    let code = r#"
        type Point = { x: Float, y: Float }

        # Point は Clone を実装していないため、エラーを出すべき
        clone: [T: Clone](value: T) -> T = value.clone()
    "#;

    let result = typecheck(code);
    assert!(result.is_err());
}
```

---

## 4. 完全実装優先順位

| 優先度 | 機能 | 影響範囲 | ステータス |
|--------|------|----------|------------|
| **P0** | メソッドバインディング構文解析 | パーサー | 未実装 |
| **P1** | pub 自動バインディング機構 | パーサー + 型チェック | 未実装 |
| **P2** | ジェネリクス制約構文 | パーサー + 型チェック | ✅ 完了済み |

---

## 5. 関連ファイル一覧

### 5.1 修正が必要なファイル

| ファイルパス | 修正内容 |
|-------------|----------|
| `src/frontend/core/parser/ast.rs` | `GenericParam` 構造体追加、`StmtKind::Fn` に `generic_params` フィールド追加 |
| `src/frontend/core/parser/statements/declarations.rs` | `parse_generic_params_with_constraints` 追加、`parse_var_stmt` 修正、`parse_type_annotation` 拡張 |
| `src/frontend/typecheck/checking/mod.rs` | `generic_params` フィールド整合 |
| `src/frontend/typecheck/inference/statements.rs` | `generic_params` フィールド整合 |
| `src/frontend/typecheck/inference/expressions.rs` | `generic_params` フィールド整合 |
| `src/middle/core/ir_gen.rs` | `generic_params` フィールド整合 |

### 5.2 新規追加が必要なファイル

| ファイルパス | 説明 |
|-------------|------|
| `src/frontend/core/parser/statements/method_bind.rs` | メソッドバインディング解析ロジック（未実装） |
| `src/frontend/typecheck/checking/auto_bind.rs` | 自動バインディング検査ロジック（未実装） |

---

## 6. 検収基準

### 6.1 メソッドバインディング
- [ ] `Type.method: (Params) -> ReturnType = ...` 構文を解析可能
- [ ] AST が正しく `MethodBind` ノードを生成
- [ ] 型チェックが正しくメソッドを型にバインド可能

### 6.2 pub 自動バインディング
- [ ] `pub fn` を正しく認識可能
- [ ] 最初のパラメータの型に自動バインド可能
- [ ] `p.method()` 糖衣構文呼び出しをサポート

### 6.3 ジェネリクス制約
- [x] `[T: Clone]` 構文を解析可能
- [ ] 型チェックが制約充足を検証可能（未実装）
- [ ] エラーメッセージが欠落制約を明確に示す（未実装）
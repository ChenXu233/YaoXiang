---
title: 'RFC 例：パターンマッチング構文の強化'
---

# RFC 例：パターンマッチング構文の強化

> **注意**: これは RFC テンプレート例であり、完全な RFC 提案の書き方を示しています。独自の RFC を作成する際は、このテンプレートを参考にしてください。
>
> **ステータス**: 例（参考のみ）

> **著者**: 晨煦（サンプル著者） **作成日**: 2025-01-05 **最終更新**: 2026-02-12

## 概要

YaoXiang に、より強力なパターンマッチング機能を追加します。ネストパターン、ガード式、`let`
パターンバインディングを含みます。

## 動機

### なぜこの機能が必要なのか？

現在の `match` 式は機能が限定的であり、以下の一般的なシナリオを処理できません：

```yaoxiang
# 无法解构嵌套结构
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }
match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"  # ❌ 不支持
}

# 无法在模式中绑定变量
match result {
    ok(value) => print(value)          # ❌ 需要显式解构
}
```

### 現状の問題点

1. ネストパターンの分解がサポートされていない
2. パターン内でガード式を使用できない
3. `let` 文がパターンマッチングをサポートしていない

## 提案

### 基本設計

`match` 式の構文を拡張し、以下をサポートします：

1. **ネストパターン分解**：任意の深さの構造体分解
2. **ガード式**：パターンの後に `if` 条件を追加
3. **パターン変数バインディング**：パターンから直接変数をバインド

### 例

```yaoxiang
# 嵌套解构
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }

match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"
    Person(name: n, address: Address(city: c, _)) => n + " from " + c
}

# 卫表达式
match n {
    n if n > 0 and n < 10 => "1-9"
    n if n >= 10 => "10+"
    _ => "unknown"
}

# 模式绑定
match result {
    ok(value) => print(value)          # value 已绑定
    err(e) => log_error(e)
}

# 嵌套 + 绑定
match data {
    User(name: first, profile: Profile(age: a)) if a >= 18 => first + " is adult"
}
```

### `let` 文のパターンマッチング

```yaoxiang
# 新语法
let Point(x: 0, y: _) = point  # 仅当 x == 0 时绑定
let Ok(value) = result         # 解构 Result

# 多重绑定
let (a, b, c) = tuple          # 解构元组
```

## 詳細設計

### 構文の変更

```
MatchExpr   ::= 'match' Expr '{' MatchArm+ '}'
MatchArm    ::= Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
Pattern     ::= LiteralPattern
              | IdentifierPattern
              | StructPattern
              | TuplePattern
              | OrPattern
              | RestPattern

LiteralPattern ::= '_' | Literal
IdentifierPattern ::= Identifier (':' Pattern)?
StructPattern ::= Identifier '(' FieldPattern (',' FieldPattern)* ','? ')'
FieldPattern  ::= Identifier ':' Pattern | Identifier
TuplePattern  ::= '(' Pattern (',' Pattern)* ','? ')'
OrPattern     ::= Pattern '|' Pattern
RestPattern   ::= '...'
```

### 型システムへの影響

- パターンマッチングの型チェックを拡張する必要がある
- パターンマッチ成功時、パターン変数は正しい型を取得する

### コンパイラの変更

| コンポーネント | 変更内容                         |
| -------------- | -------------------------------- |
| lexer          | パターン関連トークンを追加       |
| parser         | パターン解析ロジックを追加       |
| typecheck      | パターンの型推論とバインディング |
| codegen        | パターンマッチングのコード生成   |

### 後方互換性

- ✅ 完全な後方互換性
- 新規構文の追加のみで、既存の `match` 構文は変更なし

## トレードオフ

### 利点

- 構文の表現力が向上し、コードがより簡潔になる
- 主流言語のパターンマッチングとの一貫性（Rust、Scala、Elixir）
- ランタイムエラーの削減、マッチしないケースの早期検出

### 欠点

- コンパイラの実装複雑度が増す
- 学習曲線がやや急になる

## 代替案

| 案                             | 採用しない理由                       |
| ------------------------------ | ------------------------------------ |
| トップレベルの分解のみサポート | 一般的なネストシナリオに対応できない |
| 関数型スタイルの使用           | 命令型コードとの混在使用が不自然     |
| v2.0 まで延期                  | ユーザーから強い要望あり             |

## 実装戦略

### 依存関係

- 外部依存なし
- 基本的な型システムの完了が前提

### リスク

- パターンコンパイルの複雑さがパフォーマンス問題を引き起こす可能性
- 過度なネストによるスタックオーバーフローの可能性

## 未解決の問題

1. [ ] 循環パターン（`@` バインディング）の構文は？
2. [ ] コンパイル時のパターンの網羅性チェックをサポートするか？
3. [ ] パフォーマンス最適化戦略は？

## 参考文献

- [Rust パターンマッチング](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala パターンマッチング](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir パターンマッチング](https://elixir-lang.org/getting-started/pattern-matching.html)

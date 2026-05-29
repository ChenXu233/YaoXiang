```yaml
---
title: RFC 例：パターン照合構文の強化
---

# RFC 例：パターン照合構文の強化

> **注意**: これは RFC テンプレート例であり、完全な RFC 提案の書き方を示しています。
> 独自の RFC を書く際に参考してください。
>
> **ステータス**: 例（参考のみ）

> **著者**: 晨煦（例著者）
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-12

## 概要

YaoXiang により強力なパターン照合機能を追加します。ネストされたパターン、ガード式、`let` パターン束縛を含みます。

## 動機

### この機能が必要な理由

現在の `match` 式は機能が限られており、以下の一般的なシナリオに対応できません：

```yaoxiang
# ネストされた構造体を分解できない
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }
match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"  # ❌ サポート外
}

# パターンで変数を束縛できない
match result {
    ok(value) => print(value)          # ❌ 明示的な分解が必要
}
```

### 現在の問題

1. ネストされたパターンの分解不支持
2. パターンでガード式を使用できない
3. `let` 文でパターン照合不支持

## 提案

### コア設計

`match` 式構文を拡張し、以下をサポート：

1. **ネストされたパターンの分解**：任意の深さの構造体分解
2. **ガード式**：パターン後に `if` 条件を追加
3. **パターン変数束縛**：パターンから直接変数を束縛

### 例

```yaoxiang
# ネストされた分解
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }

match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"
    Person(name: n, address: Address(city: c, _)) => n + " from " + c
}

# ガード式
match n {
    n if n > 0 && n < 10 => "1-9"
    n if n >= 10 => "10+"
    _ => "unknown"
}

# パターン束縛
match result {
    ok(value) => print(value)          # value は既に束縛済み
    err(e) => log_error(e)
}

# ネスト + 束縛
match data {
    User(name: first, profile: Profile(age: a)) if a >= 18 => first + " is adult"
}
```

### `let` 文のパターン照合

```yaoxiang
# 新構文
let Point(x: 0, y: _) = point  # x == 0 の場合のみ束縛
let Ok(value) = result         # Result を分解

# 多重束縛
let (a, b, c) = tuple          # タプルを分解
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

- パターン照合の型チェックを拡張する必要がある
- パターン変数は照合成功時に正しい型を取得する

### コンパイラの変更

| コンポーネント | 変更内容 |
|------|------|
| lexer | パターン関連の新しい token 追加 |
| parser | パターン解析ロジック追加 |
| typecheck | パターンの型推論と束縛 |
| codegen | パターン照合コード生成 |

### 後方互換性

- ✅ 完全な後方互換性
- 新構文の追加のみで元の `match` 構文は変更なし

## トレードオフ

### 优点

- 構文がより表現力豊かになり、コードが簡潔になる
- 主流言語のパターン照合と一貫性あり（Rust、Scala、Elixir）
- 実行時エラーを減らし、不一致を早期に検出

### 欠点

- コンパイラの実現複雑性が増加
- 学習曲線がわずかに上昇

## 代替案

| 方案 | 採用しない理由 |
|------|--------------|
| トップレベルの分解のみサポート | 一般的なネストシナリオに対応できない |
| 関数型スタイルを使用 | 命令型コードとの混在が不自然 |
| v2.0 まで遅延 | ユーザーから既に強い需要あり |

## 実装戦略

### 段階分け

1. **段階 1 (v0.6)**: ネストされた分解とガード式
2. **段階 2 (v0.7)**: パターン変数束縛
3. **段階 3 (v0.8)**: `let` パターン照合

### 依存関係

- 外部依存なし
- 先に基礎的な型システムを完了する必要がある

### リスク

- パターンのコンパイル複雑性によりパフォーマンス問題が発生する可能性
- ネストが深すぎる場合スタックオーバーフローの可能性

## 未解決の問題

1. [ ] ループパターン（`@` 束縛）の構文は？
2. [ ] コンパイル時のパターン窮尽チェックをサポートするか？
3. [ ] パフォーマンス最適化戦略は？

## 参考文献

- [Rust パターン照合](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala パターン照合](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir パターン照合](https://elixir-lang.org/getting-started/pattern-matching.html)
```
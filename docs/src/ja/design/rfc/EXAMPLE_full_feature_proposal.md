---
title: "RFC 例: 強化されたパターンマッチング構文"
---

# RFC 例: 強化されたパターンマッチング構文

> **注意**: これは RFC テンプレートの例であり、完全な RFC 提案の書き方を示しています。
> 独自の RFC を作成する際は、このテンプレートを参考にしてください。
>
> **ステータス**: 例(参考のみ)

> **著者**: 晨煦(例示著者)
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-12

## 要約

YaoXiang に、より強力なパターンマッチング機能を追加する。ネストパターン、ガード式、`let` パターンバインディングを含む。

## 動機

### なぜこの機能が必要なのか?

現在の `match` 式は機能が限られており、以下の一般的なシナリオを処理できません。

```yaoxiang
# ネストした構造体を分解できない
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }
match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"  # ❌ サポートされていない
}

# パターン内で変数をバインドできない
match result {
    ok(value) => print(value)          # ❌ 明示的な分解が必要
}
```

### 現在の問題点

1. ネストパターンの分解がサポートされていない
2. パターン内でガード式を使用できない
3. `let` 文がパターンマッチングをサポートしていない

## 提案

### 基本設計

`match` 式の構文を拡張し、以下をサポートする:

1. **ネストパターン分解**: 任意の深さの構造体分解
2. **ガード式**: パターンの後に `if` 条件を追加
3. **パターン変数バインディング**: パターンから直接変数をバインド

### 例

```yaoxiang
# ネスト分解
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

# パターンバインディング
match result {
    ok(value) => print(value)          # value はバインド済み
    err(e) => log_error(e)
}

# ネスト + バインディング
match data {
    User(name: first, profile: Profile(age: a)) if a >= 18 => first + " is adult"
}
```

### `let` 文のパターンマッチング

```yaoxiang
# 新構文
let Point(x: 0, y: _) = point  # x == 0 の場合のみバインド
let Ok(value) = result         # Result を分解

# 複数バインディング
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

- パターンマッチングの型チェックを拡張する必要がある
- パターンの変数は、マッチ成功時に正しい型を取得する

### コンパイラの変更

| コンポーネント | 変更内容 |
|------|------|
| lexer | パターン関連 token を追加 |
| parser | パターン解析ロジックを追加 |
| typecheck | パターンの型推論とバインディング |
| codegen | パターンマッチングのコード生成 |

### 後方互換性

- ✅ 完全な後方互換性を維持
- 構文の追加のみであり、既存の `match` 構文は変更なし

## トレードオフ

### 利点

- 構文の表現力が向上し、コードがより簡潔になる
- 主流言語のパターンマッチング(Rust、Scala、Elixir)と整合性が取れる
- ランタイムエラーを削減し、不一致を早期に検出できる

### 欠点

- コンパイラの実装が複雑になる
- 学習曲線がわずかに上昇する

## 代替案

| 代替案 | 採用しない理由 |
|------|--------------|
| 最上位レベルの分解のみサポート | 一般的なネストシナリオを処理できない |
| 関数型スタイルを採用 | 命令型コードとの混在が不自然になる |
| v2.0 まで延期 | ユーザーから強い需要がある |

## 実装戦略

### 依存関係

- 外部依存なし
- まず基本的な型システムを完成させる必要がある

### リスク

- パターンのコンパイルが複雑になると、パフォーマンス問題が発生する可能性がある
- 過度に深いネストによりスタックオーバーフローが発生する可能性がある

## オープンな問題

1. [ ] 循環パターン(`@` バインディング)の構文は?
2. [ ] コンパイル時のパターンの網羅性チェックをサポートするか?
3. [ ] パフォーマンス最適化の方針は?

## 参考文献

- [Rust パターンマッチング](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala パターンマッチング](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir パターンマッチング](https://elixir-lang.org/getting-started/pattern-matching.html)
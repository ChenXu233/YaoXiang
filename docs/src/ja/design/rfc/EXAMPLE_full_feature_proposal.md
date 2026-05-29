---
title: "RFC 示例：增强模式匹配構文"
---

# RFC 示例：增强パターン匹配構文

> **注意**: これは RFC テンプレート示例であり、完全な RFC 提案の書き方を示しています。
> これを参考にしながらRFCを作成してください。
>
> **ステータス**: 示例（仅供参考）

> **作成者**: 晨煦（示例作成者）
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-12

## 摘要

YaoXiang により強力なパターン照合機能を追加し、ネストされたパターン、衛表現、`let` パターン束縛を含める。

## 動機

### この機能が必要な理由

現在の `match` 式は機能が限られており、以下の一般的なシナリオに対処できません：

```yaoxiang
# ネストされた構造体を分解できない
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }
match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"  # ❌ サポート外
}

# パターン内で変数を束縛できない
match result {
    ok(value) => print(value)          # ❌ 明示的な分解が必要
}
```

### 現在の問題点

1. ネストされたパターンの分解がサポートされていない
2. パターン内で衛表現を使用できない
3. `let` 文がパターン照合をサポートしていない

## 提案

### コア設計

`match` 式の構文を拡張し、以下をサポート：

1. **ネストされたパターン分解**：任意の深さの構造体分解
2. **衛表現**：パターン後に `if` 条件を追加
3. **パターン変数束縛**：パターンから直接変数を束縛

### 示例

```yaoxiang
# ネストされた分解
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }

match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"
    Person(name: n, address: Address(city: c, _)) => n + " from " + c
}

# 衛表現
match n {
    n if n > 0 && n < 10 => "1-9"
    n if n >= 10 => "10+"
    _ => "unknown"
}

# パターン束縛
match result {
    ok(value) => print(value)          # value が既に束縛されている
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

# 複数束縛
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
- パターン変数は照合成功時に正しい型を得る

### コンパイラの改动

| コンポーネント | 改动 |
|------|------|
| lexer | パターン関連の新しいトークン |
| parser | 新しいパターン解析ロジック |
| typecheck | パターンの型推論と束縛 |
| codegen | パターン照合コード生成 |

### 後方互換性

- ✅ 完全な後方互換性
- 新構文の追加のみで、`match` 構文は変更なし

## 权衡

### メリット

- より表現力豊かな構文で、コードが簡潔になる
- 主流言語のパターン照合と一致（Rust、Scala、Elixir）
- 実行時エラーを減らし、不一致を早期に検出

### デメリット

- コンパイラの実装複雑度が増加
- 学習曲線がやや上昇

## 代替案

| 案 | 採用しない理由 |
|------|--------------|
| トップレベルの分解のみサポート | 一般的なネストシナリオに対処できない |
| 関数型スタイルの使用 | 命令型コードとの混在が不自然 |
| v2.0 まで延期 | ユーザーから既に強い需要あり |

## 実装戦略

### 段階的划分

1. **段階 1 (v0.6)**: ネストされた分解と衛表現
2. **段階 2 (v0.7)**: パターン変数束縛
3. **段階 3 (v0.8)**: `let` パターン照合

### 依存関係

- 外部依存なし
- 先に基礎的な型システムを完了する必要がある

### リスク

- パターンのコンパイル複雑度によりパフォーマンス問題が発生する可能性
- ネストが深すぎるとスタックオーバーフローの可能性

## 開放問題

1. [ ] ループパターン（`@` 束縛）の構文は？
2. [ ] コンパイル時のパターン網羅性チェックをサポートするか？
3. [ ] パフォーマンス最適化戦略は？

## 参考文献

- [Rust パターン照合](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala パターン照合](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir パターン照合](https://elixir-lang.org/getting-started/pattern-matching.html)
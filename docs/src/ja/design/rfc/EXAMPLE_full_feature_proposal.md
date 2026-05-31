---
title: "RFC 示例：增强モードマッチング構文"
---

# RFC 示例：增强模式匹配语法

> **注意**: これは RFC テンプレート示例であり、完全な RFC 提案の書き方を示しています。
> 請参考此模板来编写您自己的 RFC。
>
> **状態**: 示例（仅供参考）

> **作者**: 晨煦（示例作者）
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-12

## 摘要

YaoXiang により強力なモードマッチング機能を追加します。ネストされたパターン、ガード式、`let` パターン束縛を含みます。

## 動機

### なぜこの機能が必要か？

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

1. ネストされたパターン分解がサポートされていない
2. パターン内でガード式を使用できない
3. `let` 文がモードマッチングをサポートしていない

## 提案

### コア設計

`match` 式の構文を拡張し、以下をサポートします：

1. **ネストされたパターン分解**：任意の深さの構造体分解
2. **ガード式**：パターン後に `if` 条件を追加
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

# ガード式
match n {
    n if n > 0 && n < 10 => "1-9"
    n if n >= 10 => "10+"
    _ => "unknown"
}

# パターン束縛
match result {
    ok(value) => print(value)          # value が束縛済み
    err(e) => log_error(e)
}

# ネスト + 束縛
match data {
    User(name: first, profile: Profile(age: a)) if a >= 18 => first + " is adult"
}
```

### `let` 文のモードマッチング

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

- モードマッチングの型チェックを拡張する必要がある
- パターンの変数は照合成功時に正しい型を取得する

### コンパイラの更改

| コンポーネント | 更改内容 |
|------|------|
| lexer | パターン関連トークンの新規追加 |
| parser | パターン解析ロジックの新規追加 |
| typecheck | パターンの型推論と束縛 |
| codegen | モードマッチングコード生成 |

### 後方互換性

- ✅ 完全な後方互換性
- 新構文の追加のみで、元の `match` 構文は変更なし

## 权衡

### 优点

- 構文の表現力が向上し、コードがより簡潔になる
- 主流言語のモードマッチングと一貫性を保つ（Rust、Scala、Elixir）
- 実行時エラーを減らし、不一致を早期に検出

### 缺点

- コンパイラの実装複雑度が増加
- 学習曲線がやや上昇

## 代替方案

| 方案 | 为什么不选择 |
|------|--------------|
| トップレベルの分解のみサポート | 一般的なネストシナリオに対処できない |
| 関数型スタイルを使用 | 命令型コードと混在させると不自然 |
| v2.0 まで延期 | ユーザーからの強い需求がすでにある |

## 実装策略

### 段階的アプローチ

1. **段階 1 (v0.6)**: ネストされた分解とガード式
2. **段階 2 (v0.7)**: パターン変数束縛
3. **段階 3 (v0.8)**: `let` モードマッチング

### 依存関係

- 外部依存なし
- 先に基礎的な型システムを完了する必要がある

### リスク

- パターンコンパイルの複雑さによりパフォーマンス問題が発生する可能性がある
- 過度なネストはスタックオーバーフローを起こす可能性がある

## 開放問題

1. [ ] 循環パターン（`@` 束縛）の構文は？
2. [ ] コンパイル時のパターン網羅性チェックをサポートするか？
3. [ ] パフォーマンス最適化戦略は？

## 参考文献

- [Rust モードマッチング](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala モードマッチング](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir モードマッチング](https://elixir-lang.org/getting-started/pattern-matching.html)
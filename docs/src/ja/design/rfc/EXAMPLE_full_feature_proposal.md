---
title: 'RFC 例：パターン照合の構文強化'
---

# RFC 例：パターン照合の構文強化

> **注意**: これは RFC テンプレート例であり、完全な RFC 提案の書き方を示しています。このテンプレートを避けて您自身の RFC を作成してください。
>
> **状態**: 例（参考用）

> **作者**: 晨煦（サンプル作者） **作成日**: 2025-01-05 **最終更新**: 2026-02-12

## 概要

YaoXiang に、より強力なパターン照合機能を追加します。ネストされたパターン、ガード式、`let` パターン束縛を含みます。

## 動機

### なぜこの機能が必要か？

現在の `match` 式は機能が限られており、以下の一般的なシナリオに対応できません：

```yaoxiang
# ネストされた構造を分解できない
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

### 現在の問題

1. ネストされたパターンの分解がサポートされていない
2. パターン内でガード式を使用できない
3. `let` 文がパターン照合をサポートしていない

## 提案

### コア設計

`match` 式の構文を拡張し、以下をサポートします：

1. **ネストされたパターンの分解**: 任意の深さの構造体の分解
2. **ガード式**: パターンの後に `if` 条件を追加
3. **パターン変数束縛**: パターンから直接変数を束縛

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
    ok(value) => print(value)          # value が束縛済み
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

# 複数の束縛
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

- パターン照合の型検査を拡張する必要がある
- パターン変数は照合成功時に正しい型を得る

### コンパイラの変更

| コンポーネント | 変更点               |
| --------- | ------------------ |
| lexer     | パターン関連の新しいトークン |
| parser    | パターン解析の新しいロジック |
| typecheck | パターンの型推論と束縛 |
| codegen   | パターン照合コード生成   |

### 後方互換性

- ✅ 完全な後方互換性
- 新構文のみ追加し、既存の `match` 構文は変更なし

## トレードオフ

### 利点

- より表現力豊かな構文、より簡潔なコード
- 主流言語のパターン照合と一貫性を保つ（Rust、Scala、Elixir）
- 実行時エラーを減らし、不一致を早期に検出

### 欠点

- コンパイラ実装の複雑さが増す
- 学習曲線がやや上昇

## 代替案

| 方案           | なぜ選択しなかったか           |
| -------------- | ---------------------- |
| トップレベルの分解のみサポート | 一般的なネストシナリオを処理できない   |
| 関数型スタイルを使用      | 命令型コードと混在させると不自然     |
| v2.0 まで遅延        | ユーザーから強い需要がすでにある     |

## 実装戦略

### 依存関係

- 外部依存なし
- 先に基礎型システムを完了する必要がある

### リスク

- パターンコンパイルの複雑さがパフォーマンス問題を引き起こす可能性
- ネストが深すぎるとスタックオーバーフローの可能性

## 未解決の問題

1. [ ] ループパターン（`@` 束縛）の構文は？
2. [ ] コンパイル時のパターン網羅性チェック支持的か？
3. [ ] パフォーマンス最適化戦略は？

## 参考文献

- [Rust パターン照合](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala パターン照合](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir パターン照合](https://elixir-lang.org/getting-started/pattern-matching.html)

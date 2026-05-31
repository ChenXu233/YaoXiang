---
title: "RFC-004：カリー化メソッドの位置指定バインディング設計"
---

# RFC-004: カリー化メソッドの位置指定バインディング設計

> **ステータス**: 承認済み
> **著者**: 晨煦
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-18（組み込みバインディング、後置バインディング構文の追加）

## 概要

本 RFC は、関数を型の任意のパラメータ位置に精确にバインディングできる新しい**位置指定バインディング**構文を提案する。単一位置バインディングと複数位置バインディングをサポートし、カリー化バインディングにおける「誰が呼び出し元か」という問題を根本的に解決し、`self` キーワードを導入する必要がない。

## 動機

### なぜこの機能が必要か？

現在の言語設計では、独立関数を型メソッドとしてバインディングする際に以下の問題が発生している：

1. **呼び出し元の位置が不柔軟**：従来のバインディングでは `obj.method(args)` の `obj` を 첫 번째 引数に固定只能
2. **複数パラメータのバインディングが困難**：メソッドが同じ型の複数のパラメータを受け取る必要がある場合、优美に表現できない
3. **カリー化意味論の曖昧さ**：部分適用時に「どの位置にバインディングされているか」の区別が難しい

### 設計目標：関数型と OOP の両方のプログラミング視点を統一

本設計は**関数型と OOP の 두 가지 プログラミング視点を統一する**ことを目指す：

```yaoxiang
# 関数視点：すべての引数を明示的に渡す
distance(p1, p2)

# OOP 視点：暗黙の this
p1.distance(p2)

# [positions] 構文糖衣により 두 가지 書き方は同等、本質的にはどちらも関数呼び出し
Point.distance = distance[0]   # this を位置 0 にバインディング
```

**核心的な価値**：
- 基盤は関数、上層はメソッド構文
- `self` キーワードを導入せず、言語のシンプルさを維持
- 完全に関数化：メソッド呼び出しの本質は引数渡し
- `[0]`, `[1]`, `[-1]` で this バインディング位置を柔軟に制御
- **構文の統一**：関数定義は `name: (params) -> Return = body` 形式を使用

### 現在の問題

```yaoxiang
# 既存設計の問題：
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# 第一个引数에만 바인딩 가능
Point.distance = distance  # distance[0] 와 동등
# p1.distance(p2) → distance(p1, p2) ✓

# 그러나 transform 의 시그니처가 transform(Vector, Point) 인 경우는?
# p1.transform(v1) → transform(v1, p1) 의미론을 표현할 수 없음
```

## 提案

### 核心設計：デフォルトバインディング + 任意の位置指定

#### 最初の一致する位置にデフォルトバインディング

**デフォルト動作**：`Type.method = function` は自動的に 해당 型と一致する最初のパラメータ位置を探してバインディング

```yaoxiang
# デフォルトバインディング：最初の一致する型位置
Point.distance = distance           # コンパイラが自動的に最初の Point パラメータ位置を検索
p1.distance(p2)                     # → distance(p1, p2)

# 関数に 2 つの Point パラメータがある場合、最初の一致する位置にバインディング
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}
# バインディング：Point.distance = distance
# 呼び出し：p1.distance(p2) → distance(p1, p2) ✓

# 特殊位置（最初の一致以外）が必要な場合のみ明示的に指定
Point.compare = distance[1]        # 2 番目の Point パラメータにバインディング
p1.compare(p2)                    # → distance(p2, p1)
```

**バインディング失敗時の処理**：
- **一致する型が見つからない**：関数パラメータに該当型がない場合、エラーまたは警告
- **ファクトリ関数パターン**：パラメータが一致しない場合、ファクトリ関数として使用

```yaoxiang
# ケース1：一致する型が見つからない
create_point: () -> Point = { ... }
Point.create = create_point        # エラー：Point 型パラメータがない

# ケース2：ファクトリ関数パターン（オプション）
Point.create = create_point        # ファクトリ関数として、呼び出し：Point.create()
```

**利点**：
- スマートバインディング：型に基づいて自動的に一致、直感に従う
- 型安全性：型が一致する場合のみバインディング、エラーを回避
- 柔軟な制御：デフォルトバインディングが期待する動作でない場合、任意に位置を指定可能

#### 自動カリー化バインディング

関数パラメータ数がバインディング位置数を 초과する場合、自動的にカリー化関数を生成：

```yaoxiang
Point: Type = { x: Float, y: Float }

# 基本関数：3 つのパラメータ
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# バインディング時に自動カリー化
Point.scale = scale[0, 1]   # Point を位置 0、1 にバインディング、位置 2 は保持

# 呼び出し時に自動部分適用
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0) 直接呼び出し
result = scaled              # → Point(4.0, 6.0)

# チェーン呼び出しがより优美的
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### 位置インデックスバインディング構文

`[position]` 構文を導入して、関数パラメータと型のバインディング関係を精密に制御：

```yaoxiang
# 構文形式：Type.method = function[positions]

# === 基本バインディング ===

# 単一位置バインディング
Point.distance = distance[1]           # 位置 1 のパラメータにバインディング（インデックスは 0 から開始）
# 使用：p1.distance(p2) → distance(p2, p1)

# 複数位置バインディング（タプル構造化）
Point.transform = transform[1, 2]      # 位置 1, 2 のパラメータにバインディング
# 使用：p1.transform(v1) → transform(v1, p1)
# 元関数シグネチャ：transform(Point, Vector) → Point
# バインディング後：Point.transform(Vector) → Point
```

### 詳細な構文定義

```
バインディング宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'

位置リスト ::= 位置 (',' 位置)*
位置     ::= 整数                    # プレースホルダ
           | '_'                    # この位置をスキップ（プレースホルダ）
           | 整数 '..' 整数         # 位置範囲（将来の拡張）

関数名   ::= 識別子
型       ::= 識別子 (ジェネリクス引数)?
```

### 組み込みバインディング

バインディングは、独立したバインディングステートメントなしで、型定義体内直接記述できる：

```yaoxiang
# 方法1：型定義体内で直接バインディング
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # 位置 0 にバインディング
}

# 方法2：匿名関数 + 位置バインディング
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
# 構文：((params) => body)[position]
```

**カリー化意味論**：
- `distance = distance[0]` としてバインディングする際、元関数シグネチャは `(a: Point, b: Point) -> Float`
- 生成される method シグネチャ：`b: Point -> Float`（位置 0 は呼び出し元が填充）

### 使用例

```yaoxiang
# === 完全な例 ===

Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

# 1. 基本距離計算
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

# バインディング：Point.distance = distance[1]
# 呼び出し：p1.distance(p2) → distance(p2, p1)
# しかし p1.distance(p2) → distance(p1, p2) を望むため：
Point.distance = distance[0]

# 2. 変換操作（複数位置バインディング）
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# バインディング Point.transform = transform[1]
# 呼び出し：p.transform(v) → transform(v, p) ❌
# バインディング Point.transform = transform[0]
# 呼び出し：p.transform(v) → transform(p, v) ✓

# 3. 複雑な複数パラメータ関数
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# 位置 1 のパラメータのみバインディング（Point 型）、位置 3 のパラメータを保持
Point.scale = multiply[0, _]
# 呼び出し：p.scale(2.0) → multiply(p, 2.0)

# 4. 異型間バインディング
Circle: Type = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# 距離メソッドを Circle 型にバインディング
Circle.distance = distance[0, 1]
# 呼び出し：c1.distance(c2) → distance(c1, c2)
```

### タプル構造化サポート

```yaoxiang
# === タプル構造化バインディング ===

# 関数がタプルパラメータを受け取る
process_coordinates: (coord: (Float, Float)) -> String = {
    return match coord {
        (0.0, 0.0) -> "origin"
        (x, 0.0) -> "on x-axis at ${x}"
        (0.0, y) -> "on y-axis at ${y}"
        (x, y) -> "point at (${x}, ${y})"
    }
}

Coord: Type = { x: Float, y: Float }

# 自動構造化バインディング：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# 使用：coord.describe() → process_coordinates((coord.x, coord.y))
```

### 複数返回值バインディング

```yaoxiang
# === 複数返回值バインディング ===

min_max: (list: List(Int)) -> (Int, Int) = {
    min = list.reduce(Int.MAX, (a, b) => if a < b then a else b)
    max = list.reduce(Int.MIN, (a, b) => if a > b then a else b)
    return (min, max)
}

List.range: (T:Type)->((self: List(T)) -> (T, T)) = min_max[1]
# 使用：(min_val, max_val) = list.range()
```

## 詳細な設計

### コンパイラ実装

### 型検査規則

```rust
fn check_binding_type_compatibility(
    binding: &Binding,
    func: &Function
) -> Result<(), TypeError> {
    // 1. 自動検索位置（明示的に指定されていない場合）、一致するかどうか確認
    if binding.positions.is_empty() {
        return Err(TypeError::NoMatchingParameter(
            binding.type_name.clone(),
            func.name.clone()
        ));
    }

    // 2. すべての位置インデックスが有効であることを検証
    for pos in &binding.positions {
        if *pos >= func.params.len() {
            return Err(TypeError::InvalidBindingPosition(*pos));
        }
    }

    // 3. バインディング位置の型互換性を検査
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. メソッド呼び出し引数と残留引数が一致するかを検査
    Ok(())
}
```

### ランタイム動作

| シナリオ | バインディング構文 | 呼び出し | 変換後 |
|------|---------|------|--------|
| デフォルトバインディング | `Point.distance = distance` | `p1.distance(p2)` | `distance(p1, p2)` |
| 自動一致 | `Point.transform = transform` | `p.transform(v)` | `transform(p, v)` |
| 単一位置 | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| 単一位置 | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| 自動カリー化 | `Point.scale = scale[0, _]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| プレースホルダ | `Type.method = func[1, _]` | `obj.method(arg)` | `func(arg, obj)` |

**説明**：
- **デフォルトバインディング**：自動的に最初の一致する型位置を検索
- `[0]`：this を位置 0 にバインディング（最初のパラメータ）
- `[1]`：this を位置 1 にバインディング（2 番目のパラメータ）
- `[-1]`：this を最後の位置にバインディング（末尾からカウント）

## トレードオフ

### 利点

- **スマートデフォルトバインディング**：デフォルトで最初の一致する型位置にバインディング、明示的な `[positions]` 指定が不要
- **精密な制御**：任意のパラメータ位置にバインディング可能、柔軟性が高い
- **型安全性**：コンパイル時に完全な型検査、型が一致する場合のみバインディング
- **構文が简洁**： `[position]` 構文が直感的で理解しやすい
- **`self` キーワードなし**：言語のシンプルさを維持
- **カリー化に優しい**：自然に部分適用とチェーン呼び出しをサポート
- **OOP に優しい**：自動カリー化により OOP プログラマが簡単に移行可能

### 欠点

- **学習コスト**：位置インデックス概念を理解する必要がある
- **コンパイルの複雑さ**：バインディング解析と型検査によりコンパイラの複雑さが増加
- **デバッグ難易度**：エラーメッセージでバインディング位置の問題を明確に示す必要がある

## 代替案

| 案 | 説明 | 为什么不選 |
|------|------|-----------|
| `self` キーワード | Python/Rust スタイルの `self` を導入 | YaoXiang の暗黙的 `self` なしという設計思想に反する |
| 名前付き引数バインディング | 名前付き引数 `func(a=obj)` を使用 | 関数シグネチャ定義の修改が必要となり、複雑性が増す |
| マクロシステム | マクロでバインディングを実装 | ランタイムオーバーヘッドが大きく、型安全性が低下 |
| 演算子オーバーロード | `self` を特定位置に制限 | 構文が統一されず、意味論が混乱する |

## 実装戦略

### フェーズ分け

1. **Phase 1: 基本バインディング**（v0.3）
   - 単一位置 `[n]` バインディング構文を実装（n は 0 から開始、負数サポート）
   - 基本的な型検査とコード生成
   - ユニットテストカバレッジ

2. **Phase 2: 上級機能**（v0.5）
   - 範囲構文 `[n..m]` をサポート
   - コンパイル時の位置計算最適化

### 依存関係

- 外部依存なし
- RFC-001（エラー処理）との直接的な関連なし
- 独立して実装可能

### リスク

- 既存バインディング構文との互換性処理
- パフォーマンス最適化戦略（コンパイル時展開 vs ランタイム検索）

## 開放問題

以下の問題は設計で既に解決済み、付録A に記録：

- ~~位置インデックスは 0 から開始~~ → 決定済み：0 から開始
- ~~負数インデックス~~ → 決定済み：サポート
- ~~プレースホルダ~~ → 決定済み：`_` を使用
- ~~範囲構文~~ → 決定済み：実装

**残余の開放問題**：

- [ ] 既存バインディング構文との互換性処理
- [ ] パフォーマンス最適化戦略（コンパイル時展開 vs ランタイム検索）

---

## 付録

### 付録A：設計意思決定記録

| 意思決定 | 決定 | 理由 |
|------|------|------|
| インデックス基準 | 0 から開始 | タプル/引数リストインデックスと一致 |
| 負数インデックス | サポート | 柔軟性あり、末尾からカウント可能 |
| プレースホルダ | `_` | 简洁で汎用的な記号 |
| 範囲構文 | 実装 | 一括バインディング、例：`[0..2]` |
| 構文スタイル | 中置 `Type.method = func[positions]` | RFC-010 と統一 |
| **デフォルトバインディングロジック** | **最初の一致する型位置にバインディング** | **よりスマートで安全、直感に従う** |
| **バインディング失敗時の処理** | **一致するものが見つからない場合はエラー/警告/ファクトリ関数** | **コンテキストに応じて柔軟に処理** |
| **関数構文** | **パラメータ名がシグネチャに含まれる `name: (params) -> Return`** | **RFC-010 と統一** |

### 付録B：用語集

| 用語 | 定義 |
|------|------|
| バインディング位置 | 関数パラメータリスト内のインデックス位置 |
| 位置バインディング | 型を複数パラメータ位置にバインディング |
| 部分適用 | 一部の引数のみ提供、残りの呼び出しを返す関数を返す |
| **統一構文** | **`name: (params) -> Return = body`、パラメータ名がシグネチャ内で宣言** |
| **型一致バインディング** | **デフォルトバインディングロジック：呼び出し元の型と一致する最初位置を自動検索** |
| **ファクトリ関数バインディング** | **関数パラメータに一致するものがない場合、コンストラクタとして使用** |

---

## 参考文献

- [Rust impl 構文](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 型クラス](https://wiki.haskell.org/Type_class)
- [Kotlin 拡張関数](https://kotlinlang.org/docs/extensions.html)
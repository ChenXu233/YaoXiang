```yaml
---
title: RFC-004：カリー化メソッドの複数位置聯合束縛設計
---

# RFC-004: カリー化メソッドの複数位置聯合束縛設計

> **状態**: 承認済み
> **著者**: 晨煦
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-18（組み込み束縛、後置束縛構文を追加）

## 概要

本 RFC は、関数を型の任意のパラメータ位置に精密に束縛することを可能にする新しい**複数位置聯合束縛**構文を提案する。単位置束縛と複数位置聯合束縛をサポートし、カリー化束縛における「誰が呼び出し元か」という問題を根本的に解決し、`self` キーワードの導入を必要としない。

## 動機

### なぜこの機能が必要か？

現在の言語設計では、独立関数を型メソッドとして束縛する際に以下の問題を抱えている：

1. **呼び出し元の位置が柔軟でない**: 従来の束縛では `obj.method(args)` の `obj` を最初の引数に固定することしかできない
2. **複数引数束縛が困難**: メソッドが同じ型の複数引数を受け取る必要がある場合、優雅に表現できない
3. **カリー化セマンティクスの曖昧さ**: 部分適用時に「どの位置に束縛するか」の区別が難しい

### 設計目標：2つのプログラミング視点の統一

本設計は**関数型と OOP という2つのプログラミング視点を統一する**ことを目指す：

```yaoxiang
# 関数視点：すべての引数を明示的に渡す
distance(p1, p2)

# OOP視点：暗黙的な this
p1.distance(p2)

# [positions] 構文糖衣により2つの記述が同等になり、本質的にどちらも関数呼び出し
Point.distance = distance[0]   # this を位置 0 に束縛
```

**コアバリュー**：
- 下層は関数、上層はメソッド構文
- `self` キーワードを導入せず、言語のシンプルさを維持
- 完全関数型：メソッド呼び出しの本質は引数渡しまたは値変体
- `[0]`, `[1]`, `[-1]` で this 束縛位置を柔軟に制御
- **構文の統一**：関数定義は `name: (params) -> Return = body` 形式を使用

### 現在の問題

```yaoxiang
# 既存設計の問題点：
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# 最初の引数にしか束縛できない
Point.distance = distance  # distance[0] と同等
# p1.distance(p2) → distance(p1, p2) ✓

# しかし transform のシグネチャが transform(Vector, Point) だったら？
# p1.transform(v1) → transform(v1, p1) のセマンティクスを表現できない
```

## 提案

### コア設計：デフォルト束縛 + 任意位置指定

#### 最初の型一致位置へのデフォルト束縛

**デフォルト動作**：`Type.method = function` はその型と一致する最初の位置を自動検索して束縛

```yaoxiang
# デフォルトで最初の型一致位置に束縛
Point.distance = distance           # コンパイラが最初の Point 引数位置を自動検索
p1.distance(p2)                     # → distance(p1, p2)

# 関数が2つの Point 引数を持つ場合、最初の型一致位置に束縛
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}
# 束縛：Point.distance = distance
# 呼び出し：p1.distance(p2) → distance(p1, p2) ✓

# 特殊位置（最初の型一致位置以外）が必要な場合のみ明示的に指定
Point.compare = distance[1]        # 2番目の Point 引数に束縛
p1.compare(p2)                    # → distance(p2, p1)
```

**束縛失敗時の処理**：
- **一致する型が見つからない**: 関数の引数にその型がない場合、エラーまたは警告
- **ファクトリ関数モード**: 引数に一致するものがない場合、コンストラクタとして使用

```yaoxiang
# ケース1：一致する型が見つからない
create_point: () -> Point = { ... }
Point.create = create_point        # エラー：Point 型の引数がない

# ケース2：ファクトリ関数モード（オプション）
Point.create = create_point        # ファクトリ関数として呼び出し：Point.create()
```

**利点**：
- 知能束縛：型に基づいて自動一致し、直感的
- 型安全性：型が一致するものだけが束縛され、ミスを避ける
- 柔軟な制御：デフォルト束縛が望んだ動作でない場合、位置を明示的に指定可能

#### 自動カリー化束縛

関数引数数が束縛位置数より多い場合、自動的にカリー化関数を生成：

```yaoxiang
Point: Type = { x: Float, y: Float }

# 基本関数：3 個の引数
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# 束縛時に自動カリー化
Point.scale = scale[0, 1]   # Point を位置 0、1 に束縛、位置 2 は保持

# 呼び出し時に自動部分適用
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0) 直接呼び出し
result = scaled              # → Point(4.0, 6.0)

# チェーン呼び出しがより優雅
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### 位置インデックス束縛構文

関数の引数と型の束縛関係を精密に制御するために `[position]` 構文を導入：

```yaoxiang
# 構文形式：Type.method = function[positions]

# === 基本的な束縛 ===

# 単位置束縛
Point.distance = distance[1]           # 位置1（インデックスは0から開始）
# 使用：p1.distance(p2) → distance(p2, p1)

# 複数位置聯合束縛（タプルデストラクト）
Point.transform = transform[1, 2]      # 位置1,2に束縛
# 使用：p1.transform(v1) → transform(v1, p1)
# 元関数シグネチャ：transform(Point, Vector) → Point
# 束縛後：Point.transform(Vector) → Point
```

### 詳細な構文定義

```
束縛宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'

位置リスト ::= 位置 (',' 位置)*
位置     ::= 整数                    # プレースホルダー
           | '_'                    # この位置をスキップ（プレースホルダー）
           | 整数 '..' 整数         # 位置範囲（将来拡張用）

関数名   ::= 識別子
型     ::= 識別子 (ジェネリック引数)?
```

### 組み込み束縛

束縛は独立した束縛文 없이、型定義体内直接に記述できる：

```yaoxiang
# 方法1：型定義体内で直接束縛
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # 位置0に束縛
}

# 方法2：匿名関数 + 位置束縛
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

**カリー化セマンティクス**：
- `distance = distance[0]` を束縛時、元関数シグネチャ `(a: Point, b: Point) -> Float`
- 生成される method シグネチャ：`b: Point -> Float`（位置0は呼び出し元が填充）

### 使用例

```yaoxiang
# === 完全な例 ===

Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

# 1. 基本的な距離計算
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

# 束縛：Point.distance = distance[1]
# 呼び出し：p1.distance(p2) → distance(p2, p1)
# しかし p1.distance(p2) → distance(p1, p2) が欲しいので：
Point.distance = distance[0]

# 2. 変換操作（複数位置束縛）
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# 束縛 Point.transform = transform[1]
# 呼び出し：p.transform(v) → transform(v, p) ❌
# 束縛 Point.transform = transform[0]
# 呼び出し：p.transform(v) → transform(p, v) ✓

# 3. 複雑な複数引数関数
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# 位置1（Point型）のみ束縛、位置3を保持
Point.scale = multiply[0, _]
# 呼び出し：p.scale(2.0) → multiply(p, 2.0)

# 4. 異型間束縛
Circle: Type = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# 距離メソッドを Circle 型に束縛
Circle.distance = distance[0, 1]
# 呼び出し：c1.distance(c2) → distance(c1, c2)
```

### タプルデストラクトサポート

```yaoxiang
# === タプルデストラクト束縛 ===

# 関数がタプル引数を受け取る
process_coordinates: (coord: (Float, Float)) -> String = {
    return match coord {
        (0.0, 0.0) -> "origin"
        (x, 0.0) -> "on x-axis at ${x}"
        (0.0, y) -> "on y-axis at ${y}"
        (x, y) -> "point at (${x}, ${y})"
    }
}

Coord: Type = { x: Float, y: Float }

# 自動デストラクト束縛：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# 使用：coord.describe() → process_coordinates((coord.x, coord.y))
```

### 複数戻り値束縛

```yaoxiang
# === 複数戻り値束縛 ===

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
    // 1. 自動位置検索の場合（明示的に指定なし）、一致するものが見つかったか確認
    if binding.positions.is_empty() {
        return Err(TypeError::NoMatchingParameter(
            binding.type_name.clone(),
            func.name.clone()
        ));
    }

    // 2. すべての位置インデックスが有効か検証
    for pos in &binding.positions {
        if *pos >= func.params.len() {
            return Err(TypeError::InvalidBindingPosition(*pos));
        }
    }

    // 3. 束縛位置の型互換性を検査
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. メソッド呼び出し引数と残り引数の一致を検査
    Ok(())
}
```

### 実行時動作

| シナリオ | 束縛構文 | 呼び出し | 変換後 |
|------|---------|------|--------|
| デフォルト束縛 | `Point.distance = distance` | `p1.distance(p2)` | `distance(p1, p2)` |
| 自動一致 | `Point.transform = transform` | `p.transform(v)` | `transform(p, v)` |
| 単位置 | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| 単位置 | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| 自動カリー化 | `Point.scale = scale[0, _]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| プレースホルダー | `Type.method = func[1, _]` | `obj.method(arg)` | `func(arg, obj)` |

**説明**：
- **デフォルト束縛**: 最初の型一致位置を自動検索
- `[0]`: this を位置 0（第1引数）に束縛
- `[1]`: this を位置 1（第2引数）に束縛
- `[-1]`: this を最後位置（末尾からカウント）に束縛

## トレードオフ

### 利点

- **知能デフォルト束縛**: デフォルトで最初の型一致位置に束縛し、明示的な `[positions]` 指定が不要
- **精密な制御**: 任意のパラメータ位置に束縛可能で、柔軟性が高い
- **型安全性**: コンパイル時に完全な型検査を行い、型が一致するものだけが束縛
- **構文が簡潔**: `[position]` 構文が直感的で理解しやすい
- **`self` キーワードなし**: 言語のシンプルさを維持
- **カリー化に対応**: 自然に部分適用とチェーン呼び出しをサポート
- **OOP に友好**: 自動カリー化により OOP 開発者の移行が容易

### 欠点

- **学習コスト**: 位置インデックス概念の理解が必要
- **コンパイル複雑度**: 束縛解析と型検査がコンパイラの複雑度を増加
- **デバッグ難易度**: エラーメッセージで束縛位置の問題を明確に示す必要がある

## 代替案

| 案 | 説明 | 採用しない理由 |
|------|------|-----------|
| `self` キーワード | Python/Rust 風の `self` を導入 | YaoXiang の暗黙的 `self` なし設計哲学に反する |
| 名前付き引数束縛 | 名前付き引数 `func(a=obj)` を使用 | 関数シグネチャ定義の修改が必要で複雑性が増す |
| マクロシステム | マクロで束縛を実装 | 実行時オーバーヘッドが大きく、型安全性が低下 |
| 演算子オーバーロード | `self` を特定位置に制限 | 構文が統一されず、セマンティクスが混乱 |

## 実装戦略

### 段階的実施

1. **Phase 1: 基本的な束縛**（v0.3）
   - 単位置 `[n]` 束縛構文の実装（n は 0 から開始、負数サポート）
   - 基本的な型検査とコード生成
   - ユニットテストのカバレッジ

2. **Phase 2: 高機能特性**（v0.5）
   - 範囲構文 `[n..m]` のサポート
   - コンパイル時位置計算の最適化

### 依存関係

- 外部依存なし
- RFC-001（エラーハンドリング）との直接的な関連なし
- 独立して実装可能

### リスク

- 既存束縛構文との互換性处理
- パフォーマンス最適化戦略（コンパイル時展開 vs 実行時検索）

## 開放問題

以下の問題はすでに設計で解決済みであり、付録Aに記録：

- ~~位置インデックスは 0 から開始~~ → 決定済み：0 から開始
- ~~負数インデックス~~ → 決定済み：サポート
- ~~プレースホルダー~~ → 決定済み：`_` を使用
- ~~範囲構文~~ → 決定済み：実装

**残りの開放問題**：

- [ ] 既存束縛構文との互換性処理
- [ ] パフォーマンス最適化戦略（コンパイル時展開 vs 実行時検索）

---

## 付録

### 付録A：設計上の決定記録

| 決定 | 結論 | 理由 |
|------|------|------|
| インデックス基準 | 0 から開始 | タプル/引数リストインデックスと一致 |
| 負数インデックス | サポート | 柔軟性あり、末尾からカウント |
| プレースホルダー | `_` | 簡潔で汎用的な記号 |
| 範囲構文 | 実装 | バッチ束縛、例：`[0..2]` |
| 構文スタイル | 中置 `Type.method = func[positions]` | RFC-010 と統一 |
| **デフォルト束縛ロジック** | **最初の型一致位置に束縛** | **より知能的、より安全で直感的** |
| **束縛失敗時の処理** | **一致するものが見つからない場合はエラー/警告/ファクトリ関数** | **コンテキストに応じて柔軟に処理** |
| **関数構文** | **シグネチャ内に引数名 `name: (params) -> Return`** | **RFC-010 と統一** |

### 付録B：用語集

| 用語 | 定義 |
|------|------|
| 束縛位置 | 関数引数リスト内のインデックス位置 |
| 聯合束縛 | 型を複数の引数位置に束縛 |
| 部分適用 | 一部の引数のみを提供し、未完了呼び出しの関数を返す |
| **統一構文** | **`name: (params) -> Return = body`、シグネチャ内で引数名を宣言** |
| **型一致束縛** | **デフォルト束縛ロジック：呼び出し元の型と一致する最初の位置を自動検索** |
| **ファクトリ関数束縛** | **関数の引数に一致する型がない場合、コンストラクタとして使用** |

---

## 参考文献

- [Rust impl 構文](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 型クラス](https://wiki.haskell.org/Type_class)
- [Kotlin 拡張関数](https://kotlinlang.org/docs/extensions.html)
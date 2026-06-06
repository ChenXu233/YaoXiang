```yaml
title: "RFC-004: 柯里化メソッドの位置統合バインディング設計"
status: "承認済み"
author: "晨煦"
created: "2025-01-05"
updated: "2026-02-18（内置バインディング、後置バインディング構文の追加）"
```

# RFC-004: 柯里化メソッドの位置統合バインディング設計

## 摘要

本 RFC は`**多位置統合バインディング**`という新しい構文を提案する。関数を型の任意のパラメータ位置に精密にバインディングすることを可能にし、単位置バインディングと多位置統合バインディングの両方をサポートする。柯里化バインディングにおける「誰がコーラーか」という問題を根本的に解決し、`self` キーワードの導入は不要である。

## 動機

### なぜこの機能が必要か？

現在の言語設計では、独立関数を型メソッドとしてバインディングする際に以下の問題を抱えている：

1. **コーラー位置が不柔軟**：伝統的なバインディングでは `obj.method(args)` の `obj` を 첫 번째 引数에만固定できる
2. **多パラメータバインディングの困難**：メソッドが同じ型の複数のパラメータを受け取る必要がある場合、優雅に表現できない
3. **柯里化意味論の曖昧性**：部分適用の際に「どの位置にバインディングするか」を区別しにくい

### 設計目標：2つのプログラミング視点の統一

本設計は**関数型と OOP の2つのプログラミング視点を統一する**ことを目指す：

```yaoxiang
# 関数視点：全てのパラメータを明示的に渡す
distance(p1, p2)

# OOP 視点：暗黙的な this
p1.distance(p2)

# [positions] 構文糖衣により2つの記述は同等、本質は関数呼び出し
Point.distance = distance[0]   # this を位置 0 にバインディング
```

**コアバリュー**：
- 下層は関数、上層はメソッド構文
- `self` キーワードを導入しない、语言の簡潔性を維持
- 完全関数化：メソッド呼び出しの本質はパラメータ 전달
- `[0]`, `[1]`, `[-1]` で this バインディング位置を柔軟に制御
- **構文の統一**：関数定義は `name: (params) -> Return = body` フォーマットを使用

### 現在の問題

```yaoxiang
# 既存設計の問題：
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# 最初の引数にしかバインディングできない
Point.distance = distance  # distance[0] と同等
# p1.distance(p2) → distance(p1, p2) ✓

# しかし transform のシグネチャが transform(Vector, Point) なら？
# p1.transform(v1) → transform(v1, p1) の意味を表現できない
```

## 提案

### コア設計：デフォルトバインディング + 任意の位置指定

#### 最初の型一致位置へのデフォルトバインディング

**デフォルト動作**：`Type.method = function` は自動的にその型と一致する最初のパラメータ位置を探してバインディング

```yaoxiang
# デフォルトバインディング：最初の型一致位置
Point.distance = distance           # コンパイラが自動的に最初の Point パラメータ位置を検索
p1.distance(p2)                     # → distance(p1, p2)

# 関数が2つの Point パラメータを持つ場合、最初の一致位置にバインディング
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}
# バインディング：Point.distance = distance
# 呼び出し：p1.distance(p2) → distance(p1, p2) ✓

# 特殊位置（最初の一致以外）が必要な場合のみ明示的に指定
Point.compare = distance[1]        # 2番目の Point パラメータにバインディング
p1.compare(p2)                    # → distance(p2, p1)
```

**バインディング失敗処理**：
- **一致する型が見つからない**：関数パラメータにその型がない場合、エラーまたは警告
- **ファクトリ関数モード**：パラメータが一致しない場合、コンストラクタとして使用

```yaoxiang
# ケース1：一致する型が見つからない
create_point: () -> Point = { ... }
Point.create = create_point        # エラー：Point 型パラメータがない

# ケース2：ファクトリ関数モード（任意）
Point.create = create_point        # ファクトリ関数として、呼び出し：Point.create()
```

**利点**：
- スマートバインディング：型に基づいて自動一致、直感に適合
- 型安全：型が一致する場合のみバインディング、エラーを回避
- 柔軟な制御：デフォルトバインディングが期待する動作でない場合、位置を明示的に指定可能

#### 自動柯里化バインディング

関数パラメータ数 > バインディング位置数の場合、自動的に柯里化関数を生成：

```yaoxiang
Point: Type = { x: Float, y: Float }

# 基本関数：3 つのパラメータ
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# バインディング時に自動柯里化
Point.scale = scale[0, 1]   # Point を位置 0、1 にバインディング、位置 2 は保留

# 呼び出し時に自動部分適用
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0) 直接呼び出し
result = scaled              # → Point(4.0, 6.0)

# チェイン呼び出しがより優雅
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### 位置インデックスバインディング構文

`[position]` 構文を導入して、関数パラメータと型のバインディング関係を精密に制御：

```yaoxiang
# 構文フォーマット：Type.method = function[positions]

# === 基本バインディング ===

# 単位置バインディング
Point.distance = distance[1]           # 位置1（インデックスは0から開始）の引数にバインディング
# 使用：p1.distance(p2) → distance(p2, p1)

# 多位置統合バインディング（タプルデストラクト）
Point.transform = transform[1, 2]      # 位置1、2の引数にバインディング
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
           | 整数 '..' 整数         # 位置範囲（将来拡張）

関数名   ::= 識別子
型     ::= 識別子 (型パラメータ)?
```

### 内置バインディング

バインディングは独立したバインディング statements の代わりに、型定義体内直接記述可能：

```yaoxiang
# 方法1：型定義体内で直接バインディング
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # 位置0にバインディング
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

**柯里化意味論**：
- バインディング `distance = distance[0]` 時、元関数シグネチャは `(a: Point, b: Point) -> Float`
- 生成される method シグネチャ：`b: Point -> Float`（位置0はコーラーが填充）

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
# しかし p1.distance(p2) → distance(p1, p2) を所欲のため：
Point.distance = distance[0]

# 2. 変換操作（多位置バインディング）
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# バインディング Point.transform = transform[1]
# 呼び出し：p.transform(v) → transform(v, p) ❌
# バインディング Point.transform = transform[0]
# 呼び出し：p.transform(v) → transform(p, v) ✓

# 3. 複雑な多パラメータ関数
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# 位置1（Point型）のみバインディング、位置3を保留
Point.scale = multiply[0, _]
# 呼び出し：p.scale(2.0) → multiply(p, 2.0)

# 4. クロス型バインディング
Circle: Type = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# 距離メソッドを Circle 型にバインディング
Circle.distance = distance[0, 1]
# 呼び出し：c1.distance(c2) → distance(c1, c2)
```

### タプルデストラクトサポート

```yaoxiang
# === タプルデストラクトバインディング ===

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

# 自動デストラクトバインディング：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# 使用：coord.describe() → process_coordinates((coord.x, coord.y))
```

### 複数返り値バインディング

```yaoxiang
# === 複数返り値バインディング ===

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

### 型チェックルール

```rust
fn check_binding_type_compatibility(
    binding: &Binding,
    func: &Function
) -> Result<(), TypeError> {
    // 1. 自動検索位置（明示的に指定なし）の場合、一致が見つかったかを確認
    if binding.positions.is_empty() {
        return Err(TypeError::NoMatchingParameter(
            binding.type_name.clone(),
            func.name.clone()
        ));
    }

    // 2. 全てのパラメータ位置のインデックスが有効であることを検証
    for pos in &binding.positions {
        if *pos >= func.params.len() {
            return Err(TypeError::InvalidBindingPosition(*pos));
        }
    }

    // 3. バインディング位置の型互換性を確認
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. メソッド呼び出しパラメータが残余パラメータと一致することを確認
    Ok(())
}
```

### 実行時動作

| シナリオ | バインディング構文 | 呼び出し | 変換後 |
|------|---------|------|--------|
| デフォルトバインディング | `Point.distance = distance` | `p1.distance(p2)` | `distance(p1, p2)` |
| 自動一致 | `Point.transform = transform` | `p.transform(v)` | `transform(p, v)` |
| 単位置 | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| 単位置 | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| 自動柯里化 | `Point.scale = scale[0, _]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| プレースホルダ | `Type.method = func[1, _]` | `obj.method(arg)` | `func(arg, obj)` |

**説明**：
- **デフォルトバインディング**：最初に型が一致する位置を自動検索
- `[0]`：this を位置 0（第1引数）にバインディング
- `[1]`：this を位置 1（第2引数）にバインディング
- `[-1]`：this を最后の位置（末尾からカウント）にバインディング

## トレードオフ

### 利点

- **スマートデフォルトバインディング**：デフォルトで最初の型が一致する位置にバインディング、`[positions]` の明示的な指定が不要
- **精密な制御**：任意のパラメータ位置にバインディング可能、柔軟性が高い
- **型安全**：编译時完全に型チェック、型が一致する場合のみバインディング
- **構文が簡潔**：`[position]` 構文は直感的で理解しやすい
- **`self` キーワードなし**：语言の簡潔性を維持
- **柯里化に優しい**：自然に部分適用とチェイン呼び出しをサポート
- **OOP に優しい**：自動柯里化により OOP プログラマの移行が容易

### 欠点

- **学習コスト**：位置インデックス概念の理解が必要
- **コンパイルの複雑さ**：バインディング解析と型チェックがコンパイラの複雑さを増加
- **デバッグ難易度**：エラーメッセージはバインディング位置の問題を明確に示す必要がある

## 代替案

| 方案 | 説明 | 採用しない理由 |
|------|------|-----------|
| `self` キーワード | Python/Rust スタイルの `self` を導入 | YaoXiang の暗黙的 `self` なし設計哲学に違反 |
| 名前付きパラメータバインディング | 名前付きパラメータを使用 `func(a=obj)` | 関数シグネチャ定義の修改が必要、複雑性が増加 |
| マクロシステム | マクロでバインディングを実装 | 実行時オーバーヘッド大、型安全性が低下 |
| 演算子オーバーロード | `self` を特定位置に制限 | 構文が統一されず、意味論が混乱 |

## 実装戦略

### フェーズ区分

1. **Phase 1: 基本バインディング**（v0.3）
   - 単位置 `[n]` バインディング構文の実装（n は 0 から開始、負数サポート）
   - 基本的な型チェックとコード生成
   - ユニットテストカバレッジ

2. **Phase 2: 高級機能**（v0.5）
   - 範囲構文 `[n..m]` のサポート
   - 编译時位置計算最適化

### 依存関係

- 外部依存なし
- RFC-001（錯誤処理）との直接関連なし
- 独立実装可能

### リスク

- 既存バインディング構文との互換性処理
- パフォーマンス最適化戦略（编译時展開 vs 実行時検索）

## 開放問題

以下の問題は設計で既に解決済み、付録Aに記録：

- ~~位置インデックスは 0 から開始~~ → 決定済み：0 から開始
- ~~負数インデックス~~ → 決定済み：サポート
- ~~プレースホルダ~~ → 決定済み：`_` を使用
- ~~範囲構文~~ → 決定済み：実装

**残余開放問題**：

- [ ] 既存バインディング構文との互換性処理
- [ ] パフォーマンス最適化戦略（编译時展開 vs 実行時検索）

---

## 付録

### 付録A：設計意思決定記録

| 意思決定 | 決定 | 理由 |
|------|------|------|
| インデックス基準 | 0 から開始 | タプル/パラメータリストインデックスと一致 |
| 負数インデックス | サポート | 柔軟、末尾からカウント |
| プレースホルダ | `_` | 簡潔、汎用記号 |
| 範囲構文 | 実装 | バッチバインディング、例：`[0..2]` |
| 構文スタイル | 中置 `Type.method = func[positions]` | RFC-010 と統一 |
| **デフォルトバインディング論理** | **最初の型が一致する位置にバインディング** | **よりスマート、より安全、直感に適合** |
| **バインディング失敗処理** | **一致するものが見つからない場合エラー/警告/ファクトリ関数** | **コンテキストに応じて柔軟に処理** |
| **関数構文** | **シグネチャ内にパラメータ名 `name: (params) -> Return`** | **RFC-010 と統一** |

### 付録B：用語集

| 用語 | 定義 |
|------|------|
| バインディング位置 | 関数パラメータリスト内のインデックス位置 |
| 統合バインディング | 型を複数パラメータ位置にバインディング |
| 部分適用 | 一部の引数のみ提供、未完了呼び出しの関数を返す |
| **統一構文** | **`name: (params) -> Return = body`、シグネチャ内にパラメータ名を宣言** |
| **型一致バインディング** | **デフォルトバインディング論理：コーラー型と一致する最初のパラメータ位置を自動検索** |
| **ファクトリ関数バインディング** | **関数のパラメータに一致する型がない場合、コンストラクタとして使用** |

---

## 参考文献

- [Rust impl 構文](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 型クラス](https://wiki.haskell.org/Type_class)
- [Kotlin 拡張関数](https://kotlinlang.org/docs/extensions.html)
---
title: "RFC-004: カレ化されたメソッドの位置連合バインディング設計"
status: "承認済み"
author: "晨煦"
created: "2025-01-05"
updated: "2026-02-18（ビルトインバインディング、後置バインディング構文の追加）"
issue: "#132"
---

# RFC-004: カレ化されたメソッドの位置連合バインディング設計

## 概要

本 RFC は全新的な**多位置連合バインディング**構文を提案する。関数を型の任意の引数位置に正確にバインディングすることを可能にし、单位置バインディングと多位置連合バインディングをサポートすることで、カレ化されたバインディングにおける「誰が呼び出し側か」という問題を根本的に解決し、`self` キーワードを導入する必要がない。

## 動機

### なぜこの機能が必要なのか？

現在の言語設計では、独立した関数を型メソッドとしてバインディングする際に以下の問題を抱えている：

1. **呼び出し側位置の柔軟性が低い**：従来のバインディングでは `obj.method(args)` の `obj` を最初の引数に固定只能
2. **複数引数バインディングの困難**：メソッドが同じ型の複数の引数を受け取る必要がある場合、優雅に表現できない
3. **カレ化意味の曖昧性**：部分適用時に「どの位置にバインディングされているか」を区別しにくい

### 設計目標：2つのプログラミング視点の統一

本設計は**関数型とOOPの2つのプログラミング視点を統一する**ことを目指す：

```yaoxiang
# 関数視点：すべての引数を明示的に渡す
distance(p1, p2)

# OOP視点：暗黙的な this
p1.distance(p2)

# [positions] 構文糖衣により2つの書き方が同等、本質的にはどちらも関数呼び出し
Point.distance = distance[0]   # this を第 0 位置にバインディング
```

**コアバリュー**：
- 底层は関数、上層はメソッド構文
- `self` キーワードを導入せず、言語の簡潔さを維持
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

# 只能バインディング到第一个引数
Point.distance = distance  # distance[0] と同等
# p1.distance(p2) → distance(p1, p2) ✓

# しかし transform のシグネチャが transform(Vector, Point) なら？
# p1.transform(v1) → transform(v1, p1) の意味を表現できない
```

## 提案

### コア設計：明示的な位置指定

**コアルール：`[n]` を書かない = バインディングしない。** `Point.name = func` は只是名称空間エイリアスであり、任何な暗黙的なバインディングをトリガーしない。`p.name(args)` のような `.` 呼び出し構文を有効にするには、明示的に指定する必要がある：`Point.name = func[n]`。

#### 单位置バインディング

```yaoxiang
# 明示的に第 1 の Point 引数位置にバインディング（インデックスは 0 から開始）
Point.distance = distance[0]
p1.distance(p2)                     # → distance(p1, p2)

# 第 2 の Point 引数位置にバインディング
Point.compare = distance[1]         # 第 2 の Point 引数にバインディング
p1.compare(p2)                      # → distance(p2, p1)
```

**`[n]` を書かない = バインディングしない**：

```yaoxiang
# [n] がない → 純粋な名称空間エイリアス、. 呼び出し構文なし
Point.distance = distance            # Point.distance(p1, p2) のみ
# p1.distance(p2)  ❌  バインディングなし

# ファクトリ関数は自然合法、特別处理不要
create_point: () -> Point = { ... }
Point.create = create_point          # Point.create()   ✅
```
- 型安全性：型が一致する場合のみバインディング、错误を回避
- 柔軟な制御：`[n]` によりバインディング位置を精密に制御

#### カレ化バインディング

関数の引数数がバインディング位置数より多い場合、自动的にカレ化された関数が生成される。**バインディングは常に明示的な操作である。**

```yaoxiang
Point: Type = { x: Float, y: Float }

# 基礎関数
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# 位置 0 に明示的にバインディング → カレ化：残留引数 factor は呼び出し側が提供
Point.scale = scale[0]

# 呼び出し
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0)

# 链式呼び出しはより優雅
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### 位置インデックスバインディング構文

`[position]` 構文を導入して、関数の引数と型のバインディング関係を精密に制御する：

```yaoxiang
# 構文形式：Type.method = function[positions]

# === 基礎バインディング ===

# 单位置バインディング
Point.distance = distance[1]           # 第1引数にバインディング（インデックスは0から開始）
# 使用：p1.distance(p2) → distance(p2, p1)

# 多位置連合バインディング（タプルデストラクト）
Point.transform = transform[1, 2]      # 第1,2引数にバインディング
# 使用：p1.transform(v1) → transform(v1, p1)
# 元関数シグネチャ：transform(Point, Vector) → Point
# バインディング後：Point.transform(Vector) → Point
```

### 詳細な構文定義

```
バインディング宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'

位置リスト ::= 位置 (',' 位置)*
位置     ::= 整数                    # プレースホルダー
           | '_'                    # この位置をスキップ（プレースホルダー）
           | 整数 '..' 整数         # 位置範囲（将来拡張）

関数名   ::= 識別子
型     ::= 識別子 (泛型引数)?
```

### ビルドインバインディング

バインディングは、单独的バインディングステートメントなしで、型定義体内直に書くことができる：

```yaoxiang
# 方法1：型定義体内で直にバインディング
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

**カレ化意味**：
- `distance = distance[0]` をバインディングする際、元関数シグネチャは `(a: Point, b: Point) -> Float`
- 生成される method シグネチャ：`b: Point -> Float`（第 0 位は呼び出し側が填充）

### 使用例

```yaoxiang
# === 完全例 ===

Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

# 1. 基礎距離計算
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

# バインディング：Point.distance = distance[1]
# 呼び出し：p1.distance(p2) → distance(p2, p1)
# しかし p1.distance(p2) → distance(p1, p2) を所欲なので：
Point.distance = distance[0]

# 2. 変換操作（多位置バインディング）
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# バインディング Point.transform = transform[1]
# 呼び出し：p.transform(v) → transform(v, p) ❌
# バインディング Point.transform = transform[0]
# 呼び出し：p.transform(v) → transform(p, v) ✓

# 3. 複雑な複数引数関数
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# 第1引数（Point型）のみバインディング、第3引数を保持
Point.scale = multiply[0, _]
# 呼び出し：p.scale(2.0) → multiply(p, 2.0)

# 4. 跨タイプバインディング
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

# 自动デストラクトバインディング：Coord -> (Float, Float)
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
### 型チェックルール

```rust
fn check_binding_type_compatibility(
    binding: &Binding,
    func: &Function
) -> Result<(), TypeError> {
    // 1. 自動検索位置の場合（明示的に指定なし）、一致するものが見つかったかチェック
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

    // 3. バインディング位置の型互換性をチェック
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. メソッド呼び出し引数と残留引数の一致をチェック
    Ok(())
}
```

### 実行時動作

| シナリオ | バインディング構文 | 呼び出し | 変換後 |
|------|---------|------|--------|
| バインディングなし | `Point.distance = distance` | `Point.distance(p1, p2)` | `distance(p1, p2)` |
| 单位置 | `Point.distance = distance[0]` | `p1.distance(p2)` | `distance(p1, p2)` |
| 单位置 | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| 負数インデックス | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| 多位置(カレ化) | `Point.scale = scale[0]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| プレースホルダー | `Type.method = func[1]` | `obj.method(arg)` | `func(arg, obj)` |

**説明**：
- **バインディングなし**：`Point.name = func` 只是名称空間エイリアス、. 呼び出し構文なし
- `[0]`：呼び出し側を第 0 位（最初の引数）にバインディング
- `[1]`：呼び出し側を第 1 位（2番目の引数）にバインディング
- `[-1]`：呼び出し側を最後位（末尾からカウント）にバインディング

## トレードオフ

### 优点

- **明示的なバインディング**：`[n]` は唯一のバインディング機構、不書不バインディング、暗黙的動作なし
- **精密な制御**：任意の引数位置にバインディング可能、柔軟性が高い
- **型安全性**：コンパイル時に完全な型チェック、型が一致する場合のみバインディング
- **簡潔な構文**：`[position]` 構文は直感的で理解しやすい
- **`self` キーワードなし**：言語の簡潔さを維持
- **カレ化に優しい**：自然に部分適用と链式呼び出しをサポート
- **OOPに優しい**：自动カレ化によりOOPプログラマーは気軽に移行可能

### 缺点

- **学習コスト**：位置インデックス概念の理解が必要
- **コンパイル复杂度**：バインディング解析と型チェックがコンパイラ复杂度を增加
- **デバッグ難易度**：错误メッセージはバインディング位置問題を明確に示す必要がある

## 代替案

| 方案 | 記述 | なぜ選択しないか |
|------|------|-----------|
| `self` キーワード | Python/Rust スタイルの `self` を導入 | YaoXiang の暗黙的 `self` なしという設計哲学に違反 |
| 命名引数バインディング | 命名引数 `func(a=obj)` を使用 | 関数シグネチャ定義の修改が必要、複雑性が増加 |
| マクロシステム | マクロでバインディングを実装 | 実行時オーバーヘッド大、型安全性が低下 |
| 演算子オーバーロード | `self` を特定位置に制限 | 構文が統一されず、意味が混乱 |

## 実装戦略

### フェーズ分け

1. **Phase 1: 基礎バインディング**（v0.3）
   - 单位置 `[n]` バインディング構文を実装（n は 0 から開始、負数サポート）
   - 基本的な型チェックとコード生成
   - ユニットテスト覆盖

2. **Phase 2: 上級機能**（v0.5）
   - 範囲構文 `[n..m]` をサポート
   - コンパイル時位置計算最適化

### 依存関係

- 外部依存なし
- RFC-001（错误処理）との直接関連なし
- 独立して実装可能

### リスク

- 既存バインディング構文との互換性处理
- パフォーマンス最適化戦略（コンパイル時展開 vs 実行時検索）

## 開放問題

以下的问题是已在设计中解决、記録在付録A：

- ~~位置インデックスは 0 から開始~~ → 決定済み：0 から開始
- ~~負数インデックス~~ → 決定済み：サポート
- ~~プレースホルダー~~ → 決定済み：`_` を使用
- ~~範囲構文~~ → 決定済み：実装

**残留する開放問題**：

- [ ] 既存バインディング構文との互換性处理
- [ ] パフォーマンス最適化戦略（コンパイル時展開 vs 実行時検索）

---

## 付録

### 付録A：設計意思決定記録

| 意思決定 | 決定 | 理由 |
|------|------|------|
| インデックス基準 | 0 から開始 | タプル/引数リストのインデックスと一致 |
| 負数インデックス | サポート | 柔軟、末尾からカウント可能 |
| プレースホルダー | `_` | 簡潔で汎用的な記号 |
| 範囲構文 | 実装 | 一括バインディング、`[0..2]` など |
| 構文スタイル | 中置 `Type.method = func[positions]` | RFC-010 と統一 |
| **バインディングルール** | **明示的な `[n]` のみバインディング、不書はバインディングなし** | **暗黙的動作なし、関数定義とバインディングは直交** |
| **名称空間** | **`Type.name` は只是名称空間の所属、バインディングをトリガーしない** | **定義とバインディングを分離** |
| **関数構文** | **引数名がシグネチャ内で `name: (params) -> Return`** | **RFC-010 と統一** |

### 付録B：用語集

| 用語 | 定義 |
|------|------|
| バインディング位置 | 関数引数リスト内のインデックス位置 |
| 連合バインディング | 型を複数の引数位置にバインディング |
| 部分適用 | 一部の引数のみを提供返し、未完の呼び出しの関数を返す |
| **統一構文** | **`name: (params) -> Return = body`、引数名はシグネチャ内で宣言** |
| **名称空間関数** | **`Type.name` 構文、関数は Type の名称空間に属し、暗黙的バインディングを含まない** |
| **明示的バインディング** | **`Type.name = func[n]`、唯一のメソッドバインディング機構** |

---

## 参考文献

- [Rust impl 構文](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 型クラス](https://wiki.haskell.org/Type_class)
- [Kotlin 拡張関数](https://kotlinlang.org/docs/extensions.html)
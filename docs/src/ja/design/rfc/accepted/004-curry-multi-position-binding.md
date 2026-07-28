---
title: 'RFC-004: カーリー化メソッドの位置拘束構文設計'
status: '承認済み'
author: '晨煦'
created: '2025-01-05'
updated: '2026-02-18（組込み拘束と後置拘束構文を追加）'
issue: '#132'
---

# RFC-004: カーリー化メソッドの位置拘束設計

## 概要

本 RFC は、関数を型の任意のパラメータ位置に正確に拘束することを可能にする新しい**位置拘束構文**を提案する。単位置拘束と多位置拘束の両方をサポートし、カーリー化拘束における「誰が呼び出し元か」という問題を根本的に解決する。`self`
キーワードを導入する必要はない。

## 動機

### なぜこの機能が必要か？

現在の言語設計では、独立した関数を型メソッドとして拘束する際に以下の問題を抱えている：

1. **呼び出し元の位置が柔軟でない**：従来の拘束では `obj.method(args)` の `obj`
   を最初の引数に固定するだけ
2. **複数引数拘束が困難**：メソッドが同じ型の複数の引数を受け取る必要がある場合、優雅に表現できない
3. **カーリー化セマンティクスの曖昧性**：部分適用時に「どの位置に拘束されているか」を区別しにくい

### 設計目標：2つのプログラミング視点の統一

本設計は**関数型とOOPの2つのプログラミング視点を統一する**ことを目指す：

```yaoxiang
# 関数視点：すべての引数を明示的に渡す
distance(p1, p2)

# OOP視点：暗黙的な this
p1.distance(p2)

# [positions] 糖衣構文により2つの写法は同等、本質的にはどちらも関数呼び出し
Point.distance = distance[0]   # this を位置 0 に拘束
```

**核となる価値**：

- 基盤は関数、上層はメソッド構文
- `self` キーワードを導入しないことで言語のシンプルさを維持
- 完全に関数化：メソッド呼び出しの本質は引数渡し
- `[0]`, `[1]`, `[-1]` で this 拘束位置を柔軟に制御
- **構文の統一**：関数定義では `name: (params) -> Return = body` 形式を使用

### 現在の問題

```yaoxiang
# 既存設計の問題：
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# 最初の引数にしか拘束できない
Point.distance = distance  # distance[0] と等価
# p1.distance(p2) → distance(p1, p2) ✓

# しかし transform のシグネチャが transform(Vector, Point) だったら？
# p1.transform(v1) → transform(v1, p1) のセマンティクスを表現できない
```

## 提案

### コア設計：明示的な位置指定

**コアルール：`[n]` を書かない = 拘束なし。** `Point.name = func`
は単なる名前空間エイリアスであり、暗黙的な拘束をトリガーしない。`p.name(args)` のような `.`
呼び出し構文を機能させるには、明示的に `Point.name = func[n]` と指定する必要がある。

#### 単位置拘束

```yaoxiang
# 最初の Point 引数位置（インデックスは0から開始）に明示的に拘束
Point.distance = distance[0]
p1.distance(p2)                     # → distance(p1, p2)

# 2番目の Point 引数位置に拘束
Point.compare = distance[1]         # 2番目の Point パラメータに拘束
p1.compare(p2)                      # → distance(p2, p1)
```

**`[n]` を書かない = 拘束なし**：

```yaoxiang
# [n] がない場合 → 純粋な名前空間エイリアス、. 呼び出し構文なし
Point.distance = distance            # Point.distance(p1, p2) のみ
# p1.distance(p2)  ❌  拘束なし

# ファクトリ関数は自然に合法であり、特別な処理不要
create_point: () -> Point = { ... }
Point.create = create_point          # Point.create()   ✅
```

- 型安全性：型が一致するものだけが拘束され、ミスを防ぐ
- 柔軟な制御：`[n]` により拘束位置を精密に制御

#### カーリー化拘束

関数パラメータ数 > 拘束位置数の場合、カーリー化関数が自動的に生成される。**拘束は常に明示的な操作である。**

```yaoxiang
Point: Type = { x: Float, y: Float }

# 基本関数
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# 位置 0 に明示的に拘束 → カーリー化：残余パラメータ factor は呼び出し元が提供
Point.scale = scale[0]

# 呼び出し
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0)

# チェーン呼び出しがより優雅
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### 位置インデックス拘束構文

関数のパラメータと型の拘束関係を精密に制御するための `[position]` 構文を導入する：

```yaoxiang
# 構文形式：Type.method = function[positions]

# === 基本拘束 ===

# 単位置拘束
Point.distance = distance[1]           # 第1引数に拘束（インデックスは0から開始）
# 使用：p1.distance(p2) → distance(p2, p1)

# 多位置拘束（タプルデストラクト）
Point.transform = transform[1, 2]      # 第1, 2引数に拘束
# 使用：p1.transform(v1) → transform(v1, p1)
# 元の関数シグネチャ：transform(Point, Vector) → Point
# 拘束後：Point.transform(Vector) → Point
```

### 詳細な構文定義

```
拘束宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'

位置リスト ::= 位置 (',' 位置)*
位置     ::= 整数                    # プレースホルダ
           | '_'                    # この位置をスキップ（プレースホルダ）
           | 整数 '..' 整数         # 位置範囲（将来拡張用）

関数名   ::= 識別子
型     ::= 識別子 (ジェネリクス引数)?
```

### 組込み拘束

拘束は独立した拘束宣言なしで、型定義体内直接に記述できる：

```yaoxiang
# 方法1：型定義体内で直接拘束
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # 位置0に拘束
}

# 方法2：無名関数 + 位置拘束
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

**カーリー化セマンティクス**：

- `distance = distance[0]` を拘束する際、元の関数シグネチャは `(a: Point, b: Point) -> Float`
- 生成される method シグネチャ：`b: Point -> Float`（位置0は呼び出し元が埋める）

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

# 拘束：Point.distance = distance[1]
# 呼び出し：p1.distance(p2) → distance(p2, p1)
# しかし p1.distance(p2) → distance(p1, p2) が欲しいので：
Point.distance = distance[0]

# 2. 変換操作（多位置拘束）
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# 拘束 Point.transform = transform[1]
# 呼び出し：p.transform(v) → transform(v, p) ❌
# 拘束 Point.transform = transform[0]
# 呼び出し：p.transform(v) → transform(p, v) ✓

# 3. 複雑な複数パラメータ関数
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# 第1引数（Point型）のみを拘束、第3引数を保持
Point.scale = multiply[0, _]
# 呼び出し：p.scale(2.0) → multiply(p, 2.0)

# 4. 異型間拘束
Circle: Type = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# Circle 型に距離メソッドを拘束
Circle.distance = distance[0, 1]
# 呼び出し：c1.distance(c2) → distance(c1, c2)
```

### タプルデストラクトサポート

```yaoxiang
# === タプルデストラクト拘束 ===

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

# 自動デストラクト拘束：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# 使用：coord.describe() → process_coordinates((coord.x, coord.y))
```

### 複数戻り値拘束

```yaoxiang
# === 複数戻り値拘束 ===

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

### 型検査ルール

```rust
fn check_binding_type_compatibility(
    binding: &Binding,
    func: &Function
) -> Result<(), TypeError> {
    // 1. 自動位置検索の場合（明示的に指定されていない）、一致するものが見つかったか確認
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

    // 3. 拘束位置の型互換性を検査
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. メソッド呼び出し引数が残余引数と一致するか確認
    Ok(())
}
```

### ランタイム動作

| シナリオ         | 拘束構文                       | 呼び出し                 | 変換先             |
| ---------------- | ------------------------------ | ------------------------ | ------------------ |
| 拘束なし         | `Point.distance = distance`    | `Point.distance(p1, p2)` | `distance(p1, p2)` |
| 単位置           | `Point.distance = distance[0]` | `p1.distance(p2)`        | `distance(p1, p2)` |
| 単位置           | `Point.distance = distance[1]` | `p1.distance(p2)`        | `distance(p2, p1)` |
| 負数インデックス | `Point.test = func[-1]`        | `p.test(a, b)`           | `func(a, b, p)`    |
| 多位置(カーリー) | `Point.scale = scale[0]`       | `p.scale(2.0)`           | `scale(p, 2.0)`    |
| プレースホルダ   | `Type.method = func[1]`        | `obj.method(arg)`        | `func(arg, obj)`   |

**説明**：

- **拘束なし**：`Point.name = func` は単なる名前空間エイリアスであり、`.` 呼び出し構文なし
- `[0]`：呼び出し元を位置 0（第1引数）に拘束
- `[1]`：呼び出し元を位置 1（第2引数）に拘束
- `[-1]`：呼び出し元を最後の位置（末尾からカウント）に拘束

## トレードオフ

### 利点

- **明示的な拘束**：`[n]` が唯一の拘束メカニズムであり、書かない場合は拘束なし、暗黙的な動作なし
- **精密な制御**：任意のパラメータ位置に拘束でき、柔軟性が高い
- **型安全**：コンパイル時に完全な型検査が行われ、型が一致するものだけが拘束される
- **簡潔な構文**：`[position]` 構文は直感的で理解しやすい
- **`self` キーワードなし**：言語のシンプルさを維持
- **カーリー化フレンドリー**：部分適用とチェーン呼び出しを自然にサポート
- **OOPフレンドリー**：自動カーリー化によりOOP開発者は移行が容易

### 欠点

- **学習コスト**：位置インデックス概念の理解が必要
- **コンパイル複雑度**：拘束解析と型検査がコンパイラの複雑度を増す
- **デバッグ難易度**：エラー情報は拘束位置の問題を明確に示す必要がある

## 代替案

| 案                   | 説明                                 | 採用しない理由                                      |
| -------------------- | ------------------------------------ | --------------------------------------------------- |
| `self` キーワード    | Python/Rust スタイルの `self` を導入 | YaoXiang の暗黙的 `self` なしという設計哲学に反する |
| 名前付き引数拘束     | 名前付き引数 `func(a=obj)` を使用    | 関数シグネチャ定義の修正が必要になり、複雑性が増す  |
| マクロシステム       | マクロで拘束を実装                   | ランタイムオーバーヘッドが大きく、型安全性が低下    |
| 演算子オーバーロード | `self` を特定位置に制限              | 構文が統一されず、セマンティクスが混乱              |

## 実装戦略

### フェーズ分け

1. **Phase 1: 基本拘束**（v0.3）
   - 単位置 `[n]` 拘束構文を実装（n は 0 から開始、負数もサポート）
   - 基本的な型検査とコード生成
   - ユニットテストによるカバレッジ

2. **Phase 2: 上級機能**（v0.5）
   - 範囲構文 `[n..m]` のサポート
   - コンパイル時位置計算の最適化

### 依存関係

- 外部依存なし
- RFC-001（エラー処理）との直接的な関連なし
- 独立して実装可能

### リスク

- 既存の拘束構文との互換性処理
- パフォーマンス最適化戦略（コンパイル時展開 vs ランタイム検索）

## 未解決の問題

以下の問題は設計で既に解決済みであり、付録Aに記録されている：

- ~~位置インデックスは 0 から開始~~ → 決定済み：0 から開始
- ~~負数インデックス~~ → 決定済み：サポート
- ~~プレースホルダ~~ → 決定済み：`_` を使用
- ~~範囲構文~~ → 決定済み：実装

**残りの未解決の問題**：

- [ ] 既存の拘束構文との互換性処理
- [ ] パフォーマンス最適化戦略（コンパイル時展開 vs ランタイム検索）

---

## 付録

### 付録A：設計決定の記録

| 決定             | 決定内容                                                        | 理由                                        |
| ---------------- | --------------------------------------------------------------- | ------------------------------------------- |
| インデックス基準 | 0 から開始                                                      | タプル/パラメータリストのインデックスと一致 |
| 負数インデックス | サポート                                                        | 柔軟性が高く、末尾からカウント可能          |
| プレースホルダ   | `_`                                                             | 簡潔で汎用的な記号                          |
| 範囲構文         | 実装                                                            | 一括拘束、`[0..2]` など                     |
| 構文スタイル     | 中置 `Type.method = func[positions]`                            | RFC-010 と統一                              |
| **拘束ルール**   | **明示的な `[n]` でのみ拘束、`[n]` なしでは拘束なし**           | **暗黙的な動作なし、関数定義と拘束は直交**  |
| **名前空間**     | **`Type.name` は名前空間所属を表すだけで拘束をトリガーしない**  | **定義と拘束の分離**                        |
| **関数構文**     | **パラメータ名はシグネチャ内で宣言 `name: (params) -> Return`** | **RFC-010 と統一**                          |

### 付録B：用語集

| 用語             | 定義                                                                     |
| ---------------- | ------------------------------------------------------------------------ |
| 拘束位置         | 関数パラメータリスト内のインデックス位置                                 |
| 聯合拘束         | 型を複数のパラメータ位置に拘束すること                                   |
| 部分適用         | 一部の引数のみを提供し、未完了の呼び出しを返す                           |
| **統一構文**     | **`name: (params) -> Return = body`、パラメータ名はシグネチャ内で宣言**  |
| **名前空間関数** | **`Type.name` 構文、関数は型の名前空間に属し、暗黙的な拘束を意味しない** |
| **明示的拘束**   | **`Type.name = func[n]`、唯一の方法拘束メカニズム**                      |

---

## 参考文献

- [Rust impl 構文](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 型クラス](https://wiki.haskell.org/Type_class)
- [Kotlin 拡張関数](https://kotlinlang.org/docs/extensions.html)

```markdown
---
title: "RFC-004: カリー化メソッドの複数位置結合バインディング設計"
status: "承認済み"
author: "晨煦"
created: "2025-01-05"
updated: "2026-02-18（組み込みバインディングと後置バインディング構文を追加）"
issue: "#132"
---

# RFC-004: カリー化メソッドの複数位置結合バインディング設計

## 要約

本RFCは、関数を型の任意のパラメータ位置に正確にバインドできる全く新しい**複数位置結合バインディング**構文を提案する。単一位置バインディングと複数位置結合バインディングをサポートし、カリー化バインディングにおける「誰が呼び出し元か」という問題を、`self`キーワードを導入することなく根本的に解決する。

## 動機

### なぜこの機能が必要なのか？

現在の言語設計では、独立した関数を型のメソッドにバインドする際に以下の問題に直面している：

1. **呼び出し元の位置が柔軟でない**：伝統的なバインディングでは、`obj.method(args)`の`obj`を最初のパラメータに固定することしかできない
2. **複数パラメータのバインドが困難**：メソッドが複数の同型パラメータを受け取る必要がある場合、エレガントに表現できない
3. **カリー化のセマンティクスが曖昧**：部分適用時に「どの位置にバインドされるか」を区別しにくい

### 設計目標：二つのプログラミング視点を統一する

本設計は**関数型とOOPという二つのプログラミング視点を統一する**ことを目的とする：

```yaoxiang
# 関数視点：全てのパラメータを明示的に渡す
distance(p1, p2)

# OOP視点：暗黙のthis
p1.distance(p2)

# [positions]構文糖衣により二つの書き方が等価になり、本質的にはどちらも関数呼び出し
Point.distance = distance[0]   # thisを0番目の位置にバインド
```

**中核的価値**：
- 根底は関数、上層はメソッド構文
- `self`キーワードを導入せず、言語の簡潔さを維持
- 完全な関数型：メソッド呼び出しは本質的にパラメータ渡し
- `[0]`, `[1]`, `[-1]`によりthisのバインド位置を柔軟に制御
- **構文の統一**：関数定義は `name: (params) -> Return = body` 形式を使用

### 現在の問題

```yaoxiang
# 既存設計の問題：
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# 最初のパラメータにしかバインドできない
Point.distance = distance  # distance[0] と等価
# p1.distance(p2) → distance(p1, p2) ✓

# しかし、もし transform のシグネチャが transform(Vector, Point) だったら？
# p1.transform(v1) → transform(v1, p1) という意味を表現できない
```

## 提案

### 中核設計：明示的な位置指定

**中核ルール：`[n]` を書かない = バインドしない。** `Point.name = func` は単なる名前空間の別名であり、暗黙のバインドは一切発生しない。`p.name(args)` のような `.` 呼び出し構文を有効にするには、必ず明示的に指定する必要がある：`Point.name = func[n]`。

#### 単一位置バインディング

```yaoxiang
# 最初の Point パラメータ位置に明示的にバインド（インデックスは0から開始）
Point.distance = distance[0]
p1.distance(p2)                     # → distance(p1, p2)

# 2番目の Point パラメータ位置にバインド
Point.compare = distance[1]         # 2番目の Point パラメータにバインド
p1.compare(p2)                      # → distance(p2, p1)
```

**`[n]` を書かない = バインドしない**：

```yaoxiang
# [n] なし → 純粋な名前空間の別名、. 呼び出し構文はなし
Point.distance = distance            # Point.distance(p1, p2) のみ
# p1.distance(p2)  ❌  バインドされていない

# ファクトリ関数は自然に合法、特別な処理は不要
create_point: () -> Point = { ... }
Point.create = create_point          # Point.create()   ✅
```
- 型安全：型が一致する場合のみバインドし、エラーを回避
- 柔軟な制御：`[n]` によりバインド位置を正確に制御

#### カリー化バインディング

関数のパラメータ数がバインド位置数より多い場合、自動的にカリー化関数が生成される。**バインドは常に明示的な操作である。**

```yaoxiang
Point: Type = { x: Float, y: Float }

# 基本関数
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# 位置0に明示的にバインド → カリー化：残りのパラメータ factor は呼び出し元が提供
Point.scale = scale[0]

# 呼び出し
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0)

# チェーン呼び出しがよりエレガントになる
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### 位置インデックスバインディング構文

`[position]` 構文を導入し、関数のパラメータと型のバインド関係を正確に制御する：

```yaoxiang
# 構文形式：Type.method = function[positions]

# === 基本バインディング ===

# 単一位置バインディング
Point.distance = distance[1]           # 1番目のパラメータにバインド（インデックスは0から開始）
# 使用：p1.distance(p2) → distance(p2, p1)

# 複数位置結合バインディング（タプル分解）
Point.transform = transform[1, 2]      # 1, 2番目のパラメータにバインド
# 使用：p1.transform(v1) → transform(v1, p1)
# 元の関数のシグネチャ：transform(Point, Vector) → Point
# バインド後：Point.transform(Vector) → Point
```

### 詳細な構文定義

```
バインディング宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'

位置リスト ::= 位置 (',' 位置)*
位置     ::= 整数                    # プレースホルダ
           | '_'                    # この位置をスキップ（プレースホルダ）
           | 整数 '..' 整数         # 位置範囲（将来の拡張）

関数名   ::= 識別子
型       ::= 識別子 (ジェネリクスパラメータ)?
```

### 組み込みバインディング

バインディングは型定義本体内で直接記述でき、個別のバインディング文は不要：

```yaoxiang
# 方式1：型定義本体内で直接バインド
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # 位置0にバインド
}

# 方式2：無名関数 + 位置バインディング
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

**カリー化のセマンティクス**：
- `distance = distance[0]` をバインドする際、元の関数のシグネチャは `(a: Point, b: Point) -> Float`
- 生成される method のシグネチャ：`b: Point -> Float`（0番目は呼び出し元が埋める）

### 使用例

```yaoxiang
# === 完全な使用例 ===

Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

# 1. 基本的な距離計算
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

# バインディング：Point.distance = distance[1]
# 呼び出し：p1.distance(p2) → distance(p2, p1)
# しかし、p1.distance(p2) → distance(p1, p2) を実現したいので：
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

# 1番目のパラメータ（Point型）のみにバインド、3番目のパラメータを保持
Point.scale = multiply[0, _]
# 呼び出し：p.scale(2.0) → multiply(p, 2.0)

# 4. 跨型バインディング
Circle: Type = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# 距離メソッドを Circle 型にバインド
Circle.distance = distance[0, 1]
# 呼び出し：c1.distance(c2) → distance(c1, c2)
```

### タプル分解サポート

```yaoxiang
# === タプル分解バインディング ===

# 関数がタプルパラメータを受信する
process_coordinates: (coord: (Float, Float)) -> String = {
    return match coord {
        (0.0, 0.0) -> "origin"
        (x, 0.0) -> "on x-axis at ${x}"
        (0.0, y) -> "on y-axis at ${y}"
        (x, y) -> "point at (${x}, ${y})"
    }
}

Coord: Type = { x: Float, y: Float }

# 自動分解バインディング：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# 使用：coord.describe() → process_coordinates((coord.x, coord.y))
```

### 複数戻り値バインディング

```yaoxiang
# === 複数戻り値バインディング ===

min_max: (list: List(Int)) -> (Int, Int) = {
    min = list.reduce(Int.MAX, (a, b) => if a < b then a else b)
    max = list.reduce(Int.MIN, (a, b) => if a > b then a else b)
    return (min, max)
}

List.range: (T:Type)->((self: List(T)) -> (T, T)) = min_max[1]
# 使用：(min_val, max_val) = list.range()
```

## 詳細設計

### コンパイラ実装

### 型検査ルール

```rust
fn check_binding_type_compatibility(
    binding: &Binding,
    func: &Function
) -> Result<(), TypeError> {
    // 1. 自動位置検索の場合（明示的に指定されていない）、一致が見つかるかをチェック
    if binding.positions.is_empty() {
        return Err(TypeError::NoMatchingParameter(
            binding.type_name.clone(),
            func.name.clone()
        ));
    }

    // 2. 全ての位置インデックスが有効であることを検証
    for pos in &binding.positions {
        if *pos >= func.params.len() {
            return Err(TypeError::InvalidBindingPosition(*pos));
        }
    }

    // 3. バインド位置の型互換性をチェック
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. メソッド呼び出しパラメータが残りのパラメータと一致するかをチェック
    Ok(())
}
```

### ランタイム動作

| シナリオ | バインディング構文 | 呼び出し | 変換 |
|------|---------|------|--------|
| バインドなし | `Point.distance = distance` | `Point.distance(p1, p2)` | `distance(p1, p2)` |
| 単一位置 | `Point.distance = distance[0]` | `p1.distance(p2)` | `distance(p1, p2)` |
| 単一位置 | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| 負数インデックス | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| 複数位置（カリー化） | `Point.scale = scale[0]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| プレースホルダ | `Type.method = func[1]` | `obj.method(arg)` | `func(arg, obj)` |

**説明**：
- **バインドなし**：`Point.name = func` は単なる名前空間の別名であり、`.` 呼び出し構文はない
- `[0]`：呼び出し元を0番目（最初のパラメータ）にバインド
- `[1]`：呼び出し元を1番目（2番目のパラメータ）にバインド
- `[-1]`：呼び出し元を最後の位置（末尾から数えて）にバインド

## トレードオフ

### 利点

- **明示的バインド**：`[n]` が唯一のバインド機構であり、書かない場合はバインドされず、暗黙の動作は発生しない
- **正確な制御**：任意のパラメータ位置にバインド可能で、柔軟性が高い
- **型安全**：コンパイル時に完全に型チェックされ、型が一致する場合のみバインド
- **簡潔な構文**：`[position]` 構文は直感的で理解しやすい
- **`self` キーワードが不要**：言語の簡潔さを維持
- **カリー化に親和的**：部分適用とチェーン呼び出しを自然にサポート
- **OOP に親和的**：自動カリー化によりOOPプログラマーが違和感なく移行可能

### 欠点

- **学習コスト**：位置インデックスの概念を理解する必要がある
- **コンパイラの複雑さ**：バインド解析と型検査によりコンパイラの複雑さが増す
- **デバッグの難しさ**：エラーメッセージがバインド位置の問題を明確に指し示す必要がある

## 代替案

| 案 | 説明 | 採用しない理由 |
|------|------|-----------|
| `self` キーワード | Python / Rust スタイルの `self` を導入 | YaoXiang の暗黙の `self` を排除する設計哲学に反する |
| 名前付きパラメータバインディング | 名前付きパラメータ `func(a=obj)` を使用 | 関数のシグネチャ定義を変更する必要があり、複雑さが増す |
| マクロシステム | マクロを用いてバインディングを実装 | ランタイムコストが大きく、型安全性が低下する |
| 演算子オーバーロード | 特定の位置に `self` を限定 | 構文が統一されず、セマンティクスが混乱する |

## 実装戦略

### フェーズ分け

1. **フェーズ1：基本バインディング**（v0.3）
   - 単一位置 `[n]` バインディング構文の実装（n は0から開始、負数をサポート）
   - 基本的な型検査とコード生成
   - ユニットテストによる網羅

2. **フェーズ2：高度な機能**（v0.5）
   - 範囲構文 `[n..m]` のサポート
   - コンパイル時の位置計算の最適化

### 依存関係

- 外部依存なし
- RFC-001（エラー処理）との直接的な関連なし
- 独立して実装可能

### リスク

- 既存のバインディング構文との互換性処理
- パフォーマンス最適化戦略（コンパイル時展開 vs ランタイム検索）

## 未解決問題

以下の問題は既に設計で解決済みであり、付録Aに記録されている：

- ~~位置インデックスが0から開始~~ → 決定：0から開始
- ~~負数インデックス~~ → 決定：サポートする
- ~~プレースホルダ~~ → 決定：`_` を使用する
- ~~範囲構文~~ → 決定：実装する

**残りの未解決問題**：

- [ ] 既存のバインディング構文との互換性処理
- [ ] パフォーマンス最適化戦略（コンパイル時展開 vs ランタイム検索）

---

## 付録

### 付録A：設計決定記録

| 決定 | 結果 | 理由 |
|------|------|------|
| インデックスの基準 | 0から開始 | タプル / パラメータリストのインデックスと一致 |
| 負数インデックス | サポート | 柔軟で、末尾からカウント可能 |
| プレースホルダ | `_` | 簡潔で汎用的な記号 |
| 範囲構文 | 実装 | 一括バインディングのため（例：`[0..2]`） |
| 構文スタイル | 中置 `Type.method = func[positions]` | RFC-010 と統一 |
| **バインドルール** | **明示的な `[n]` のみバインド、書かない場合はバインドしない** | **暗黙の動作を排除し、関数定義とバインドを直交させる** |
| **名前空間** | **`Type.name` は単なる名前空間への所属であり、バインドは発生しない** | **定義とバインドを分離** |
| **関数構文** | **パラメータ名はシグネチャ内で `name: (params) -> Return`** | **RFC-010 と統一** |

### 付録B：用語集

| 用語 | 定義 |
|------|------|
| バインド位置 | 関数のパラメータリスト内のインデックス位置 |
| 結合バインディング | 型を複数のパラメータ位置にバインドすること |
| 部分適用 | 一部のパラメータのみを提供し、未完了の呼び出しを返す関数 |
| **統一構文** | **`name: (params) -> Return = body`、パラメータ名はシグネチャ内で宣言** |
| **名前空間関数** | **`Type.name` 構文。関数は Type の名前空間に属し、暗黙のバインドを含まない** |
| **明示的バインディング** | **`Type.name = func[n]`、唯一のメソッドバインド機構** |

---

## 参考文献

- [Rust impl 構文](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 型クラス](https://wiki.haskell.org/Type_class)
- [Kotlin 拡張関数](https://kotlinlang.org/docs/extensions.html)
```
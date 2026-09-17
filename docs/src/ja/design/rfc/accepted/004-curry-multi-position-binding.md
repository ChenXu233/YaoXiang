---
title: 'RFC-004: カリー化メソッドの複数位置聯合バインディング設計'
status: '承認済み'
author: '晨煦'
created: '2025-01-05'
updated: '2026-09-16（§カリー化チェーンの終端と括弧の意味論を追加）'
issue: '#132'
---

# RFC-004: カリー化メソッドの複数位置聯合バインディング設計

> **補足（2026-09-16、括弧の意味論）**：本文のカリー化チェーンには、これまで明記されていなかった終端ルールがある。戻り値の**括弧**は「これは完全な型（値）である」ことを宣言し、チェーンはここで終端する：
>
> ```yaoxiang
> f: (a: Int) -> (b: Int) -> Int       # 柯里化：两段参数，f(1)(2)
> g: (a: Int) -> ((b: Int) -> Int)    # 返回函数：一段参数，g(1) 得函数
> ```
>
> 根拠：注釈は即ち型。`g` は `g(1) : (b: Int) -> Int` を宣言しているため、`g(1)`
> は**その関数でなければならない**（「次のパラメータ」ではなく）。これは RFC-010 と言語仕様における
> `map: (T: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` の記述の本来意図と、`stdlib.md`
> のメソッドシグネチャ `-> ((self: ...) -> ...)` と一致している。
>
> 反射的に、括弧なしの入れ子 `Fn` は一律カリー化となる（RFC-011 の `(T: Type) -> (x: T) -> T`
> を含む）。両者の振る舞いは区別可能であり、どちらも使用可能である。
>
> 実装上：`Type::Paren` は「括弧が引数グループを構成しない」（すなわち括弧の後に `->`
> が続かない）場合にのみ生成され、チェーン分割とフォーマッターの 2 箇所でのみ認識され、その他の場面では透過的に扱われる。

## 概要

本 RFC は、関数を型の任意のパラメータ位置に正確にバインドできる全く新しい**複数位置聯合バインディング**構文を提案し、単一位置バインディングと複数位置聯合バインディングをサポートし、`self`
キーワードを導入することなく、カリー化バインディングにおける「呼び出し元は誰か」という問題を根本的に解決する。

## 動機

### なぜこの機能が必要なのか？

現在の言語設計では、独立した関数を型のメソッドにバインドする際に以下の問題がある：

1. **呼び出し元の位置が柔軟でない**：従来のバインディングでは `obj.method(args)` の `obj`
   を最初のパラメータに固定することしかできない
2. **複数パラメータのバインドが困難**：メソッドが同じ型の複数のパラメータを受け取る必要がある場合、优雅に表現できない
3. **カリー化の意味論の曖昧さ**：部分適用時に「どの位置にバインドするか」を区別しにくい

### 設計目標：2 つのプログラミング視点を統一する

本設計は**関数型プログラミングと OOP の 2 つの視点を統一する**ことを目的とする：

```yaoxiang
# 関数視点：すべての引数を明示的に渡す
distance(p1, p2)

# OOP 視点：暗黙の this
p1.distance(p2)

# [positions] 構文糖衣により両方の記法は等価となり、本質的にはどちらも関数呼び出し
Point.distance = distance[0]   # this を位置 0 にバインド
```

**核心的価値**：

- 低層は関数、上層はメソッド構文
- `self` キーワードを導入せず、言語の簡潔さを保つ
- 完全に関数的：メソッド呼び出しは本質的に引数渡し
- `[0]`, `[1]`, `[-1]` で this のバインド位置を柔軟に制御
- **構文の統一**：関数定義は `name: (params) -> Return = body` 形式を使用

### 現在の問題

```yaoxiang
# 既存設計の問題：
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# 最初のパラメータにのみバインド可能
Point.distance = distance  # distance[0] と等価
# p1.distance(p2) → distance(p1, p2) ✓

# しかし transform のシグネチャが transform(Vector, Point) だったら？
# p1.transform(v1) → transform(v1, p1) の意味論を优雅に表現できない
```

## 提案

### 中核設計：明示的な位置指定

**中核ルール：`[n]` を書かない = バインドしない。** `Point.name = func`
は単なる名前空間のエイリアスであり、暗黙のバインディングは一切発生しない。`p.name(args)` という `.`
呼び出し構文を有効にするには、必ず明示的に指定する必要がある：`Point.name = func[n]`。

#### 単一位置バインディング

```yaoxiang
# 最初の Point パラメータ位置に明示的にバインド（インデックスは 0 から開始）
Point.distance = distance[0]
p1.distance(p2)                     # → distance(p1, p2)

# 2 番目の Point パラメータ位置にバインド
Point.compare = distance[1]         # 2 番目の Point パラメータにバインド
p1.compare(p2)                      # → distance(p2, p1)
```

**`[n]` を書かない = バインドしない**：

```yaoxiang
# [n] がない → 純粋な名前空間エイリアスで、. 呼び出し構文もない
Point.distance = distance            # Point.distance(p1, p2) のみ
# p1.distance(p2)  ❌  バインドなし

# ファクトリ関数は自然に合法で、特別な処理は不要
create_point: () -> Point = { ... }
Point.create = create_point          # Point.create()   ✅
```

- 型安全：型が一致する場合のみバインドされ、エラーを回避
- 柔軟な制御：`[n]` でバインド位置を正確に制御

#### カリー化バインディング

関数のパラメータ数がバインド位置より多い場合、カリー化関数が自動生成される。**バインディングは常に明示的な操作である。**

```yaoxiang
Point: Type = { x: Float, y: Float }

# 基本関数
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# 位置 0 に明示的にバインド → カリー化：残りのパラメータ factor は呼び出し側が提供
Point.scale = scale[0]

# 呼び出し
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0)

# チェーン呼び出しの方がより优雅
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### 位置インデックスバインディング構文

`[position]` 構文を導入し、関数のパラメータと型のバインド関係を正確に制御する：

```yaoxiang
# 構文形式：Type.method = function[positions]

# === 基本バインディング ===

# 単一位置バインディング
Point.distance = distance[1]           # 位置 1 にバインド（インデックスは 0 から開始）
# 使用法：p1.distance(p2) → distance(p2, p1)

# 複数位置聯合バインディング（タプル分解）
Point.transform = transform[1, 2]      # 位置 1, 2 にバインド
# 使用法：p1.transform(v1) → transform(v1, p1)
# 元の関数シグネチャ：transform(Point, Vector) → Point
# バインド後：Point.transform(Vector) → Point
```

### 詳細な構文定義

```
バインディング宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'

位置リスト ::= 位置 (',' 位置)*
位置       ::= 整数                    # プレースホルダ
           | '_'                    # この位置をスキップ（プレースホルダ）
           | 整数 '..' 整数         # 位置範囲（将来の拡張）

関数名     ::= 識別子
型         ::= 識別子 (ジェネリクスパラメータ)?
```

### 組み込みバインディング

バインディングは別個のバインディング文なしで、型定義本体内に直接記述できる：

```yaoxiang
# 方法 1：型定義本体内に直接バインド
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # 位置 0 にバインド
}

# 方法 2：無名関数 + 位置バインディング
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

**カリー化の意味論**：

- `distance = distance[0]` をバインドするとき、元関数のシグネチャは `(a: Point, b: Point) -> Float`
- 生成されるメソッドのシグネチャ：`b: Point -> Float`（位置 0 は呼び出し側が埋める）

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

# バインディング：Point.distance = distance[1]
# 呼び出し：p1.distance(p2) → distance(p2, p1)
# しかし p1.distance(p2) → distance(p1, p2) を望んでいるため：
Point.distance = distance[0]

# 2. 変換操作（複数位置バインディング）
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# Point.transform = transform[1] をバインド
# 呼び出し：p.transform(v) → transform(v, p) ❌
# Point.transform = transform[0] をバインド
# 呼び出し：p.transform(v) → transform(p, v) ✓

# 3. 複雑な複数パラメータ関数
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# 1 番目のパラメータ（Point 型）のみをバインドし、3 番目のパラメータを保持
Point.scale = multiply[0, _]
# 呼び出し：p.scale(2.0) → multiply(p, 2.0)

# 4. 型をまたぐバインディング
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

# 自動分解バインディング：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# 使用法：coord.describe() → process_coordinates((coord.x, coord.y))
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
# 使用法：(min_val, max_val) = list.range()
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

    // 2. すべての位置インデックスが有効であることを検証
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

    // 4. メソッド呼び出しのパラメータと残りのパラメータが一致するかをチェック
    Ok(())
}
```

### 実行時の振る舞い

| シナリオ             | バインディング構文             | 呼び出し                 | 変換先             |
| -------------------- | ------------------------------ | ------------------------ | ------------------ |
| バインドしない       | `Point.distance = distance`    | `Point.distance(p1, p2)` | `distance(p1, p2)` |
| 単一位置             | `Point.distance = distance[0]` | `p1.distance(p2)`        | `distance(p1, p2)` |
| 単一位置             | `Point.distance = distance[1]` | `p1.distance(p2)`        | `distance(p2, p1)` |
| 負のインデックス     | `Point.test = func[-1]`        | `p.test(a, b)`           | `func(a, b, p)`    |
| 複数位置（カリー化） | `Point.scale = scale[0]`       | `p.scale(2.0)`           | `scale(p, 2.0)`    |
| プレースホルダ       | `Type.method = func[1]`        | `obj.method(arg)`        | `func(arg, obj)`   |

**説明**：

- **バインドしない**：`Point.name = func` は単なる名前空間エイリアスで、`.` 呼び出し構文はない
- `[0]`：呼び出し元を位置 0（最初のパラメータ）にバインド
- `[1]`：呼び出し元を位置 1（2 番目のパラメータ）にバインド
- `[-1]`：呼び出し元を最後の位置にバインド（末尾から数える）

## トレードオフ

### 利点

- **明示的なバインディング**：`[n]`
  のみがバインディング機構であり、書かない場合はバインドされず、暗黙の振る舞いはない
- **正確な制御**：任意のパラメータ位置にバインド可能で、柔軟性が高い
- **型安全**：コンパイル時に完全に型検査され、型が一致する場合のみバインド
- **構文の簡潔さ**：`[position]` 構文は直感的で理解しやすい
- **`self` キーワード不要**：言語の簡潔さを保つ
- **カリー化フレンドリー**：部分適用とチェーン呼び出しを自然にサポート
- **OOP フレンドリー**：自動カリー化により OOP プログラマが無理なく移行可能

### 欠点

- **学習コスト**：位置インデックスの概念を理解する必要がある
- **コンパイルの複雑さ**：バインディング解析と型検査がコンパイラの複雑さを増す
- **デバッグの難しさ**：エラーメッセージはバインディング位置の問題を明確に指摘する必要がある

## 代替案

| 方案                             | 説明                                        | 採用しない理由                                         |
| -------------------------------- | ------------------------------------------- | ------------------------------------------------------ |
| `self` キーワード                | Python/Rust スタイルの `self` を導入する    | YaoXiang の暗黙の `self` を持たない設計哲学に違反する  |
| 名前付きパラメータバインディング | 名前付きパラメータ `func(a=obj)` を使用する | 関数シグネチャ定義の変更が必要で、複雑さが増す         |
| マクロシステム                   | マクロでバインディングを実装する            | ランタイムのオーバーヘッドが大きく、型安全性が低下する |
| 演算子オーバーロード             | 特定の位置に `self` を制限する              | 構文が統一されず、意味論が混乱する                     |

## 実装戦略

### フェーズ分割

1. **Phase 1: 基本バインディング**（v0.3）
   - 単一位置 `[n]` バインディング構文を実装（n は 0 から開始、負の数をサポート）
   - 基本的な型検査とコード生成
   - ユニットテストによる網羅

2. **Phase 2: 上級機能**（v0.5）
   - 範囲構文 `[n..m]` のサポート
   - コンパイル時の位置計算最適化

### 依存関係

- 外部依存なし
- RFC-001（エラー処理）との直接的な関連なし
- 独立して実装可能

### リスク

- 既存のバインディング構文との互換性処理
- 性能最適化戦略（コンパイル時展開 vs ランタイム検索）

## オープンクエスチョン

以下の問題は設計で既に解決されており、付録 A に記録されている：

- ~~位置インデックスは 0 から開始~~ → 決定済み：0 から開始
- ~~負のインデックス~~ → 決定済み：サポートする
- ~~プレースホルダ~~ → 決定済み：`_` を使用
- ~~範囲構文~~ → 決定済み：実装する

**残りの未解決問題**：

- [ ] 既存のバインディング構文との互換性処理
- [ ] 性能最適化戦略（コンパイル時展開 vs ランタイム検索）

---

## 付録

### 付録 A：設計決定の記録

| 決定                     | 決定内容                                                                     | 理由                                                   |
| ------------------------ | ---------------------------------------------------------------------------- | ------------------------------------------------------ |
| インデックスの基準       | 0 から開始                                                                   | タプル/パラメータリストのインデックスと一致            |
| 負のインデックス         | サポート                                                                     | 柔軟で、末尾からカウントできる                         |
| プレースホルダ           | `_`                                                                          | 簡潔で、汎用的なシンボル                               |
| 範囲構文                 | 実装                                                                         | バッチバインディング、`[0..2]` など                    |
| 構文スタイル             | 中置 `Type.method = func[positions]`                                         | RFC-010 と統一                                         |
| **バインディングルール** | **明示的な `[n]` のみがバインドし、書かない場合はバインドしない**            | **暗黙の振る舞いなし、関数定義とバインディングは直交** |
| **名前空間**             | **`Type.name` は単なる名前空間の所属であり、バインディングを引き起こさない** | **定義とバインディングの分離**                         |
| **関数構文**             | **パラメータ名はシグネチャで宣言 `name: (params) -> Return`**                | **RFC-010 と統一**                                     |

### 付録 B：用語集

| 用語                     | 定義                                                                               |
| ------------------------ | ---------------------------------------------------------------------------------- |
| バインディング位置       | 関数のパラメータリスト内のインデックス位置                                         |
| 聯合バインディング       | 型を複数のパラメータ位置にバインドすること                                         |
| 部分適用                 | 一部のパラメータのみを提供し、呼び出し未完了の関数を返すこと                       |
| **統一構文**             | **`name: (params) -> Return = body`、パラメータ名はシグネチャで宣言**              |
| **名前空間関数**         | **`Type.name` 構文、関数は Type の名前空間に属し、暗黙のバインディングを含まない** |
| **明示的バインディング** | **`Type.name = func[n]`、唯一のメソッドバインディング機構**                        |

---

## 参考文献

- [Rust impl 構文](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 型クラス](https://wiki.haskell.org/Type_class)
- [Kotlin 拡張関数](https://kotlinlang.org/docs/extensions.html)

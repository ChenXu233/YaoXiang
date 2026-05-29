---
title: RFC-010：統一型構文
---

# RFC-010: 統一型構文 - name: type = value モデル

> **状態**: 承認済み
>
> **著者**: 晨煦
>
> **作成日**: 2025-01-20
>
> **最終更新**: 2026-03-21（フェーズ1-4実装完了、Fn/TypeDef/MethodBind を Binding に統一）

## 概要

本 RFC は極めて簡潔な統一型構文モデルを提案する：**すべては `name: type = value`**。

YaoXiang には一つの宣言形式しかない：

```
identifier : type = expression
```

ここで `type` は任意の型式、`expression` は任意の値式。
**`fn`、`struct`、`trait`、`impl` はなく、小文字の `type` キーワードもない（ただし `Type` はメタ型キーワードとして存在する）**。

> **コア設計**：`Type` 自体はジェネリクス型である。`(T: Type) -> Type` は「型パラメータ T を受け取る型」を意味する。

| 概念 | コード記述 | |
|------------|-----------------------------------------------|
| 変数 | `x: Int = 42` |
| 関数 | `add: (a: Int, b: Int) -> Int = a + b` |
| レコード型 | `Point: Type = { x: Float, y: Float }` |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| ジェネリック型 | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| ジェネリック型 | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| メソッド | `Point.draw: (self: Point, s: Surface) -> Void = ...` |
| ジェネリック関数 | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` は言語内で唯一のメタ型キーワードである**。
これは型レベルを标注するために使用され、コンパイラは Type0、Type1、Type2... の区別を自動的に処理し、ユーザーに透過的である。

```yaoxiang
// コア構文：統一 + 区別

// 変数
x: Int = 42

// 関数（パラメータ名はシグネチャ内に）
add: (a: Int, b: Int) -> Int = a + b

// レコード型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// インターフェース（本質的にはフィールドがすべて関数のレコード型）
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// メソッド定義（Type.method 構文を使用）
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

// ジェネリック型（(T: Type) -> Type = 型パラメータを受け取るジェネリック型）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int
}

Map: (K: Type, V: Type) -> Type = {
    keys: Array(K),
    values: Array(V)
}

// 使用例
p: Point = Point(1.0, 2.0)
p.draw(screen)           // 糖衣構文 → Point.draw(p, screen)
s: Drawable = p           // 構造的サブタイピング：Point は Drawable を実装
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## 動機

### なぜこの機能が必要なのか？

現在の型システムには複数の分離された概念が存在する：
- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インターフェース定義構文
- メソッドバインディング構文

これらの概念の間に統一性がなく、構文が断片化し、学習コストが高い。

### 設計目標

1. **究極の統一**：一つの構文ルールで全ての場合をカバー
2. **簡潔で优雅**：`name: type = value` の対称性美学
3. **新しいキーワード不要**：既存の構文要素を再利用
4. **理論的优雅**：型自体が Type 型の値
5. **ジェネリクスフレンドリー**：ジェネリクスシステム（RFC-011）とシームレスに統合

### ジェネリクスシステムとの統合

RFC-010 の統一構文モデルは RFC-011 のジェネリクスシステム設計と**自然に整合**し、ジェネリックパラメータが統一モデルにシームレスに統合できる：

```yaoxiang
// 基本ジェネリクス（RFC-011 フェーズ 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// ジェネリック関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 フェーズ 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約はパラメータ型に携带

// Const ジェネリクス（RFC-011 フェーズ 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：
- RFC-011 フェーズ 1（基本ジェネリクス）は RFC-010 の**強い依存**
- 基本ジェネリクスなしでは、RFC-010 のジェネリック例がコンパイルできない
- 推奨：RFC-011 フェーズ 1 と RFC-010 を同時に実装

## 提案

### コア原則：型コンストラクタ vs 関数/変数

**これは構文のアンビギュイティ解消ルールを決定する重要な設計選択である：**

| 記述 | 意味 | ルール |
|------|------|------|
| **`x: Type = ...`** | 型コンストラクタ | `: Type` が明示的 → 型として強制 |
| **`f = ...`** | 関数または変数 | `: Type` なし → HM が関数/変数として主动的に推論 |

**なぜこの設計なのか？**

`{ ... }` 構文自体にアンビギュイティがある：
- `{ x: Float, y: Float }` は**型リテラル**（レコード型）である可能性がある
- `{ a = 1 + 1 }` は**コードブロック**（実行文、Void を返す）である可能性がある

**アンビギュイティ解消ルール**：
- **あり** `: Type` → 型コンストラクタとして強制解析、`{ ... }` は型リテラル
- **なし** `: Type` → HM が `{ ... }` をコードブロックとして解析し、関数型として推論

```yaoxiang
# ✅ 型コンストラクタ：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void として推論
main = { println("Hello") }

# ❌ エラー：: Type なし、コンパイラは { ... } を型として解析できない
Point = { x: Float, y: Float }  // HM は関数として推論しており、型ではない！
```

---

**統一モデル：identifier : type = expression**

```
├── 変数
│   └── x: Int = 42
│
├── 関数
│   └── add: (a: Int, b: Int) -> Int = a + b  # : Type なし、HM が関数として推論
│
├── レコード型
│   └── Point: Type = { x: Float, y: Float }  # 必ず返す： Type
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必ず返す： Type
│
├── ジェネリック型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず返す： Type
│
├── ジェネリック型（複数パラメータ）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず返す： Type
│
├── メソッド
│   └── Point.draw: (self: Point, surface: Surface) -> Void = ...
│
└── ジェネリック関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数として推論
```

### メタ型レベル（コンパイラ内部）

**コンパイラ内部**では宇宙レベル `level: selfpointnum` を維持（文字列で保存、理論的には無限に延伸可能）。

| Level | 説明 |
|-------|------|
| `Type0` | 日常型（`Int`、`Float`、`Point`） |
| `Type1` | 型コンストラクタ（`List`、`Maybe`） |
| `Type2+` | 高階コンストラクタ |

**ユーザーはこれらの数字を見ない**、`: Type` だけを見る。

> **カリー＝ハワード同型対応**：宇宙レベルの存在は実装の詳細ではなく、論理的整合性の必要条件である。カリー＝ハワード同型対応は型を命題と同等看待し、`Type: Type`（「型の型も型である」）を許可すると、「この文は偽である」に似たラッセル悖論が生じる——型システムではジャイヤール逆説として現れる。YaoXiang の `Type0 / Type1 / Type2…` 分層（つまりマーティン＝レーフ型理論の累積宇宙）は、各型が某一レベルに属することを確認し、`Typeₙ : Typeₙ₊₁` が決して閉じない上昇チェーンを形成し、逆説を根本的に回避する。これは、YaoXiang の型システムがカリー＝ハワード意味で **論理的に整合している** ことを意味する。

### 構文定義

#### 1. 変数宣言

```yaoxiang
// 基本構文
x: Int = 42
name: String = "Alice"
flag: Bool = true

// 型推論（省略可能）
y = 100  // Int として推論
```

#### 2. 関数定義

```yaoxiang
// 完全構文（パラメータ名はシグネチャ内で宣言）
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// パラメータ名付き
greet: (name: String) -> String = {
    return "Hello, ${name}!"
}

// 複数パラメータ
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }
}

// 複数行関数本体
calc2: (x: Float, y: Float) -> Float = {
    if x > y {
        return x
    }
    return y
}
```

#### 返り値ルール

すべての関数は `return` キーワードを明示的に使用して値を返さなければならない（`()` を返す関数を除く）：

```yaoxiang
// Void 以外の返り型 - return を使用する必要がある
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Void 返り型 - return は省略可能（通常は省略）
print: (msg: String) -> Void = {
    // return は不要
}

// 単一行式（直接値を返す、return は不要）
greet: (name: String) -> String = "Hello, ${name}!"

// 複数行関数本体 - return を使用する必要がある
max: (a: Int, b: Int) -> Int = {
    if a > b {
        return a
    } else {
        return b
    }
}
```

#### 3. 型定義

型定義は YaoXiang 統一構文の核心であり、フィールド、デフォルト値、バインディングメソッド、インターフェース実装を含む：


##### 基本型

**レコード型**：フィールドリスト、フィールド型は任意の型式でよい。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値付きフィールド**：フィールドにはデフォルト値を設定でき、構築時に省略可能。

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

使用例：

```yaoxiang
Point() → Point(x=0, y=0)
Point(x=1) → Point(x=1, y=0)
Point(x=1, y=2) → Point(x=1, y=2)
```

**デフォルト値なしフィールド**：構築時に必ず指定する必要がある。

```yaoxiang
Point2: Type = {
    x: Float,
    y: Float
}
```

使用例：
```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### バインディングメソッド

**方法1：型定義体内で直接外部関数をバインディング**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置0にバインディング、カリー化後の method: (b: Point) -> Float
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

**方法2：匿名関数 + 位置バインディング**

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// 構文：((params) => body)[position]
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

##### インターフェース実装

**インターフェース名は型体内記述し、コンパイラが自動的に実装をチェック**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Point: Type = {
    x: Float,
    y: Float,
    Drawable,          // Drawable インターフェースを実装
    Serializable      // Serializable インターフェースを実装
}
```

##### インターフェース定義

**インターフェース = フィールドがすべて関数のレコード型**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空型/空インターフェース
EmptyType: Type = {}
Empty: Type = {}
```

##### メソッド定義（外部）

**型メソッド**：特定の型に関連（Type.method 構文を使用）

```yaoxiang
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}
```

##### メソッドバインディング（外部）

通常メソッドは `[position]` 構文で型にバインディング可能（詳細構文は RFC-004 を参照）。

**手動バインディング**：

```yaoxiang
// 明示的バインディング
Point.distance = distance[0]

// バインディング位置を指定
Point.transform = transform[1]  // this を第1引数にバインディング
```

**複数位置バインディング**：

```yaoxiang
// 複数位置をバインディング（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**リバースバインディング**（型メソッドを通常関数に変換）：

```yaoxiang
// 型メソッドを通常関数に変換
draw_point: (p: Point, surface: Surface) -> Void = Point.draw
```

#### 4. インターフェース合成

```yaoxiang
// インターフェース合成 = 型の交差点
DrawableSerializable: Type = Drawable & Serializable

// 交差点型を使用
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    return item.serialize()
}
```

#### 5. ジェネリック型

```yaoxiang
// 基本ジェネリクス（RFC-011 フェーズ 1）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T:Type)-((self: List(T), item: T) -> Void),
    get: (T:Type)->((self: List(T), index: Int) -> Maybe(T))
}

// 具体的なインスタンス化（RFC-023 構文）
IntList: Type = List(Int)

IntList.push = {
    self.data.append(item)
    self.length = self.length + 1
}

List.push = (type: Type) -> {
    return (self: List(type), item: type) -> {
        self.data.append(item)
        self.length = self.length + 1
    }
}

IntList.push(Int)(self, item)  // 呼び出し例

// ジェネリックメソッド（RFC-023 構文：型パラメータは呼び出し時に自動推論）
List.push: (self: List(T), item: T) -> Void = {
    self.data.append(item)
    self.length = self.length + 1
}

List.get: (self: List(T), index: Int) -> Maybe(T) = {
    if index >= 0 && index < self.length {
        return Maybe.Just(self.data[index])
    } else {
        return Maybe.Nothing
    }
}
```

#### 6. ジェネリック呼び出し構文

ジェネリック型とジェネリック関数の呼び出しは統一して `()` 構文を使用する。`[]` はジェネリックコンテキストでは使用しない。

**コアルール**：

1. **`()` ですべてを適用**：型適用、関数呼び出し、値構築はすべて `()` を使用

```yaoxiang
# 型标注
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から
empty: List(Int) = List()

# ジェネリック関数呼び出し——型はパラメータから自動的に流れる
strings = map(numbers, f)
// T=Int は numbers: List(Int) から
// R=String は f: (Int) -> String から
```

2. **Type は左、値は右**：`name: type = value`——Type パラメータは左側で宣言し、右側は常に具体的な値。空コンテナ `List()` の `T` は左側の型注釈から取得する必要がある。

3. **型情報は一度だけ記述すればよい**——パラメータ宣言時に書けば、コンパイラがそれを流動させる：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左側で一度だけ記述
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String は numbers と f の型から自動的に
```

4. **値構築は要素から型を推論**：

```yaoxiang
x = List(1, 2, 3)       // List(Int) と推論
y = List("a", "b")      // List(String) と推論
z = List()              // ❌ コンパイルエラー：T を推論できない
z: List(Int) = List()   // ✅ T=Int は左側注釈から
```

5. **型エイリアス**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` → `List(1,2,3)`。
> 旧式の `[]` ジェネリック構文は完全に削除された。`[]` は配列/リストリテラルとインデックスアクセスのためだけに使用される。

### 例

#### 完全例

```yaoxiang
// ======== 1. インターフェース定義 ========

Drawable: Type = {
    draw: (self: Self, surface: Surface) -> Void,
    bounding_box: (self: Self) -> Rect
}

Serializable: Type = {
    serialize: (self: Self) -> String
}

Transformable: Type = {
    translate: (self: Self, dx: Float, dy: Float) -> Self,
    scale: (self: Self, factor: Float) -> Self
}

// ======== 2. 型定義 ========

Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable,
    Transformable
}

Rect: Type = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable,
    Serializable,
    Transformable
}

// ======== 3. メソッド定義 ========

// Point のメソッド
draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

bounding_box: (self: Point) -> Rect = {
    return Rect(self.x - 1, self.y - 1, 2, 2)
}

serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

translate: (self: Point, dx: Float, dy: Float) -> Point = {
    return Point(self.x + dx, self.y + dy)
}

scale: (self: Point, factor: Float) -> Point = {
    return Point(self.x * factor, self.y * factor)
}

// 通常メソッド（pub、自動的に Point.distance にバインディング）
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// Rect のメソッド
draw: (self: Rect, surface: Surface) -> Void = {
    surface.draw_rect(self.x, self.y, self.width, self.height)
}

bounding_box: (self: Rect) -> Rect = self

serialize: (self: Rect) -> String = {
    return "Rect(${self.x}, ${self.y}, ${self.width}, ${self.height})"
}

translate: (self: Rect, dx: Float, dy: Float) -> Rect = {
    return Rect(self.x + dx, self.y + dy, self.width, self.height)
}

scale: (self: Rect, factor: Float) -> Rect = {
    return Rect(self.x * factor, self.y * factor, self.width * factor, self.height * factor)
}

// ======== 4. メソッドバインディング ========

Point.distance = distance[0]  // 位置0にバインディング、カリー化後の method: (p2: Point) -> Float
Point.transform = transform[1]  // 位置1にバインディング、カリー化後の method: (dx: Float, dy: Float) -> Point
Rect.transform = transform[1]  // 位置1にバインディング、カリー化後の method: (dx: Float, dy: Float) -> Rect

// ...同理、其他メソッドもバインディング...

// ======== 5. 使用 ========

// インスタンス作成
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// メソッド呼び出し（糖衣構文）
p.draw(screen)
r.draw(screen)

// 通常メソッド呼び出し（直接呼び出し）
d: Float = distance(p, Point(0.0, 0.0))

// チェーン呼び出し
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// インターフェース代入
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// ジェネリック関数（RFC-023 構文：呼び出し時に型パラメータを省略、自动推論）
process_all: (items: List(T)) -> Void = {
    for item in items {
        print(item.serialize())
    }
}

process_all([p, r])
```

## 詳細設計

### インターフェースチェックアルゴリズム

```rust
fn check_type_implements_interface(
    typ: &Type,
    iface: &Type
) -> Result<(), TypeError> {
    // インターフェースの各フィールド（関数フィールド）に対して
    for (field_name, iface_field) in &iface.fields {
        // 型が同名メソッドを持っているかチェック
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換であるかチェック
            // インターフェースフィールド: (Surface) -> Void
            // メソッドシグネチャ: (Point, Surface) -> Void
            // 比較：self パラメータを除いた後、一致する必要がある
            if !method_signature_matches(method, iface_field.type_) {
                return Err(TypeError::MethodSignatureMismatch {
                    type_name: typ.name,
                    interface_name: iface.name,
                    method_name: field_name,
                });
            }
        } else {
            return Err(TypeError::MissingMethod {
                type_name: typ.name,
                interface_name: iface.name,
                method_name: field_name,
            });
        }
    }
    Ok(())
}
```

### インターフェース直接代入とコンパイル時最適化

インターフェース型は直接代入をサポートし、コンパイラは代入の右辺型に基づいて自動的に最適な呼び出し戦略を選択する：

```yaoxiang
// 具体的な型を直接代入 → コンパイル時に具体的な型を特定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：circle_draw(screen) を直接呼び出し、vtable なし

// 関数の返り値 → コンパイル時に具体的な型を特定不可、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable 経由でメソッドを検索

// 異種混合コレクション → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable 経由でメソッドを検索
}
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| `d: Drawable = Circle(1)` | 具体的な型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()` | 不明 | vtable |
| `shapes: List(Drawable) = [...]` | 異種混合 | vtable |

**ルール**：
1. 右辺が具体的な型コンストラクタでコンパイル時に特定可能な場合は、直接呼び出し IR を生成
2. 右辺の型がコンパイル時に特定できない場合は、vtable メカニズムにフォールバック
3. vtable が実行時ポリモーフィズムの正確性を保証

### ダックタイピングサポート

```yaoxiang
// 同じメソッドを持っていれば、インターフェース型に代入可能
CustomPoint: Type = {
    draw: (self: CustomPoint, surface: Surface) -> Void,
    x: Float,
    y: Float
}

custom: CustomPoint = CustomPoint(
    (self: CustomPoint, surface: Surface) => surface.plot(self.x, self.y),
    1.0,
    2.0
)
```

### 構文変更

| 以前 | 以後 |
|------|------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Self, err: (E) -> Self }` |
| `impl` キーワードが必要 | キーワード不要、インターフェース名は型本体に記述 |

## 構文設計説明：名前付き関数は本質的に Lambda の糖衣構文

### コア理解

**名前付き関数と Lambda 式は同じものである！** 唯一の利点は、名前付き関数が Lambda に名前を付けていることである。

```yaoxiang
// この2つは本質的に同一
add: (a: Int, b: Int) -> Int = a + b           // 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全に同等）
```

### 糖衣構文モデル

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**重要な点**：シグネチャがパラメータ型を完全に宣言している場合、Lambda ヘッダーのパラメータ名は冗長になり、省略可能。

### パラメータスコープルール

**パラメータが外層変数をオーバーライド**：シグネチャ内パラメータのスコープは関数本体をオーバーライドし、内部スコープの方が優先順位が高い。

```yaoxiang
x = 10  // 外層変数

double: (x: Int) -> Int = x * 2  // ✅ パラメータ x が外層の x をオーバーライド、結果は 20
```

### 注釈位置の柔軟性

型注釈は以下のいずれかの位置に記述可能で、**少なくとも一方に記述すればよい**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambda ヘッダーのみ | `double = (x: Int) => x * 2` | ✅ 有効 |
| 両方に記述 | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全例

```yaoxiang
// ✅ 推奨：シグネチャ完全、Lambda ヘッダーは省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 有効：Lambda ヘッダーで型を注釈
double = (x: Int) => x * 2

// ✅ 有効：両方に注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の利点

| 特性 | 利点 |
|------|------|
| **簡潔** | シグネチャが完全ならパラメータ名の繰り返し不要 |
| **柔軟** | Lambda 形式を保持、好みのものを使用可能 |
| **一貫性** | 変数宣言 `x: Int = 42` と統一パターンを維持 |
| **直感的** | `name: Type = body` が直接「name という名前、Type 型、body 値」を対応 |

## トレードオフ

### 利点

| 利点 | 説明 |
|------|------|
| 究極の統一 | 一つの構文ルールで全てをカバー |
| 理論的优雅 | 完璧に対称な `name: type = value` |
| 新キーワード不要 | 既存の構文要素を再利用 |
| 実装が容易 | コンパイラは一つの宣言形式のみを処理 |
| 学習が容易 | 1つのパターンを覚えれば全てのコードが記述可能 |
| 拡張が容易 | 新機能は自然にこのモデルに統合可能 |

### 欠点

| 欠点 | 説明 |
|------|------|
| 命名規則 | メソッドは `Type.method` 命名に従う必要がある |
| 冗長 | 完全構文は簡略構文より長いが、推論可能 |
| 学習曲線 | 統一モデルを理解する必要がある |

### 緩和措置

```yaoxiang
// 1. 明確なエラーメッセージ
// コンパイルエラー例：
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. 型推論
// 型を省略可能、コンパイラが推論
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE ヒント
// IDE が不足メソッドを自動的にヒント表示
```


### リスク

| リスク | 影響 | 緩和措置 |
|------|------|----------|
| 解析複雑度 | 統一構文が解析複雑度を増加させる可能性 | 再帰下降解析器を使用 |
| パフォーマンスオーバーヘッド | vtable 検索に余計なオーバーヘッドの可能性 | コンパイル時単相化最適化 |

---

## 隠し要素 🎮：言語の源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 型の方を定義してみよう...
Type: Type = Type
```

**警告**：これは**名状し難い**ものである！

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二、二生三、三生万物。                                   ║
║   易有太极，是生两仪。                                         ║
║                                                              ║
║   Type: Type = Type                                          ║
║   此乃爻象之源，语言之边界。                                   ║
║   编译器在此沉默，哲学在此驻足。                               ║
║                                                              ║
║   感谢你触达语言的哲学边界。                                   ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：コンパイラは `Type: Type = Type` を正しく処理できない（Type0/Type1 宇宙逆説が発生する）が、この「隠し要素」を意図的に保持している——コンパイルしようとすると、言語創始者からの禅的メッセージが届く。これは技術的境界であるだけでなく、YaoXiang の型哲学への敬意でもある。

---

## 付録

### 構文 BNF

```bnf
program ::= statement*

statement ::= declaration | expression

# 統一宣言：name: Type = expression
declaration ::= identifier ':' type_expr '=' expression

# 型式
type_expr ::= identifier
       | identifier '(' type_expr (',' type_expr)* ')'      # 型適用
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # 関数型
       | '{' type_field* '}'                       # レコード/インターフェース型
       | 'Type'                                    # メタ型

type_field ::= identifier ':' type_expr
             | identifier                           # インターフェース制約

# ジェネリックパラメータ：関数型の一部として。例えば (T: Type, R: Type) -> (...)
# 独立した BNF ルールは不要——: Type パラメータは通常の関数パラメータ

# 式
expression ::= literal
              | identifier
              | identifier '(' expression (',' expression)* ')'  # 関数呼び出し / コンストラクタ呼び出し
              | '(' expression (',' expression)* ')'              # タプル
              | expression '.' identifier '(' arguments? ')'    # メソッド呼び出し
              | lambda
              | '{' field ':' expression (',' field ':' expression)* '}'

arguments ::= expression (',' expression)*

lambda ::= '(' parameter_list? ')' '=>' block

block ::= expression | '{' expression* '}'
```

### 用語集

| 用語 | 定義 |
|------|------|
| 宣言 | `name: type = value` 形式の代入文 |
| レコード型 | 名前付きフィールドを含む `{ ... }` 型 |
| インターフェース | フィールドがすべて関数型のレコード型 |
| ジェネリック型 | `Name: (T: Type) -> Type = { ... }` として定義された型で、型パラメータを受け取る |
| 型メソッド | `Type.method` 形式のメソッドで、特定の型に関連付けられている |
| ジェネリック関数 | `(T: Type)` 構文を使用した関数で、型パラメータが最初のパラメータグループ |
| メタ型 | `Type`、言語内で唯一の型レベルマーカー |

---

## ライフサイクルと归宿

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティ議論とフィードバックを募集中
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  承認済み   │    │  却下       │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (元の位置)  │
└─────────────┘    └─────────────┘
```
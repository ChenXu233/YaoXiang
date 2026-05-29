```yaml
---
title: "RFC-010：統一タイプ構文"
---

# RFC-010: 統一タイプ構文 - name: type = value モデル

> **ステータス**: 承認済み
>
> **著者**: 晨煦
>
> **作成日**: 2025-01-20
>
> **最終更新**: 2026-03-21（フェーズ1-4実装完了、Fn/TypeDef/MethodBind を Binding に統一）

## 概要

本 RFC は極限までシンプルな統一タイプ構文モデルを提案する：**すべてが `name: type = value`**。

YaoXiang には1種類の宣言形式しかない：

```
identifier : type = expression
```

ここで `type` は任意の型式、`expression` は任意の値式である。
**`fn`、`struct`、`trait`、`impl`、小文字の `type` キーワードはない**（ただし `Type` はメタタイプキーワードとして存在する）。

> **コア設計**：`Type` 自体がジェネリック型である。`(T: Type) -> Type` は「型パラメータ T を受け取る型」を表す。

| 概念 | コード記述 | 例 |
|------|-----------------------------------------------|-------|
| 変数 | `x: Int = 42` | |
| 関数 | `add: (a: Int, b: Int) -> Int = a + b` | |
| 記録型 | `Point: Type = { x: Float, y: Float }` | |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` | |
| ジェネリック型 | `List: (T: Type) -> Type = { data: Array(T), length: Int }` | |
| ジェネリック型 | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` | |
| メソッド | `Point.draw: (self: Point, s: Surface) -> Void = ...` | |
| ジェネリック関数 | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` | |

**`Type` は言語内で唯一のメタタイプキーワード**である。
これは型レベルを标注するために使用され、コンパイラは Type0、Type1、Type2... の区別を自動的に処理し、ユーザーには透過的である。

```yaoxiang
// コア構文：統一 + 区別

// 変数
x: Int = 42

// 関数（パラメータ名はシグネチャ内に）
add: (a: Int, b: Int) -> Int = a + b

// 記録型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// インターフェース（本質的にはフィールドがすべて関数の記録型）
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
s: Drawable = p           // 構造的サブタイプ：Point は Drawable を実装
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## 動機

### なぜこの機能が必要か？

現在の型システムには複数の分離された概念が存在する：

- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インターフェース定義構文
- メソッドバインディング構文

これらの概念間に統一性がなく、構文が断片化しており、学習コストが高い。

### 設計目標

1. **極限の統一**：1つの構文規則で全ケースをカバー
2. **簡潔でエレガント**：`name: type = value` の対称的な美しさ
3. **新しいキーワード不要**：既存の構文要素を再利用
4. **理論的なエレガントさ**：型自体が Type 型の値である
5. **ジェネリックフレンドリー**：ジェネリックシステム（RFC-011）とシームレスに統合

### ジェネリックシステムとの統合

RFC-010 の統一構文モデルは RFC-011 のジェネリックシステム設計と**本質的に適合**しており、ジェネリックパラメータは統一モデルにシームレスに統合できる：

```yaoxiang
// 基本ジェネリック（RFC-011 フェーズ 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// ジェネリック関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推断）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 フェーズ 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約はパラメータ型が携带

// Const ジェネリック（RFC-011 フェーズ 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：

- RFC-011 フェーズ 1（基本ジェネリック）は RFC-010 の**強い依存**
- 基本ジェネリックなしでは、RFC-010 のジェネリック例はコンパイルできない
- 推奨：RFC-011 フェーズ 1 と RFC-010 は同時に実装する

## 提案

### 基本原則：型構築子 vs 関数/変数

**これは構文の曖昧さ解消規則を決定する重要な設計選択である：**

| 記述 | 意味 | 規則 |
|------|------|------|
| **`x: Type = ...`** | 型構築子 | `: Type` があれば明示的に型と宣言 → 型として強制 |
| **`f = ...`** | 関数または変数 | `: Type` なし → HM が関数/変数に能動的に推断 |

**なぜこの設計なのか？**

`{ ... }` 構文自体に曖昧さがある：

- `{ x: Float, y: Float }` は**型リテラル**（記録型）かもしれない
- `{ a = 1 + 1 }` は**コードブロック**（文を実行し、Void を返す）かもしれない

**曖昧さ解消規則**：

- **`: Type` あり** → 型構築子として強制解釈、`{ ... }` は型リテラル
- **`: Type` なし** → HM が能動的に `{ ... }` をコードブロックとして解釈し、関数型に推断

```yaoxiang
# ✅ 型構築子：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が関数に推断
main = { println("Hello") }

# ❌ エラー：: Type なし、コンパイラは { ... } を型として解釈できない
Point = { x: Float, y: Float }  // HM は関数として推断！型ではない
```

---

**統一モデル：identifier : type = expression**

```
├── 変数
│   └── x: Int = 42
│
├── 関数
│   └── add: (a: Int, b: Int) -> Int = a + b  # : Type なし、HM が関数に推断
│
├── 記録型
│   └── Point: Type = { x: Float, y: Float }  # 戻り値は必ず： Type
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 戻り値は必ず： Type
│
├── ジェネリック型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 戻り値は必ず： Type
│
├── ジェネリック型（多引数）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 戻り値は必ず： Type
│
├── メソッド
│   └── Point.draw: (self: Point, surface: Surface) -> Void = ...
│
└── ジェネリック関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数に推断
```

### メタタイプレベル（コンパイラ内部）

**コンパイラ内部**では宇宙レベル `level: selfpointnum`（文字列で存储、理論的には無限に延長可能）を維持する。

| Level | 説明 |
|-------|------|
| `Type0` | 日常的な型（`Int`、`Float`、`Point`） |
| `Type1` | 型構築子（`List`、`Maybe`） |
| `Type2+` | 高階構築子 |

**ユーザーはこれらの数字を見ない**、単に `: Type` を見るだけである。

> **Curry-Howard 同型対応**：宇宙レベルの存在はエンジニアリングの実装詳細ではなく、論理的整合性の必要条件である。Curry-Howard 同型対応は型を命題と同一視し、`Type: Type`（「型の型も型である」）を許可すると「この文は偽である」に類似した Russell 逆理が生じ、型システムでは Girard 逆理として現れる。YaoXiang の `Type0 / Type1 / Type2…` 分層（Martin-Löf 型理論の累積宇宙）は、各型が某一レベルに属することを確認し、`Typeₙ : Typeₙ₊₁` が決して閉じない上昇チェーンを形成し、逆理を根本的に回避する。これは YaoXiang の型システムが Curry-Howard の意味では **論理学的に整合している** ことを意味する。

### 構文定義

#### 1. 変数宣言

```yaoxiang
// 基本構文
x: Int = 42
name: String = "Alice"
flag: Bool = true

// 型推导（省略可能）
y = 100  // Int に推断
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

#### 戻り規則

すべての関数は値を返すために `return` キーワードを明示的に使用しなければならない（`()` を返す関数を除く）：

```yaoxiang
// Void 以外の戻り型 - return を使用する必要がある
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Void 戻り型 - return は省略可能（通常は省略）
print: (msg: String) -> Void = {
    // return は不要
}

// 単一行式（直接値を返す、return 不要）
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

型定義は YaoXiang 統一構文のコアであり、フィールド、デフォルト値、バインディングメソッド、インターフェース実装を含む：

##### 基本型

**記録型**：フィールドリスト、フィールド型は任意の型式で可以是。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値付きフィールド**：フィールドにはデフォルト値可以是、構築時に省略可能。

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

**デフォルト値なしフィールド**：構築時に提供する必要がある。

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

**方法1：型定義体内で外部関数を直接バインディング**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置 0 にバインディング、カリー化後 method: (b: Point) -> Float
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

**インターフェース名は型体内記述、コンパイラが実装を自動的にチェック**

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

**インターフェース = フィールドがすべて関数の記録型**

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

一般メソッドは `[position]` 構文で型にバインディング可以是（詳細構文は RFC-004 を参照）。

**手動バインディング**：

```yaoxiang
// 明示的バインディング
Point.distance = distance[0]

// バインディング位置を指定
Point.transform = transform[1]  // this を第 1 引数にバインディング
```

**複数位置バインディング**：

```yaoxiang
// 複数位置をバインディング（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**リバースバインディング**（型メソッドから一般関数へ）：

```yaoxiang
// 型メソッドを一般関数に変換
draw_point: (p: Point, surface: Surface) -> Void = Point.draw
```

#### 4. インターフェース組合

```yaoxiang
// インターフェース組合 = 型の交差点
DrawableSerializable: Type = Drawable & Serializable

// 交差点型を使用
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    return item.serialize()
}
```

#### 5. ジェネリック型

```yaoxiang
// 基本ジェネリック（RFC-011 フェーズ 1）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T:Type)-((self: List(T), item: T) -> Void),
    get: (T:Type)->((self: List(T), index: Int) -> Maybe(T))
}

// 具体的インスタンス化（RFC-023 構文）
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

// ジェネリックメソッド（RFC-023 構文：型パラメータは呼び出し時に自動推断）
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

ジェネリック型とジェネリック関数の呼び出しは統一的に `()` 構文を使用する。`[]` はジェネリックコンテキストでは使用しない。

**コア規則**：

1. **`()` がすべてを処理**：型適用、関数呼び出し、値構築はすべて `()` を使用

```yaoxiang
# 型标注
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から
empty: List(Int) = List()

# ジェネリック関数呼び出し——型はパラメータから自動的にフロー
strings = map(numbers, f)
// T=Int は numbers: List(Int) から
// R=String は f: (Int) -> String から
```

2. **Type は左、値は右**：`name: type = value`——Type パラメータは左側で宣言、右側は常に具体的値。空コンテナ `List()` の `T` は左側の型注釈から取得する必要がある。

3. **型情報は1回だけ記述**——パラメータ宣言時に、コンパイラがそれをフローさせる：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左側で1回だけ記述
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String は numbers と f の型から自動取得
```

4. **値構築は要素から型を推断**：

```yaoxiang
x = List(1, 2, 3)       // List(Int) に推断
y = List("a", "b")      // List(String) に推断
z = List()              // ❌ コンパイルエラー：T を推断できない
z: List(Int) = List()   // ✅ T=Int は左側注釈から
```

5. **型エイリアス**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` → `List(1,2,3)`。
> 旧式の `[]` ジェネリック構文は完全に削除された。`[]` は配列/リストリテラルとインデックスアクセスのためだけに使用。

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

// 一般メソッド（pub、自動的に Point.distance にバインディング）
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

Point.distance = distance[0]  // 位置 0 にバインディング、カリー化後 method: (p2: Point) -> Float
Point.transform = transform[1]  // 位置 1 にバインディング、カリー化後 method: (dx: Float, dy: Float) -> Point
Rect.transform = transform[1]  // 位置 1 にバインディング、カリー化後 method: (dx: Float, dy: Float) -> Rect

// ... 同様に、他のメソッドもバインディング ...

// ======== 5. 使用 ========

// インスタンス作成
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// メソッド呼び出し（糖衣構文）
p.draw(screen)
r.draw(screen)

// 一般メソッド呼び出し（直接呼び出し）
d: Float = distance(p, Point(0.0, 0.0))

// チェーン呼び出し
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// インターフェース代入
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// ジェネリック関数（RFC-023 構文：呼び出し時に型パラメータを省略、自動推断）
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
    // インターフェースの各フィールド（関数フィールド）について
    for (field_name, iface_field) in &iface.fields {
        // 型が同名メソッドを持つかチェック
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換するかチェック
            // インターフェースフィールド: (Surface) -> Void
            // メソッドシグネチャ: (Point, Surface) -> Void
            // 比較：self パラメータを除いた後一致するはず
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

インターフェース型は直接代入をサポートし、コンパイラは代入の右辺値型に基づいて最適な呼び出し戦略を自動的に選択する：

```yaoxiang
// 具体的型を直接代入 → コンパイル時に具体的型を特定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：circle_draw(screen) を直接呼び出し、vtable なし

// 関数戻り値 → コンパイル時に具体的型を特定不可能、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable 経由でメソッドを検索

// 異種集合 → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable 経由でメソッドを検索
}
```

**コンパイル時最適化戦略**：

| シナリオ | 推断結果 | 呼び出し方式 |
|----------|----------|--------------|
| `d: Drawable = Circle(1)` | 具体的型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()` | 不明 | vtable |
| `shapes: List(Drawable) = [...]` | 異種 | vtable |

**規則**：

1. 右辺値が具体的型構築子でコンパイル時に特定可能な場合、直接呼び出し IR を生成
2. 右辺値型がコンパイル時に特定できない場合、vtable 機構にフォールバック
3. vtable が実行時多態の正確性を保証

### ダックタイピングサポート

```yaoxiang
// 同じメソッドを持っていれば、インターフェース型に代入可以是
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
| `impl` キーワードが必要 | キーワード不要、インターフェース名は型本体後に記述 |

## 構文設計説明：名前付き関数は本質的に Lambda の糖衣構文

### コア理解

**名前付き関数と Lambda 式は同一个ものである！** 唯一のの違いは：名前付き関数は Lambda に名前を付けたものである。

```yaoxiang
// この2つは本質的に完全同一
add: (a: Int, b: Int) -> Int = a + b           // 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全同等）
```

### 糖衣構文モデル

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**重要ポイント**：シグネチャがパラメータ型を完全に宣言している場合、Lambda ヘッダーのパラメータ名は冗長になり、省略可以是。

### パラメータスコープ規則

**パラメータは外層変数をオーバーライド**：シグネチャ内のパラメータスコープが関数本体をオーバーライド、内部スコープの優先度が高い。

```yaoxiang
x = 10  // 外層変数

double: (x: Int) -> Int = x * 2  // ✅ パラメータ x が外層 x をオーバーライド、結果は 20
```

### 注釈位置は柔軟

型注釈は以下のいずれかの位置に可以是、**少なくとも1箇所に注釈があれば良い**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambda ヘッダーのみ | `double = (x: Int) => x * 2` | ✅ 合法 |
| 両方に注釈 | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全例

```yaoxiang
// ✅ 推奨：シグネチャ完全、Lambda ヘッダー省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda ヘッダーで型を注釈
double = (x: Int) => x * 2

// ✅ 合法：両方に注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計優位性

| 特性 | 優位性 |
|------|--------|
| **簡潔** | シグネチャが完全ならパラメータ名を繰り返し書く必要なし |
| **柔軟** | Lambda 形式を保持、好みに応じて選択 |
| **一貫性** | 変数宣言 `x: Int = 42` と統一パターンを維持 |
| **直感的** | `name: Type = body` は直接的に「name という名前、Type という型、body という値」を対応 |

## トレードオフ

### 長所

| 長所 | 説明 |
|------|------|
| 極限の統一 | 1つの構文規則で全ケースをカバー |
| 理論的なエレガントさ | 完璧に対称的な `name: type = value` |
| 新しいキーワード不要 | 既存の構文要素を再利用 |
| 実装が容易 | コンパイラは1種類の宣言形式のみを処理すれば良い |
| 学習が容易 | 1つのパターンを覚えればすべてのコードが書ける |
| 拡張が容易 | 新機能は自然にこのモデルに統合可以是 |

### 短所

| 短所 | 説明 |
|------|------|
| 命名規則 | メソッドは `Type.method` 命名に従う必要がある |
| 冗長 | 完全構文は簡略構文より長いが、推断可能是 |
| 学習曲線 | 統一モデルを理解する必要がある |

### 緩和措施

```yaoxiang
// 1. 明確なエラーメッセージ
// コンパイルエラー例：
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. 型推断
// 型を省略可能是、コンパイラが推断
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE ヒント
// IDE が不足メソッドを自動提案
```


### リスク

| リスク | 影響 | 緩和措施 |
|--------|------|----------|
| 解析複雑度 | 統一構文が解析複雑度を上げる可能性 | 再帰的下向き解析器を使用 |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドの可能性 | コンパイル時単態化最適化 |

---

## ボーナス 🎮：言語の源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 型定義の型を定義しようとすると...
Type: Type = Type
```

**警告**：これは**名付けざるもの**である！

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二、二生三，三生万物。                                   ║
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

> **注**：コンパイラは `Type: Type = Type` を正しく処理できない（Type0/Type1 宇宙逆理を引き起こす）が、この「ボーナス」を意図的に保持——コンパイルしようとすると、言語創業者からの禅的メッセージを受け取る。これは技術的境界であると同時に、YaoXiang の型哲学への敬意である。

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
       | '{' type_field* '}'                       # 記録/インターフェース型
       | 'Type'                                    # メタ型

type_field ::= identifier ':' type_expr
             | identifier                           # インターフェース制約

# ジェネリックパラメータ：関数型の一部として例えば (T: Type, R: Type) -> (...)
# 独立した BNF 規則は不要——: Type パラメータは通常の関数パラメータ

# 式
expression ::= literal
              | identifier
              | identifier '(' expression (',' expression)* ')'  # 関数呼び出し / 構築子呼び出し
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
| 記録型 | 命名フィールドを含む `{ ... }` 型 |
| インターフェース | フィールドがすべて関数型の記録型 |
| ジェネリック型 | `Name: (T: Type) -> Type = { ... }` として定義された型、型パラメータを受け取る |
| 型メソッド | `Type.method` 形式のメソッド、特定の型に関連 |
| ジェネリック関数 | `(T: Type)` 構文を使用した関数、型パラメータは最初の引数グループ |
| メタ型 | `Type`、言語内で唯一の型レベルマーカー |

---

## ライフサイクルと辿り着く場所

```
┌─────────────┐
│   草案      │  ← 現在のステータス
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティの議論とフィードバックを募集中
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  承認済み   │    │  拒否済み   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (元の位置)  │
└─────────────┘    └─────────────┘
```
```markdown
---
title: "RFC-010: 統一型構文 - name: type = value モデル"
status: "承認済み"
author: "晨煦"
created: "2025-01-20"
updated: "2026-06-05（返り値ルールと {} 意味の更新）"
---

# RFC-010: 統一型構文 - name: type = value モデル


## 概要

本 RFC は、超最小限の統一型構文モデルを提案する：**すべてが `name: type = value`**。

YaoXiang には1種類の宣言形式のみが存在する：

```
identifier : type = expression
```

ここで `type` は任意の型式、`expression` は任意の値式である。
**`fn`、`struct`、`trait`、`impl` は存在しない。小文字の `type` キーワードも存在しない（ただし `Type` としてメタ型キーワードは存在する）**。

> **コア設計**：`Type` 本身就是泛型类型。`(T: Type) -> Type` は「型パラメータ T を受け取る型」を表す。

| 概念       | コード記述                                      |
|------------|-----------------------------------------------|
| 変数       | `x: Int = 42`                                |
| 関数       | `add: (a: Int, b: Int) -> Int = a + b`       |
| 記録型   | `Point: Type = { x: Float, y: Float }`       |
| インターフェース       | `Drawable: Type = { draw: (Surface) -> Void }` |
| ジェネリック型   | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| ジェネリック型   | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| メソッド       | `Point.draw: (self: Point, s: Surface) -> Void = ...` |
| ジェネリック関数   | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` は言語内で唯一のメタ型キーワードである**。
型レベルにラベルを付けるために使用され、コンパイラは Type0、Type1、Type2... の区別を自動的に処理し、ユーザーに透過的である。

```yaoxiang
// コア構文：統一 + 区別

// 変数
x: Int = 42

// 関数（パラメータ名はシグネチャ内）
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
s: Drawable = p           // 構造的サブタイピング：Point は Drawable を実装
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

これらの概念間に統一性がなく、構文が断片化し、学習コストが高い。

### 設計目標

1. **究極的统一**：1つの構文ルールで全ケースをカバー
2. **簡潔で优雅**：对称美学的な `name: type = value`
3. **新しいキーワード不要**：既存の構文要素を再利用
4. **理論的に优雅**：型自体が Type 型の値
5. **ジェネリックに優しい**：ジェネリックシステム（RFC-011）とシームレスに統合

### ジェネリックシステムとの統合

RFC-010の統一構文モデルは RFC-011 のジェネリックシステム設計と**自然に整合**しており、ジェネリックパラメータが統一モデルにシームレスに組み込まれる：

```yaoxiang
// 基本ジェネリック（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// ジェネリック関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約はパラメータ型によって携带

// Constジェネリック（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：
- RFC-011 Phase 1（基本ジェネリック）は RFC-010 の**強い依存**
- 基本ジェネリックなしでは、RFC-010 のジェネリック例はコンパイルできない
- 推奨：RFC-011 Phase 1 と RFC-010 を同時に実装

## 提案

### コア原則：型コンストラクタ vs 関数/変数

**これは構文のアンビギュイティ解消ルールを決定する重要な設計選択である：**

| 記述 | 意味 | ルール |
|------|------|------|
| **`x: Type = ...`** | 型コンストラクタ | `: Type` が明示的に宣言 → 型として強制 |
| **`f = ...`** | 関数または変数 | `: Type` なし → HM が関数/変数として推論 |

**なぜこの設計なのか？**

`{ ... }` 構文自体に曖昧さがある：
- `{ x: Float, y: Float }` は**型リテラル**（記録型）でもあり得る
- `{ a = 1 + 1 }` は**コードブロック**（文を実行し、Void を返す）でもあり得る

**アンビギュイティ解消ルール**：
- **`: Type` あり** → 型コンストラクタとして強制解決、`{ ... }` は型リテラル
- **`: Type` なし** → HM が `{ ... }` をコードブロックとして解決し、関数型として推論

```yaoxiang
# ✅ 型コンストラクタ：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void と推論
main = { println("Hello") }

# ❌ エラー：: Type なし、コンパイラは { ... } を型として解決できない
Point = { x: Float, y: Float }  // HM は関数として推論し、型ではない！
```

---

**統一モデル：identifier : type = expression**

```
├── 変数
│   └── x: Int = 42
│
├── 関数
│   └── add: (a: Int, b: Int) -> Int = a + b  # : Type なし、HM が関数と推論
│
├── 記録型
│   └── Point: Type = { x: Float, y: Float }  # 必ず返り値： Type
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必ず返り値： Type
│
├── ジェネリック型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず返り値： Type
│
├── ジェネリック型（複数パラメータ）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず返り値： Type
│
├── メソッド
│   └── Point.draw: (self: Point, surface: Surface) -> Void = ...
│
└── ジェネリック関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数と推論
```

### メタ型レベル（コンパイラ内部）

**コンパイラ内部**では宇宙レベル `level: selfpointnum` を維持する（文字列で保存、理論的には無限に延伸可能）。

| Level | 説明 |
|-------|------|
| `Type0` | 日常的な型（`Int`、`Float`、`Point`） |
| `Type1` | 型コンストラクタ（`List`、`Maybe`） |
| `Type2+` | 高階コンストラクタ |

**ユーザーはこれらの数字を見ることはない**。見るのは `: Type` のみである。

> **Curry-Howard 同形**：宇宙レベルの存在はエンジニアリング実装の詳細ではなく、論理的整合性の必要条件である。Curry-Howard 同形は型を命題と同等にする。もし `Type: Type`（「型の型も型である」）を許可すると、「この文は偽である」に類似した Russell 逆理が生じ——型システムでは Girard 逆理として出現する。YaoXiang の `Type0 / Type1 / Type2…` 分層（Martin-Löf 型理論の累積宇宙）は、各型が特定のレベルにのみ属することを保証し、`Typeₙ : Typeₙ₊₁` は決して閉じない上昇チェーンを形成し、根本的に逆理を回避する。これは、YaoXiang の型システムが Curry-Howard の意味において **論理的に一貫している** ことを意味する。

### 構文定義

#### 1. 変数宣言

```yaoxiang
// 基本構文
x: Int = 42
name: String = "Alice"
flag: Bool = true

// 型推論（省略可能）
y = 100  // Int と推論
```

#### 2. 関数定義

```yaoxiang
// 単一式形式（直接値を返す、return 不要）
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// コードブロック形式（return で値を返す必要がある）
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// 複数行コードブロック
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }
}

// Void 関数（コードブロック内で return は不要）
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### 返り値ルール

返り値は `=` の右側の形式によって決まる：

| 記述 | 返り値 |
|------|--------|
| `= expr`（波括弧なし） | `expr` を直接返す |
| `= { ... }`（波括弧あり） | `return` が必要，否则は `Void` を返す |

```yaoxiang
# 単一式：直接値を返す、return 不要
add: (a: Int, b: Int) -> Int = a + b

# コードブロック：return で値を返す必要がある
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

# Void 関数：return 不要
print: (msg: String) -> Void = {
    console.write(msg)
}
```

> **設計理由**：`{ ... }` は依存駆動計算ユニットである（以下参照）。その返り値セマンティクスは単一式と異なる。波括弧は複数文のコンテキストを導入するため、「最後の式が返り値かどうか」の曖昧さを解消するために明示的な `return` が必要である。

#### `{}` セマンティクス：依存駆動計算ユニット

`{ ... }` は YaoXiang では単なるコードブロックではない——它是**依存駆動計算ユニット**である。このセマンティクスは関数ボディ、変数初期化、`spawn` で一貫している：

**コアルール**：
- `{}` 内の代入文は記述順序ではなく依存関係で自動ソートされる
- 依存が揃えば即座に実行され、欠落があればブロックして待機
- `return` で明示的に値を返す（返り値ルール参照）

```yaoxiang
# 依存駆動：b は a に依存、コンパイラが自動ソート
result: Int = {
    b = a + 1      # a に依存 → a の後に自動配置
    a = 10         # 依存なし → 先に実行可能
    return b       # 11 を返す
}
```

> **単一式との違い**：`= expr`（波括弧なし）は値を直接返す単純なバインディング；`= { ... }`（波括弧あり）は依存駆動計算コンテキストを導入し、複数文と明示的な `return` を許可する。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一の並列プリミティブである。それは `{}` の依存駆動セマンティクスを利用して自動並列化を実現する：

- `spawn { ... }` 内の直接子の代入は自動的に並列タスクを生成
- 依存が揃ったタスクは即座に並行実行
- 呼び出し元は全サブタスクの完了をブロックして待機

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a との依存なし、並行実行）
    c = process(a, b)         # a, b に依存 → 両方の完了後に実行
    return c
}
// 呼び出し元はここでブロック、spawn ブロック内の全タスク完了まで待機
```

> **詳細定義**：`spawn` の完全なセマンティクス、タスク生成ルール、ブロッキングモデルについては `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型和裸ポインタの操作を定義するために使用する。それは `{}` の return セマンティクスを使用して型定義を上位スコープに返す：

**コアルール**：
- `unsafe {}` 内で型と裸ポインタを操作できる
- `return` で型定義を上位スコープに返す
- 返された型は `unsafe {}` の外で使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 裸ポインタ
    }
    return SqliteDb
}

# SqliteDb は unsafe ブロック外で使用可能
db = sqlite3_open("test.db")

# ❌ コンパイルエラー：handle フィールドには unsafe 権限が必要
handle = db.handle

# ✅ メソッド呼び出し経由で
db.close()
```

> **詳細定義**：`unsafe` の完全なセマンティクス、FFI 型定義、メソッドバインディングについては `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang 統一構文の中核であり、フィールド、デフォルト値、バインディングメソッド、インターフェース実装を含む：

##### 基本型

**記録型**：フィールドリスト、フィールド型は任意の型式でよい。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値付きフィールド**：フィールドにはデフォルト値を持たせられ、構築時に省略可能。

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

使用方法：

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

使用方法：
```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### バインディングメソッド

**方式1：型定義体内で外部関数を直接バインディング**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置0にバインディング、適用後 method: (b: Point) -> Float
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

**方式2：匿名関数 + 位置バインディング**

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

**インターフェース名は型体内記述、コンパイラが自動的に実装をチェック**

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

**インターフェース = 全フィールドが関数の記録型**

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

**型メソッド**：特定の型に関連付け（Type.method 構文を使用）

```yaoxiang
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}
```

##### メソッドバインディング（外部）

通常メソッドは `[position]` 構文で型にバインディングできる（詳細構文は RFC-004 を参照）。

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

**逆バインディング**（型メソッドから通常関数へ）：

```yaoxiang
// 型メソッドから通常関数へ
draw_point: (p: Point, surface: Surface) -> Void = Point.draw
```

#### 4. インターフェース組合

```yaoxiang
// インターフェース組合 = 型の交差
DrawableSerializable: Type = Drawable & Serializable

// 交差型を使用
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    return item.serialize()
}
```

#### 5. ジェネリック型

```yaoxiang
// 基本ジェネリック（RFC-011 Phase 1）
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

ジェネリック型とジェネリック関数の呼び出しは統一的に `()` 構文を使用する。`[]` はジェネリックコンテキストでは使用しない。

**コアルール**：

1. **`()` ですべてを適用**：型適用、関数呼び出し、値構築はすべて `()` を使用

```yaoxiang
# 型注釈
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から来る
empty: List(Int) = List()

# ジェネリック関数呼び出し——型はパラメータから自動的に流れる
strings = map(numbers, f)
// T=Int は numbers: List(Int) から
// R=String は f: (Int) -> String から
```

2. **Type は左、値は右**：`name: type = value`——Type パラメータは左側で宣言、右側は常に具体的値。空コンテナ `List()` の `T` は左側の型注釈から取得する必要がある。

3. **型情報は1回だけ記述すればよい**——パラメータ宣言時に書けば、コンパイラが流動させる：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左側で1回だけ記述
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String は numbers と f の型から自動取得
```

4. **値構築は要素から型を推論**：

```yaoxiang
x = List(1, 2, 3)       // List(Int) と推論
y = List("a", "b")      // List(String) と推論
z = List()              // ❌ コンパイルエラー：T を推論できない
z: List(Int) = List()   // ✅ T=Int は左側注釈から取得
```

5. **型エイリアス**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` → `List(1,2,3)`。
> 旧式の `[]` ジェネリック構文は完全に削除された。`[]` は配列/リストリテラルとインデックスアクセスのみ使用。

### 例

#### 完全な例

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

Point.distance = distance[0]  // 位置0にバインディング、適用後 method: (p2: Point) -> Float
Point.transform = transform[1]  // 位置1にバインディング、適用後 method: (dx: Float, dy: Float) -> Point
Rect.transform = transform[1]  // 位置1にバインディング、適用後 method: (dx: Float, dy: Float) -> Rect

// ...同様に他のメソッドもバインディング...

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
    // インターフェースの各フィールド（関数フィールド）について
    for (field_name, iface_field) in &iface.fields {
        // 型が同名のメソッドを持つかチェック
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換するかチェック
            // インターフェースフィールド: (Surface) -> Void
            // メソッドシグネチャ: (Point, Surface) -> Void
            // 比較：self パラメータを除いた後一致する必要がある
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
// 具体的型を直接代入 → コンパイル時に具体的型を特定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：circle_draw(screen) を直接呼び出し、vtable なし

// 関数返り値 → コンパイル時に具体的型を特定不可、vtable を使用
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
| `d: Drawable = Circle(1)` | 具体的型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()` | 不明 | vtable |
| `shapes: List(Drawable) = [...]` | 異種混合 | vtable |

**ルール**：
1. 右辺が具体的型コンストラクタでコンパイル時に特定できる場合、直接呼び出し IR を生成
2. 右辺型がコンパイル時に特定できない場合、vtable メカニズムにフォールバック
3. vtable が実行時多態の正確性を保証

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
| `impl` キーワードが必要 | キーワード不要、インターフェース名は型体の後に記述 |

## 構文設計説明：名前付き関数は本質的に Lambda の糖衣構文

### コア理解

**名前付き関数と Lambda 式は同じものである！** 唯一のの違いは、名前付き関数が Lambda に名前を付けたものである。

```yaoxiang
// この2つは本質的に同一
add: (a: Int, b: Int) -> Int = a + b           // 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全等同）
```

### 糖衣構文モデル

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**要点**：シグネチャがパラメータ型を完全に宣言している場合、Lambda ヘッダのパラメータ名は冗長になり、省略可能。

### パラメータスコープルール

**パラメータは外層変数をオーバーライド**：シグネチャ内のパラメータスコープは関数ボディをオーバーライドし、内部スコープの方が高優先度。

```yaoxiang
x = 10  // 外層変数

double: (x: Int) -> Int = x * 2  // ✅ パラメータ x が外層 x をオーバーライド、結果は 20
```

### 注釈位置は柔軟

型注釈は以下のいずれかの位置に記述でき、**少なくとも1箇所に記述すればよい**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambda ヘッダのみ | `double = (x: Int) => x * 2` | ✅ 有効 |
| 両方に記述 | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許容 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャ完全、Lambda ヘッダ省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 有効：Lambda ヘッダ内で型を注釈
double = (x: Int) => x * 2

// ✅ 有効：両方に記述
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の優位性

| 特性 | 優位性 |
|------|------|
| **簡潔** | シグネチャが完全ならパラメータ名を繰り返し書く必要がない |
| **柔軟** | Lambda 形式を保持、好みのものを使用可能 |
| **一貫** | 変数宣言 `x: Int = 42` と統一パターンを維持 |
| **直感的** | `name: Type = body` は直接的に「name という名前、Type 型、body 値」に対応する |

## トレードオフ

### 利点

| 利点 | 説明 |
|------|------|
| 究極的统一 | 1つの構文ルールで全ケースをカバー |
| 理論的に优雅 | 対称的な `name: type = value` |
| 新しいキーワード不要 | 既存の構文要素を再利用 |
| 実装が容易 | コンパイラは1種類の宣言形式のみを処理すればよい |
| 学習が容易 | 1つのパターンを覚えれば全コードが書ける |
| 拡張が容易 | 新機能は自然にこのモデルに組み込める |

### 欠点

| 欠点 | 説明 |
|------|------|
| 命名規則 | メソッドは `Type.method` 命名に従う必要がある |
| 冗長 | 完全構文は簡略構文より長いが、推論可能 |
| 学習曲線 | 統一モデルを理解する必要がある |

### 缓解措施

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

| リスク | 影響 | 缓解措施 |
|------|------|----------|
| 解析复杂度 | 統一構文は解析复杂度を高める可能性がある | 再帰下降パーサーを使用 |
| 性能オーバーヘッド | vtable 検索に追加オーバーヘッドの可能性 | コンパイル時単相化最適化 |

---

## 隠し要素 🎮：言語之源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 型の型を定義してみる...
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

> **注**：コンパイラは `Type: Type = Type` を正しく処理できない（Type0/Type1 宇宙逆理が発生）が、この「隠し要素」を意図的に保持している——コンパイルしようとすると、言語創設者からの禅的なメッセージが届く。これは技術的な境界であるだけでなく、YaoXiang の型哲学への敬意である。

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
| 記録型 | 名前付きフィールドを含む `{ ... }` 型 |
| インターフェース | 全フィールドが関数型の記録型 |
| ジェネリック型 | `Name: (T: Type) -> Type = { ... }` として定義された型で、型パラメータを受け取る |
| 型メソッド | `Type.method` 形式のメソッドで、特定の型に関連付け |
| ジェネリック関数 | `(T: Type)` 構文を使用した関数で、型パラメータが最初のパラメータグループ |
| メタ型 | `Type`、言語内で唯一の型レベルラベル |

---

## ライフサイクルと帰趨

```
┌─────────────┐
│   草案      │  ← 現在の状態
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
│ (正式設計)  │    │ (元の位置を保持) │
└─────────────┘    └─────────────┘
```
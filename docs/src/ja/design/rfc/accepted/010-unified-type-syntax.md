---
title: "RFC-010: 統一型構文 - name: type = value モデル"
status: "承認済み"
author: "晨煦"
created: "2025-01-20"
updated: "2026-06-05（戻り値規則と {} セマンティクスを更新）"
issue: "#127"
---
# RFC-010: 統一型構文 - name: type = value モデル


## 概要

本 RFC は極限まで簡素化された統一型構文モデルを提案する：**すべては `name: type = value`**。

YaoXiang には唯一の宣言形式しかない：

```
identifier : type = expression
```

ここで `type` は任意の型式であり、`expression` は任意の値式である。
**`fn` もなく、`struct` もなく、`trait` もなく、`impl` もなく、小文字の `type` キーワードもない（ただし `Type` はメタ型キーワードとして存在する）**。

> **核心設計**：`Type` 自体が一つの汎用型である。`(T: Type) -> Type` は「型パラメータ T を受け取る型」を表す。

| 概念       | コード書き方                                      |
|------------|-----------------------------------------------|
| 変数       | `x: Int = 42`                                |
| 関数       | `add: (a: Int, b: Int) -> Int = a + b`       |
| 記録型     | `Point: Type = { x: Float, y: Float }`       |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| 汎用型     | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| 汎用型     | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| メソッド   | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| 汎用関数   | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` は言語における唯一のメタ型キーワードである**。

> **名前空間 vs メソッドバインド**：`Type.name` 接頭辞は**名前空間の所属**を表すだけで、それ以外の意味はない。
> 暗黙的なバインドは一切トリガーされない。`p.draw(screen)` のような `.` 呼び出し構文を有効にするには、
> 明示的なバインドが必要である：`Point.draw = draw[0]`。
> 詳細は後述の「名前空間とメソッドバインド」セクションを参照。
ユーザーには透明に、コンパイラが内部で Type0、Type1、Type2... の区別を処理する。

```yaoxiang
// 核心構文：統一 + 区別

// 変数
x: Int = 42

// 関数（パラメータ名はシグネチャ内で）
add: (a: Int, b: Int) -> Int = a + b

// 記録型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// インターフェース（本質的に全フィールドが関数である記録型）
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

// 汎用型（(T: Type) -> Type = 型パラメータを受け取る汎用型）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int
}

Map: (K: Type, V: Type) -> Type = {
    keys: Array(K),
    values: Array(V)
}

// 使用
p: Point = Point(1.0, 2.0)
p.draw(screen)           // 構文糖 → Point.draw(p, screen)
s: Drawable = p           // 構造的サブタイピング：Point は Drawable を実装
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## 動機

### なぜこの機能が必要なのか？

現在の型システムには複数の分離した概念が存在する：
- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インターフェース定義構文
- メソッドバインド構文

これらの概念の間に統一性が欠如しており、構文の断片化と学習コストの増大を招いている。

### 設計目標

1. **極限の統一性**：一つの構文規則がすべてのケースをカバー
2. **簡潔でエレガント**：`name: type = value` の対称的な美学
3. **新しいキーワードが不要**：既存の構文要素を再利用
4. **理論的な優雅さ**：型自体も Type 型の値
5. **泛型に親和的**：泛型システム（RFC-011）とシームレスに統合

### 泛型システムとの統合

RFC-010の統一構文モデルはRFC-011の泛型システム設計と**自然に契合**し、泛型パラメータは統一モデルにシームレスに溶け込む：

```yaoxiang
// 基礎泛型（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// 泛型関数（RFC-023 構文：シグネチャの Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約は引数の型によって運ばれる

// Const 泛型（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：
- RFC-011 Phase 1（基礎泛型）はRFC-010の**強い依存先**
- 基礎泛型がなければ、RFC-010の泛型例はコンパイルできない
- 推奨：RFC-011 Phase 1 と RFC-010 は同期して実装する

## 提案

### 核心原則：型コンストラクタ vs 関数/変数

**これは重要な設計選択であり、構文の曖昧さ解消ルールを決定する：**

| 書き方 | 意味 | ルール |
|------|------|------|
| **`x: Type = ...`** | 型コンストラクタ | `: Type` を明示 → 型として強制 |
| **`f = ...`** | 関数または変数 | `: Type` なし → HM が能動的に関数/変数と推論 |

**なぜこのように設計するのか？**

`{ ... }` 構文自体に曖昧さがある：
- `{ x: Float, y: Float }` は**型リテラル**（記録型）になり得る
- `{ a = 1 + 1 }` は**コードブロック**（実行文、Void を返す）になり得る

**曖昧さを解消するルール**：
- **`: Type` がある** → 型コンストラクタとして強制解析、`{ ... }` は型リテラル
- **`: Type` がない** → HM が能動的に `{ ... }` をコードブロックとして解析、関数型と推論

```yaoxiang
# ✅ 型コンストラクタ：: Type がある
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type がない、HM が () -> Void と推論
main = { println("Hello") }

# ❌ エラー：: Type がないため、コンパイラは { ... } を型として解析できない
Point = { x: Float, y: Float }  // HM が関数と推論し、型ではない！
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
│   └── Point: Type = { x: Float, y: Float }  # 必ず返す：Type
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必ず返す：Type
│
├── 汎用型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず返す：Type
│
├── 汎用型（複数パラメータ）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず返す：Type
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示的にバインドして初めてドット呼び出し構文が使える
│
└── 汎用関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数と推論
```

### メタ型階層（コンパイラ内部）

**コンパイラ内部**は宇宙階層 `level: selfpointnum` を維持する（文字列で保存、理論上は無限に拡張可能）。

| Level | 説明 |
|-------|------|
| `Type0` | 日常的な型（`Int`、`Float`、`Point`） |
| `Type1` | 型コンストラクタ（`List`、`Maybe`） |
| `Type2+` | 高階コンストラクタ |

**ユーザーはこれらの数字を見ることはなく、`: Type` だけを見る。**

### Curry-Howard 同型：型は命题、プログラムは証明

YaoXiang の統一構文 `name: type = value` は任意に選ばれたものではない——それは Curry-Howard 同型（Curry-Howard correspondence）の直接的な写像である。この同型は深い事実を明らかにする：**型システムと論理システムは同じものの二つの面に過ぎない**。

| 論理（命題） | 型システム（YaoXiang） | 例 |
|---|---|---|
| 命題 P | 型 T | `Int`、`Bool` |
| P が真である証明 | 型 T の値 | `42: Int`、`true: Bool` |
| P → Q（含意） | 関数型 `(P) -> Q` | `(x: Int) -> Bool` |
| P ∧ Q（連言） | 記録型 `{ p: P, q: Q }` | `{ x: Int, y: Bool }` |
| ∀x.P(x）（全称量化） | 汎用関数 `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...` |
| P ⊕ Q（選言） | 列挙型 / tagged union | `Maybe: (T: Type) -> Type = { ... }` |

**`name: type = value` の Curry-Howard における意味**：

```yaoxiang
// "x: Int = 42" は "Int 型の証明が存在し、名前は x、値は 42" と読む
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" は
// "次の含意証明が存在する：Int の証明 a と b を与えれば、Int の証明を構成できる"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" は
// "Point は命題であり、その証明には Float 証明 x と Float 証明 y の両方が必要"
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要なのか？**

1. **論理的一貫性 = 型安全性**：もし型システムが型 `T` の値の構築を許すが、それに対応する正当な実行時表現がないなら、それは論理において偽の命題の証明を許すようなものである——システムが崩壊する。Curry-Howard は次のように教えている：**型安全な言語は、本質的に一貫した論理システムである**。

2. **宇宙階層は必要条件である**：後述のように、もし `Type: Type`（つまり「型の型も型である」）を許せば、Russell パラドックス（型論では Girard パラドックスとして現れる）が発生する。YaoXiang の `Type₀ : Type₁ : Type₂ : ...` の階層分けは、各型が特定の階層にのみ属し、決して閉ざれない上昇の連鎖を形成することを保証し、根本からパラドックスを回避する。これは YaoXiang の型システムが Curry-Howard 意味で**論理的に一貫している**ことを意味する。

3. **統一構文の理論的基盤**：`name: type = value` が変数、関数、型、インターフェース、泛型のすべてを一つの構文でカバーできる理由は、それらが Curry-Howard のもとで同じもの—**命題に証明を与えること**—だからである。変数は命題の証拠、関数は含意の証拠、記録は連言の証拠、泛型は全称量化の証拠である。統一構文は人為的に設計された偶然ではなく、Curry-Howard 同型の自然な帰結である。

> **参考文献**：Wadler, P. (2015). *"Propositions as Types."* Communications of the ACM, 58(12), 75–84. この記事は平易な言葉で Curry-Howard 同型の歴史と意義を解説している。

### 構文定義

#### 1. 変数宣言

```yaoxiang
// 基本構文
x: Int = 42
name: String = "Alice"
flag: Bool = true

// 型推論（省略可能）
y = 100  // Int と推論される
```

#### 2. 関数定義

```yaoxiang
// 単一式形式（直接値を返し、return 不要）
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

// Void 関数（コードブロック内に return 不要）
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### 戻り値の規則

戻り値は `=` の右辺の形式によって決まる：

| 書き方 | 戻り値 |
|------|--------|
| `= expr`（中括弧なし） | `expr` を直接返す |
| `= { ... }`（中括弧あり） | `return` を使う必要があり、なければ `Void` を返す |

```yaoxiang
# 単一式：直接値を返し、return 不要
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

> **設計理由**：`{ ... }` は依存駆動計算ユニット（後述）であり、その戻り値セマンティクスは単一式とは異なる。中括弧は複数文のコンテキストを導入するため、「最後の式が戻り値かどうか」の曖昧さを排除するために明示的な `return` が必要となる。

#### `{}` セマンティクス：依存駆動計算ユニット

`{ ... }` は YaoXiang では単なるコードブロックではなく、**依存駆動計算ユニット**である。このセマンティクスは関数本体、変数初期化、`spawn` の間で一貫している：

**核心ルール**：
- `{}` 内の代入文は記述順ではなく依存関係に従って自動的に並べられる
- 依存が揃えば即座に実行され、不足していればブロックして待機する
- `return` を使って明示的に値を返す（戻り値規則を参照）

```yaoxiang
# 依存駆動：b は a に依存し、コンパイラが自動的に a の後に並べる
result: Int = {
    b = a + 1      # a に依存 → 自動的に a の後に来る
    a = 10         # 依存なし → 先に実行可能
    return b       # 11 を返す
}
```

> **単一式との違い**：`= expr`（中括弧なし）は直接値を返す単純な束縛；`= { ... }`（中括弧あり）は依存駆動計算コンテキストを導入し、複数文と明示的な `return` を許可する。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一の並列プリミティブである。`{}` の依存駆動セマンティクスを利用して自動並列化を実現する：

- `spawn { ... }` 内の直接の子代入は自動的に並列タスクを作成する
- 依存が揃ったタスクは即座に並行実行される
- 呼び出し側はすべての子タスクの完了をブロックして待つ

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a と依存なし、並行実行）
    c = process(a, b)         # a, b に依存 → 両方の完了を待って実行
    return c
}
// spawn ブロック内のすべてのタスクが完了するまで、呼び出し側はこの地点でブロックする
```

> **詳細な定義**：`spawn` の完全なセマンティクス、タスク作成ルール、ブロッキングモデルについては `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型と raw ポインタの操作を定義するために使われる。`{}` の return セマンティクスを利用して型定義を一つ上のスコープに返す：

**核心ルール**：
- `unsafe {}` 内で型を定義し、raw ポインタを操作できる
- `return` を使って型定義を一つ上のスコープに返す
- 返された型は `unsafe {}` の外でも使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # raw ポインタ
    }
    return SqliteDb
}

# SqliteDb は unsafe ブロックの外でも使用可能
db = sqlite3_open("test.db")

# ❌ コンパイルエラー：handle フィールドには unsafe 権限が必要
handle = db.handle

# ✅ メソッド呼び出し経由
db.close()
```

> **詳細な定義**：`unsafe` の完全なセマンティクス、FFI 型定義、メソッドバインドについては `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang の統一構文の核心であり、フィールド、デフォルト値、バインドメソッド、インターフェース実装を含む：

##### 基本型

**記録型**：フィールドリスト、フィールドの型は任意の型式。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**：フィールドはデフォルト値を持つことができ、構築時にオプション。

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

**デフォルト値のないフィールド**：構築時に必ず提供しなければならない。

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

##### メソッドバインド

**方式1：型定義体内で直接外部関数をバインド**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置 0 にバインド、カリー化後 method: (b: Point) -> Float
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

**方式2：匿名関数 + 位置バインド**

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

**インターフェース名は型体内に書き、コンパイラが自動的にその実装をチェックする**

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

**インターフェース = 全フィールドが関数である記録型**

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

##### 名前空間関数定義

**`Type.name` 接頭辞は名前空間の所属**を表すだけで、それ以外の意味はない。暗黙的なバインドは一切トリガーされない。

```yaoxiang
// 名前空間関数：Point 名前空間下の通常の関数
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

// 呼び出し：通常の関数呼び出しそのもの
Point.draw(p, screen)
Point.serialize(p)
```

> **注意**：`self` はキーワードではなく、パラメータ名の慣習的な呼称に過ぎない。`p`、`this`、`x` のいずれで書いても効果は完全に同じ。
> コンパイラはパラメータ名を見ず、型を見る。

##### メソッドバインド（唯一の方法）

`p.draw(screen)` のような `.` メソッド呼び出し構文を有効にするには、**明示的なバインドが必要**。
`[position]` 構文は関数を「メソッド」としてバインドする唯一の仕組みである（詳細な構文は RFC-004 を参照）。

```yaoxiang
// 関数を定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示的なバインド — これ以降で初めて p.draw(screen) 構文が使える
Point.draw = draw[0]   // 位置 0 の引数（&Point）は呼び出し側が埋める

// 使用
p.draw(screen)          // 構文糖 → draw(&p, screen)
Point.draw(p, screen)   // 二つの呼び出し方は等価

// [0] を書かない = バインドしない。Point.draw は通常の関数エイリアスで、. 構文はない
Point.draw = draw       // バインドしない：Point.draw(p, screen) のみ可能
```

**デフォルト動作**：`[n]` を書かない = どの引数もバインドしない。ユーザーはどの引数を呼び出し側が埋めるかを明示的に決定しなければならない。

**複数位置バインド**：

```yaoxiang
// 複数位置をバインド（自動的にカリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドから通常の関数へ）：

```yaoxiang
// バインドから関数を取り出す
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. インターフェース合成

```yaoxiang
// インターフェース合成 = 型の交差
DrawableSerializable: Type = Drawable & Serializable

// 交差型の使用
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    return item.serialize()
}
```

#### 5. 汎用型

```yaoxiang
// 基礎泛型（RFC-011 Phase 1）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T:Type)-((self: List(T), item: T) -> Void),
    get: (T:Type)->((self: List(T), index: Int) -> Maybe(T))
}

// 具体インスタンス化（RFC-023 構文）
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

// 汎用メソッド（RFC-023 構文：型パラメータは呼び出し側で自動推論）
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

#### 6. 汎用呼び出し構文

汎用型と汎用関数の呼び出しは統一的に `()` 構文を使用する。`[]` はいかなる汎用コンテキストでも使用されない。

**核心ルール**：

1. **`()` ですべての適用を行う**：型適用、関数呼び出し、値構築はすべて `()` を使用

```yaoxiang
# 型注釈
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から来る
empty: List(Int) = List()

# 汎用関数呼び出し——型は引数から自動流動
strings = map(numbers, f)
// T=Int は numbers: List(Int) から来る
// R=String は f: (Int) -> String から来る
```

2. **Type は左、値は右**：`name: type = value`——Type パラメータは左側で宣言され、右側は常に具体的な値。空コンテナ `List()` の `T` は左側の型注釈から取得しなければならない。

3. **型情報は一度だけ書けばよい**——引数宣言時に、コンパイラがそれを運んで流れる：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左に一度書く
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String は numbers と f の型から自動
```

4. **値構築は要素から型を推論**：

```yaoxiang
x = List(1, 2, 3)       // List(Int) と推論
y = List("a", "b")      // List(String) と推論
z = List()              // ❌ コンパイルエラー：T を推論できない
z: List(Int) = List()   // ✅ T=Int は左側の注釈から取得
```

5. **型エイリアス**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` → `List(1,2,3)`。
> 旧い `[]` 汎用構文は完全に削除された。`[]` は配列/リストリテラルとインデックスアクセスのみに使用される。

### 例

#### 完全な例

```yaoxiang
// ======== 1. インターフェース定義 ========
// インターフェース = 全フィールドが関数型である記録型
// インターフェースには self パラメータは不要 — インターフェースは「呼び出し側の位置を除去した関数シグネチャ」のみを定義する

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // インターフェース型を返す、具体的な実装は自分の型を返す
    scale: (factor: Float) -> Transformable
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

// ======== 3. メソッド実装（通常の関数 + 明示的バインド）========

// 関数を定義（self は単なる慣習名であってキーワードではない）
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

bounding_box: (p: &Point) -> Rect = {
    return Rect(p.x - 1, p.y - 1, 2, 2)
}

serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

translate: (p: &Point, dx: Float, dy: Float) -> Point = {
    return Point(p.x + dx, p.y + dy)
}

scale: (p: &Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

distance: (p1: &Point, p2: &Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// 明示的バインド — バインドして初めてドット呼び出し構文が使える
Point.draw = draw[0]
Point.bounding_box = bounding_box[0]
Point.serialize = serialize[0]
Point.translate = translate[0]
Point.scale = scale[0]
Point.distance = distance[0]

// Rect のメソッドも同様
draw: (r: &Rect, surface: Surface) -> Void = {
    surface.draw_rect(r.x, r.y, r.width, r.height)
}
Rect.draw = draw[0]

bounding_box: (r: &Rect) -> Rect = r
Rect.bounding_box = bounding_box[0]

serialize: (r: &Rect) -> String = {
    return "Rect(${r.x}, ${r.y}, ${r.width}, ${r.height})"
}
Rect.serialize = serialize[0]

translate: (r: &Rect, dx: Float, dy: Float) -> Rect = {
    return Rect(r.x + dx, r.y + dy, r.width, r.height)
}
Rect.translate = translate[0]

scale: (r: &Rect, factor: Float) -> Rect = {
    return Rect(r.x * factor, r.y * factor, r.width * factor, r.height * factor)
}
Rect.scale = scale[0]

// ======== 4. 使用 ========

// インスタンスの作成
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// メソッド呼び出し（構文糖）
p.draw(screen)
r.draw(screen)

// 通常のメソッド呼び出し（直接呼び出し）
d: Float = distance(p, Point(0.0, 0.0))

// チェーン呼び出し
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// インターフェース代入
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// 汎用関数（RFC-023 構文：呼び出し時に型パラメータを省略、自動推論）
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
        // 型に同名のメソッドがあるかチェック
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換かチェック
            // インターフェースフィールド: (Surface) -> Void
            // メソッドシグネチャ: (Point, Surface) -> Void
            // 比較：self パラメータを除去した後、一致する必要がある
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

インターフェース型は直接代入をサポートし、コンパイラは代入の右辺の型に基づいて最適な呼び出し戦略を自動的に選択する：

```yaoxiang
// 具体型を直接代入 → コンパイル時に具体型を決定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：直接 circle_draw(screen) を呼び出す、vtable なし

// 関数の戻り値 → コンパイル時に具体型を決定できない、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable 経由でメソッドを検索

// 異種コレクション → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable 経由でメソッドを検索
}
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| `d: Drawable = Circle(1)` | 具体型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()` | 不明 | vtable |
| `shapes: List(Drawable) = [...]` | 異種 | vtable |

**ルール**：
1. 右辺が具体型コンストラクタでコンパイル時に決定可能な場合、直接呼び出し IR を生成する
2. 右辺の型がコンパイル時に決定できない場合、vtable 機構にフォールバックする
3. vtable はフォールバックとして実行時ポリモーフィズムの正確性を保証する

### ダックタイピングサポート

```yaoxiang
// 同じメソッドを持つだけで、インターフェース型に代入できる
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

### 構文の変化

| 以前 | 以後 |
|------|------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| `impl` キーワードが必要 | キーワード不要、インターフェース名は型体の後に書く |

## 構文設計説明：名前付き関数は本質的に Lambda の構文糖

### 核心的理解

**名前付き関数と Lambda 式は同じものである！** 唯一の違いは、名前付き関数が Lambda に名前をつけたことだけ。

```yaoxiang
// この二つは本質的に完全に同じ
add: (a: Int, b: Int) -> Int = a + b           // 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全に等価）
```

### 構文糖モデル

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**要点**：シグネチャが引数の型を完全に宣言している場合、Lambda ヘッダのパラメータ名は冗長になり、省略できる。

### パラメータスコープルール

**パラメータは外側の変数を覆い隠す**：シグネチャ内のパラメータスコープが関数本体を覆い隠し、内部スコープの優先度が高くなる。

```yaoxiang
x = 10  // 外側の変数

double: (x: Int) -> Int = x * 2  // ✅ パラメータ x が外側の x を覆い隠し、結果は 20
```

### 注釈位置は柔軟

型注釈は以下のいずれの位置でもよく、**少なくとも一箇所に注釈すればよい**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambda ヘッダのみ | `double = (x: Int) => x * 2` | ✅ 合法 |
| 両方に注釈 | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャが完全、Lambda ヘッダ省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda ヘッダに型を注釈
double = (x: Int) => x * 2

// ✅ 合法：両方に注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の利点

| 特性 | 利点 |
|------|------|
| **簡潔** | シグネチャが完全な場合、パラメータ名を書き直す必要がない |
| **柔軟** | Lambda 形式を残し、好きな方を使用可能 |
| **一貫性** | 変数宣言 `x: Int = 42` と同じパターンを維持 |
| **直感的** | `name: Type = body` が「名前は name、型は Type、値は body」に対応 |

## トレードオフ

### 利点

| 利点 | 説明 |
|------|------|
| 極限の統一性 | 一つの構文規則がすべてをカバー |
| 理論的な優雅さ | 完全に左右対称の `name: type = value` |
| 新キーワード不要 | 既存の構文要素を再利用 |
| 実装が容易 | コンパイラは一つの宣言形式だけを処理すればよい |
| 学習が容易 | 一つのパターンを覚えればすべてのコードが書ける |
| 拡張が容易 | 新機能をこのモデルに自然に組み込める |

### 欠点

| 欠点 | 説明 |
|------|------|
| 命名規則 | メソッドは `Type.method` 命名規則に従う必要がある |
| 冗長 | 完全構文は簡略構文より長いが、推論可能 |
| 学習曲線 | 統一モデルの理解が必要 |

### 緩和措置

```yaoxiang
// 1. 明確なエラーメッセージ
// コンパイルエラー例：
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. 型推論
// 型を省略でき、コンパイラが推論する
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE ヒント
// IDE が不足しているメソッドを自動的にヒント表示
```

### リスク

| リスク | 影響 | 緩和措置 |
|------|------|----------|
| 解析の複雑さ | 統一構文が解析の複雑さを増す可能性 | 再帰下降パーサを使用 |
| パフォーマンスオーバーヘッド | vtable 検索に追加のオーバーヘッドの可能性 | コンパイル時モノモーフィゼーション最適化 |

---

## イースターエッグ 🎮：言語の源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 型の型を定義してみる...
Type: Type = Type
```

**警告**：これは**名状しがたい**ものである！

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二、二生三、三生万物。                                   ║
║   易有太极、是生两仪。                                         ║
║                                                              ║
║   Type: Type = Type                                          ║
║   此乃爻象之源、言語之境界。                                   ║
║   编译器在此沈默、哲学在此駐足。                               ║
║                                                              ║
║   言語の哲学的境界に到達したことを感謝する。                    ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：コンパイラは `Type: Type = Type` を正しく処理できない（Type0/Type1 宇宙パラドックスを引き起こす）が、この「イースターエッグ」を意図的に残している——コンパイルを試みると、言語の創始者からの禅のメッセージを受け取る。これは技術的限界だけでなく、YaoXiang からの型哲学への敬意でもある。

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

# 汎用パラメータ：関数型の一部として、例如 (T: Type, R: Type) -> (...)
# 独立した BNF ルールは不要——: Type パラメータは通常の関数引数

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
| インターフェース | 全フィールドが関数型である記録型 |
| 汎用型 | `Name: (T: Type) -> Type = { ... }` として定義され、型パラメータを受け取る型 |
| 名前空間関数 | `Type.name` 形式の関数で、Type 名前空間に属する。暗黙的なバインドは含まない |
| メソッドバインド | `Type.name = func[n]`、`obj.name(args)` 構文を使えるように func の位置 n を呼び出し側にバインドする |
| 汎用関数 | `(T: Type)` 構文を使用する関数で、型パラメータが最初の引数群となる |
| メタ型 | `Type`、言語における唯一の型階層マーカー |

---

## ライフサイクルと帰属

```
┌─────────────┐
│   ドラフト   │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中 │  ← コミュニティの議論とフィードバックを公開
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
---
title: "RFC-010: 統一型構文 - name: type = value モデル"
status: "Accepted"
author: "晨煦"
created: "2025-01-20"
updated: "2026-06-05（戻り値規則と {} セマンティクスを更新）"
---

# RFC-010: 統一型構文 - name: type = value モデル


## 要約

本 RFC は極めて簡素で統一された型構文モデルを提案する：**すべては `name: type = value`**。

YaoXiang には一つの宣言形式しかない：

```
identifier : type = expression
```

ここで `type` は任意の型式であり、`expression` は任意の値式である。
**`fn` も `struct` も `trait` も `impl` も、小文字の `type` キーワードも存在しない**（ただし `Type` は元型キーワードとして存在する）。

> **核心設計**：`Type` 自体が一つの総称型（ジェネリクス）型である。`(T: Type) -> Type` は「型パラメータ T を受け取る型」を表す。

| 概念       | コードの書き方                                      |
|------------|-----------------------------------------------|
| 変数       | `x: Int = 42`                                |
| 関数       | `add: (a: Int, b: Int) -> Int = a + b`       |
| 記録型     | `Point: Type = { x: Float, y: Float }`       |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| 総称型     | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| 総称型     | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| メソッド   | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| 総称関数   | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` は言語で唯一の元型キーワード**である。

> **名前空間とメソッドバインドの違い**：`Type.name` という前置詞は**名前空間の所属**を示すに過ぎない。
> 暗黙のバインドは一切発生しない。`p.draw(screen)` のような `.` 呼び出し構文を有効にするには、
> 明示的なバインドが必要である：`Point.draw = draw[0]`。
> 詳細は後述の「名前空間とメソッドバインド」セクションを参照。
> これは型の階層を注釈するために使われ、コンパイラは Type0、Type1、Type2... の区別を自動的に処理し、ユーザーには透過的である。

```yaoxiang
// 核心構文：統一と区別

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

// インターフェース（本質的にフィールドがすべて関数である記録型）
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

// 総称型（(T: Type) -> Type = 型パラメータを受け取る総称型）
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
p.draw(screen)           // 構文糖衣 → Point.draw(p, screen)
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

これらの概念の間には統一性が欠如しており、構文の断片化と学習コストの増大を招いている。

### 設計目標

1. **極限の統一性**：一つの構文規則ですべてをカバー
2. **簡潔で優雅**：`name: type = value` の対称美
3. **新しいキーワードが不要**：既存の構文要素を再利用
4. **理論的に優雅**：型自体が Type 型の値である
5. **ジェネリクスに親和的**：ジェネリクスシステム（RFC-011）とシームレスに統合

### ジェネリクスシステムとの統合

RFC-010 の統一構文モデルは RFC-011 のジェネリクスシステム設計と**自然に契合**し、ジェネリクスパラメータは統一モデルにシームレスに溶け込む：

```yaoxiang
// 基本ジェネリクス（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// 総称関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約はパラメータ型が運ぶ

// Const ジェネリクス（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：
- RFC-011 Phase 1（基本ジェネリクス）は RFC-010 の**強い依存先**
- 基本ジェネリクスがなければ、RFC-010 のジェネリクスの例はコンパイルできない
- 推奨：RFC-011 Phase 1 と RFC-010 は同期的に実装する

## 提案

### 核心原則：型コンストラクタ vs 関数/変数

**これは構文の曖昧性解消ルールを決定する重要な設計選択である：**

| 書き方 | 意味 | ルール |
|------|------|------|
| **`x: Type = ...`** | 型コンストラクタ | `: Type` を明示 → 型として強制 |
| **`f = ...`** | 関数または変数 | `: Type` なし → HM が能動的に関数/変数と推論 |

**なぜこのように設計するのか？**

`{ ... }` 構文自体には曖昧性がある：
- `{ x: Float, y: Float }` は**型リテラル**（記録型）になり得る
- `{ a = 1 + 1 }` は**コードブロック**（実行文、void を返す）になり得る

**曖昧性解消のルール**：
- **`: Type` がある** → 型コンストラクタとして強制解釈、`{ ... }` は型リテラル
- **`: Type` がない** → HM が能動的に `{ ... }` をコードブロックとして解釈、関数型と推論

```yaoxiang
# ✅ 型コンストラクタ：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void と推論
main = { println("Hello") }

# ❌ エラー：: Type がないため、コンパイラは { ... } を型として解釈できない
Point = { x: Float, y: Float }  // HM は関数と推論し、型ではない！
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
│   └── Point: Type = { x: Float, y: Float }  # 必ず Type を返す
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必ず Type を返す
│
├── 総称型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず Type を返す
│
├── 総称型（複数パラメータ）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず Type を返す
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示的バインド後にのみドット呼び出し構文が有効
│
└── 総称関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数と推論
```

### 元型階層（コンパイラ内部）

**コンパイラ内部**は宇宙階層 `level: selfpointnum`（文字列で格納、理論上は無限に延長可能）を維持する。

| Level | 説明 |
|-------|------|
| `Type0` | 日常の型（`Int`、`Float`、`Point`） |
| `Type1` | 型コンストラクタ（`List`、`Maybe`） |
| `Type2+` | 高階コンストラクタ |

**ユーザーはこれらの数字を見ることはなく**、`: Type` だけを見る。

### Curry-Howard 同型：型は命題、プログラムは証明

YaoXiang の統一構文 `name: type = value` は恣意的に選ばれたものではない——それは Curry-Howard 同型（Curry-Howard correspondence）の直接的な写像である。この同型は深い事実を明らかにする：**型システムと論理システムは、同じものの二つの側面に過ぎない**。

| 論理（命題） | 型システム（YaoXiang） | 例 |
|---|---|---|
| 命題 P | 型 T | `Int`、`Bool` |
| P が真である証明 | 型 T の値 | `42: Int`、`true: Bool` |
| P → Q（含意） | 関数型 `(P) -> Q` | `(x: Int) -> Bool` |
| P ∧ Q（連言） | 記録型 `{ p: P, q: Q }` | `{ x: Int, y: Bool }` |
| ∀x.P(x)（全称量化） | 総称関数 `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...` |
| P ⊕ Q（選言） | 列挙型 / tagged union | `Maybe: (T: Type) -> Type = { ... }` |

**`name: type = value` の Curry-Howard における意味**：

```yaoxiang
// "x: Int = 42" は「Int 型の証明が存在し、名前が x、その値が 42」と読む
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" は
// 「含意の証明が存在する：Int の証明 a と b が与えられれば、Int の証明を構成できる」と読む
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" は
// 「Point は命題であり、その証明には Float の証明 x と Float の証明 y の同時提供が必要」と読む
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要なのか？**

1. **論理的一貫性 = 型安全性**：もし型システムが型 `T` の値を正当なランタイム表現なしに構築できるなら、論理において偽の命題を証明することを許すのと同じである——システムが崩壊する。Curry-Howard が教えてくれるのは：**型安全な言語は、本質的に一貫した論理システムである**ということ。

2. **宇宙階層は必要条件である**：後述のように、`Type: Type`（つまり「型の型も型である」）を許せば Russell パラドックス（型論では Girard パラドックスとして現れる）が発生する。YaoXiang の `Type₀ : Type₁ : Type₂ : ...` の階層は、各型が特定の階層にのみ属し、決して閉じない上昇の連鎖を形成することで、パラドックスを根本的に回避する。これは YaoXiang の型システムが Curry-Howard の意味で**論理的に一貫している**ことを意味する。

3. **統一構文の理論的基礎**：`name: type = value` が変数、関数、型、インターフェース、総称すべてを一つの構文でカバーできるのは、それらが Curry-Howard においてすべて同じこと——**命題に証明を提供すること**——だからである。変数は命題の証拠、関数は含意の証拠、記録は連言の証拠、総称は全称量化の証拠である。統一構文は人為的な設計の偶然ではなく、Curry-Howard 同型の自然な帰結である。

> **参考文献**：Wadler, P. (2015). *"Propositions as Types."* Communications of the ACM, 58(12), 75–84. この記事は Curry-Howard 同型の歴史と意義を平易な言葉で解説している。

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
// 単一式形式（直接値を返し、return は不要）
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

// void 関数（コードブロック内に return は不要）
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### 戻り値規則

戻り値は `=` の右辺の形式に依存する：

| 書き方 | 戻り値 |
|------|--------|
| `= expr`（波括弧なし） | `expr` を直接返す |
| `= { ... }`（波括弧あり） | `return` を使う必要がある、さもなければ void を返す |

```yaoxiang
# 単一式：直接値を返し、return は不要
add: (a: Int, b: Int) -> Int = a + b

# コードブロック：return で値を返す必要がある
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

# void 関数：return は不要
print: (msg: String) -> Void = {
    console.write(msg)
}
```

> **設計理由**：`{ ... }` は依存駆動計算ユニット（後述）であり、その戻り値セマンティクスは単一式と異なる。波括弧は複数文のコンテキストを導入するため、「最後の式が戻り値か否か」の曖昧性を解消するために明示的な `return` が必要となる。

#### `{}` セマンティクス：依存駆動計算ユニット

YaoXiang における `{ ... }` は単なるコードブロックではない——**依存駆動計算ユニット**である。このセマンティクスは関数本体、変数初期化、`spawn` において一貫している：

**核心ルール**：
- `{}` 内の代入文は書き順ではなく依存関係によって自動ソートされる
- 依存が揃えば即座に実行され、欠けていればブロックして待機する
- 明示的な `return` で値を返す（戻り値規則を参照）

```yaoxiang
# 依存駆動：b は a に依存し、コンパイラが自動ソート
result: Int = {
    b = a + 1      # a に依存 → a の後に自動配置
    a = 10         # 依存なし → 先に実行可能
    return b       # 11 を返す
}
```

> **単一式との違い**：`= expr`（波括弧なし）は直接値を返す単純なバインディング；`= { ... }`（波括弧あり）は依存駆動計算コンテキストを導入し、複数文と明示的な `return` を許容する。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一並列プリミティブである。`{}` の依存駆動セマンティクスを利用して自動並列化を実現する：

- `spawn { ... }` 内の直接の子代入は自動的に並列タスクを生成する
- 依存の揃ったタスクは即座に並行実行される
- 呼び出し元はすべての子タスクの完了をブロックして待機する

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a と依存なし、並列実行）
    c = process(a, b)         # a, b に依存 → 両方の完了を待って実行
    return c
}
// 呼び出し元は spawn ブロック内すべてのタスクが完了するまでここでブロックする
```

> **詳細な定義**：`spawn` の完全なセマンティクス、タスク生成ルール、ブロッキングモデルについては `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型（opaque type）の定義と生ポインタの操作に使用される。`{}` の return セマンティクスを利用して、型定義を上位スコープに返す：

**核心ルール**：
- `unsafe {}` 内で型を定義し、生ポインタを操作できる
- `return` を使って型定義を上位スコープに返す
- 返された型は `unsafe {}` の外でも使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 生ポインタ
    }
    return SqliteDb
}

# SqliteDb は unsafe ブロック外でも使用可能
db = sqlite3_open("test.db")

# ❌ コンパイルエラー：handle フィールドには unsafe 権限が必要
handle = db.handle

# ✅ メソッド呼び出し経由
db.close()
```

> **詳細な定義**：`unsafe` の完全なセマンティクス、FFI 型定義、メソッドバインドについては `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang の統一構文の中核であり、フィールド、デフォルト値、バインドされたメソッド、インターフェース実装を含む：

##### 基本型

**記録型**：フィールドリスト。フィールドの型は任意の型式であり得る。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**：フィールドはデフォルト値を持つことができ、構築時にはオプションとなる。

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

**デフォルト値のないフィールド**：構築時に必ず提供しなければならない。

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

##### メソッドバインド

**方法 1：型定義体内で外部関数を直接バインド**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置 0 にバインド、カリー化後 method: (b: Point) -> Float
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

**方法 2：無名関数 + 位置バインド**

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

**インターフェース名は型体内に書かれ、コンパイラが自動的にその実装をチェックする**

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

**インターフェース = フィールドがすべて関数である記録型**

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

**`Type.name` という前置詞は名前空間の所属を示すだけ**であり、暗黙のバインドは一切発生しない。

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

> **注意**：`self` はキーワードではなく、パラメータ名の慣習に過ぎない。`p`、`this`、`x` としても効果は変わらない。
> コンパイラはパラメータ名を見ず、型を見る。

##### メソッドバインド（唯一の方法）

`p.draw(screen)` のような `.` メソッド呼び出し構文を有効にするには、**明示的なバインドが必要**である。
`[position]` 構文は関数を「メソッド」としてバインドする唯一の仕組みである（詳細な構文は RFC-004 を参照）。

```yaoxiang
// 関数を定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示的バインド — この後に p.draw(screen) 構文が利用可能になる
Point.draw = draw[0]   // 位置 0 のパラメータ（&Point）は呼び出し元が埋める

// 使用
p.draw(screen)          // 構文糖衣 → draw(&p, screen)
Point.draw(p, screen)   // 2 つの呼び出し方は等価

// [0] を書かない = バインドしない。Point.draw は通常の関数別名に過ぎず、. 構文はない
Point.draw = draw       // バインドしない：Point.draw(p, screen) のみ可能
```

**デフォルト動作**：`[n]` を書かない = どのパラメータもバインドしない。ユーザーが明示的にどのパラメータを呼び出し元が埋めるかを決定しなければならない。

**複数位置バインド**：

```yaoxiang
// 複数位置をバインド（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドから通常関数へ）：

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

#### 5. 総称型

```yaoxiang
// 基本ジェネリクス（RFC-011 Phase 1）
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

// 総称メソッド（RFC-023 構文：型パラメータは呼び出し側で自動推論）
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

#### 6. 総称呼び出し構文

総称型と総称関数の呼び出しは統一的に `()` 構文を使用する。`[]` はいかなるジェネリクスのコンテキストでも使用されない。

**核心ルール**：

1. **`()` ですべての適用を行う**：型適用、関数呼び出し、値コンストラクタはすべて `()` を使用

```yaoxiang
# 型注釈
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から取得
empty: List(Int) = List()

# 総称関数呼び出し — 型はパラメータから自動流動
strings = map(numbers, f)
// T=Int は numbers: List(Int) から
// R=String は f: (Int) -> String から
```

2. **Type は左、値は右**：`name: type = value` —— Type パラメータは左で宣言され、右は常に具体的な値。空コンテナ `List()` の `T` は左側の型注釈から取得しなければならない。

3. **型情報は一箇所にだけ書けばよい**——パラメータ宣言時に、コンパイラがそれを運んで流れる：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左に一箇所だけ
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String が numbers と f の型から自動
```

4. **値コンストラクタは要素から型を推論**：

```yaoxiang
x = List(1, 2, 3)       // List(Int) と推論
y = List("a", "b")      // List(String) と推論
z = List()              // ❌ コンパイルエラー：T を推論できない
z: List(Int) = List()   // ✅ T=Int は左側の注釈から
```

5. **型エイリアス**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` → `List(1,2,3)`。
> 古い `[]` ジェネリクス構文は完全に削除された。`[]` は配列/リストリテラルとインデックスアクセスのみに使用される。

### 例

#### 完全な例

```yaoxiang
// ======== 1. インターフェース定義 ========
// インターフェース = フィールドがすべて関数型である記録型
// インターフェースには self パラメータは不要 — インターフェースは「呼び出し元位置を除いた関数シグネチャ」のみを定義する

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // インターフェース型を返し、具体的な実装は自身の型を返す
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

// 関数を定義（self は単なる慣習名であり、キーワードではない）
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

// 明示的バインド — バインド後にのみドット呼び出し構文が有効
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

// メソッド呼び出し（構文糖衣）
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

// 総称関数（RFC-023 構文：呼び出し時に型パラメータを省略、自動推論）
process_all: (items: List(T)) -> Void = {
    for item in items {
        print(item.serialize())
    }
}

process_all([p, r])
```

## 詳細設計

### インターフェース検査アルゴリズム

```rust
fn check_type_implements_interface(
    typ: &Type,
    iface: &Type
) -> Result<(), TypeError> {
    // インターフェースの各フィールド（関数のフィールド）について
    for (field_name, iface_field) in &iface.fields {
        // 型が同名のメソッドを持つかチェック
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換かチェック
            // インターフェースフィールド: (Surface) -> Void
            // メソッドシグネチャ: (Point, Surface) -> Void
            // 比較：self パラメータを除いた後がマッチする必要がある
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

インターフェース型は直接代入をサポートし、コンパイラは代入の右辺の型に応じて最適な呼び出し戦略を自動的に選択する：

```yaoxiang
// 具体的な型を直接代入 → コンパイル時に具体型を決定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：circle_draw(screen) を直接呼び出し、vtable なし

// 関数の戻り値 → コンパイル時に具体型を決定不可能、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable 経由でメソッドをLookup

// 異種コレクション → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable 経由でメソッドをLookup
}
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| `d: Drawable = Circle(1)` | 具体型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()` | 不明 | vtable |
| `shapes: List(Drawable) = [...]` | 異種 | vtable |

**ルール**：
1. 右辺が具体型のコンストラクタでありコンパイル時に決定可能な場合、直接呼び出しの IR を生成する
2. 右辺の型がコンパイル時に決定できない場合、vtable 機構にフォールバックする
3. vtable がフォールバックとしてランタイム多態性の正確性を保証する

### ダックタイピングのサポート

```yaoxiang
// 同じメソッドを持っていれば、インターフェース型に代入できる
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

### 構文変化

| 以前 | 以後 |
|------|------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| `impl` キーワードが必要 | キーワード不要、インターフェース名は型体内に書く |

## 構文設計の解説：名前付き関数は本質的に Lambda の構文糖衣

### 核心的理解

**名前付き関数と Lambda 式は同じものである！** 唯一の違いは、名前付き関数が Lambda に名前を付けたことだけである。

```yaoxiang
// この 2 つは本質的に完全に同一
add: (a: Int, b: Int) -> Int = a + b           // 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全に等価）
```

### 構文糖衣モデル

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**重要な点**：シグネチャがパラメータ型を完全に宣言している場合、Lambda 头部のパラメータ名は冗長となり、省略可能である。

### パラメータスコープ規則

**パラメータは外側の変数を覆い隠す**：シグネチャ内のパラメータスコープは関数本体を覆い、内部スコープの優先度がより高い。

```yaoxiang
x = 10  // 外側の変数

double: (x: Int) -> Int = x * 2  // ✅ パラメータ x が外側の x を覆い隠し、結果は 20
```

### 注釈位置は柔軟

型注釈は以下のいずれかの位置に置くことができ、**少なくとも一箇所に注釈すればよい**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambda 头部のみ | `double = (x: Int) => x * 2` | ✅ 合法 |
| 両側に注釈 | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許容 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャが完全、Lambda 头部は省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda 头で型を注釈
double = (x: Int) => x * 2

// ✅ 合法：両側に注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の利点

| 特性 | 利点 |
|------|------|
| **簡潔** | シグネチャが完全な場合、パラメータ名を重複して書く必要がない |
| **柔軟** | Lambda 形式を保持、好みで選択可能 |
| **一貫** | 変数宣言 `x: Int = 42` との統一パターンを維持 |
| **直感的** | `name: Type = body` が「名前 name、型 Type、値 body」に直接対応 |

## トレードオフ

### 利点

| 利点 | 説明 |
|------|------|
| 極限の統一性 | 一つの構文規則ですべてをカバー |
| 理論的に優雅 | 完全に対称的な `name: type = value` |
| 新しいキーワードが不要 | 既存の構文要素を再利用 |
| 実装が容易 | コンパイラは一つの宣言形式を処理するだけでよい |
| 学習が容易 | 一つのパターンを覚えればすべてのコードが書ける |
| 拡張が容易 | 新機能がこのモデルに自然に統合できる |

### 欠点

| 欠点 | 説明 |
|------|------|
| 命名規則 | メソッドは `Type.method` 命名に従う必要がある |
| 冗長 | 完全構文は簡略構文より長くなるが、推論可能 |
| 学習曲線 | 統一モデルの理解が必要 |

### 緩和策

```yaoxiang
// 1. 明確なエラーメッセージ
// コンパイルエラーの例：
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. 型推論
// 型を省略でき、コンパイラが推論する
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE ヒント
// IDE が不足しているメソッドを自動ヒント
```

### リスク

| リスク | 影響 | 緩和策 |
|------|------|----------|
| 解析の複雑さ | 統一構文が解析の複雑さを増す可能性 | 再帰降下解析器を使用 |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドの可能性 | コンパイル時モノモーフィゼーション最適化 |

---

## 隠し要素 🎮：言語の起源

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
║   これが爻象之源、言語の境界である。                             ║
║   コンパイラはここで沈黙し、哲学はここで足をとめる。              ║
║                                                              ║
║   言語の哲学的境界に到達してくれたことに感謝する。                 ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：コンパイラは `Type: Type = Type` を正しく処理できない（Type0/Type1 宇宙のパラドックスを引き起こす）が、この「隠し要素」を意図的に残している——コンパイルを試みると、言語の創始者からの禅的なメッセージを受け取る。これは技術的境界だけでなく、YaoXiang の型哲学への敬意である。

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
       | 'Type'                                    # 元型

type_field ::= identifier ':' type_expr
             | identifier                           # インターフェース制約

# ジェネリクスパラメータ：関数型の一部として、(T: Type, R: Type) -> (...)
# 独立した BNF ルールは不要 — : Type パラメータは通常の関数パラメータ

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
| インターフェース | フィールドがすべて関数型である記録型 |
| 総称型 | `Name: (T: Type) -> Type = { ... }` と定義される型で、型パラメータを受け取る |
| 名前空間関数 | `Type.name` 形式の関数で、Type 名前空間に属する。暗黙のバインドを含意しない |
| メソッドバインド | `Type.name = func[n]`、func の位置 n を呼び出し元としてバインドし、`obj.name(args)` 構文を利用可能にする |
| 総称関数 | `(T: Type)` 構文を使用する関数で、型パラメータが最初のパラメータ群となる |
| 元型 | `Type`、言語で唯一の型階層マーカー |

---

## ライフサイクルと帰属

```
┌─────────────┐
│   ドラフト    │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中   │  ← コミュニティでの議論とフィードバックを公開
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   承認済み   │    │   却下      │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (元の場所)  │
└─────────────┘    └─────────────┘
```
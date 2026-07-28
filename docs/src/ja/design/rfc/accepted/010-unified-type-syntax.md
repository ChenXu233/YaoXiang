---
title: 'RFC-010: 統一型構文 - name: type = value モデル'
status: '承認済み'
author: '晨煦'
updated: '2026-07-14（Never 組み込み型が実装済み、#157 がクローズ済み）'
issue: '#127'
---

# RFC-010: 統一型構文 - name: type = value モデル

## 概要

本 RFC は極限まで簡素化された統一型構文モデルを提案する：**すべては `name: type = value`**。

YaoXiang にはただ一つの宣言形式が存在する：

```
identifier : type = expression
```

ここで `type` は任意の型式、`expression` は任意の値式である。 **`fn` も `struct` も `trait` も
`impl` も、小文字の `type` キーワードも存在しない（ただし `Type`
はメタ型キーワードとして存在する）**。

> **核心設計**：`Type` 自体が一つのジェネリック型である。`(T: Type) -> Type`
> は「型引数 T を受け取る型」を表す。

| 概念             | コード記述                                                                   |
| ---------------- | ---------------------------------------------------------------------------- |
| 変数             | `x: Int = 42`                                                                |
| 関数             | `add: (a: Int, b: Int) -> Int = a + b`                                       |
| レコード型       | `Point: Type = { x: Float, y: Float }`                                       |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }`                               |
| ジェネリック型   | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                  |
| ジェネリック型   | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`     |
| メソッド         | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| ジェネリック関数 | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`    |

**`Type` は言語における唯一のメタ型キーワードである**。

> **名前空間 vs メソッドバインディング**：`Type.name`
> という前置詞は**名前空間への帰属**を示すだけで、それ以外の意味はない。暗黙のバインディングは一切発生しない。`p.draw(screen)`
> のような `.`
> 呼び出し構文を有効化するには、明示的なバインディングが必要である：`Point.draw = draw[0]`。詳細は後述の「名前空間とメソッドバインディング」セクションを参照。これは型階層の注記に用いられ、コンパイラが Type0、Type1、Type2... の区別を自動処理し、ユーザには透過的である。

```yaoxiang
// 核心構文：統一と区別

// 変数
x: Int = 42

// 関数（引数名はシグネチャ内に記述）
add: (a: Int, b: Int) -> Int = a + b

// レコード型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// インターフェース（本質的に全フィールドが関数であるレコード型）
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

// ジェネリック型（(T: Type) -> Type = 型引数を受け取るジェネリック型）
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

### なぜこの機能が必要か？

現在の型システムには複数の分離された概念が存在する：

- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インターフェース定義構文
- メソッドバインディング構文

これらの概念間には統一性が欠如しており、構文の断片化と学習コストの増大を招いている。

### 設計目標

1. **極限の統一性**：一つの構文規則がすべてのケースをカバー
2. **簡潔でエレガント**：`name: type = value` の対称的美学
3. **新キーワード不要**：既存の構文要素を再利用
4. **理論的優雅さ**：型自体も `Type` 型の値である
5. **ジェネリクスとの親和性**：ジェネリクスシステム（RFC-011）とシームレスに統合

### ジェネリクスシステムとの統合

RFC-010 の統一構文モデルは RFC-011 のジェネリクスシステム設計と**自然に契合**し、ジェネリック引数は統一モデルにシームレスに溶け込む：

```yaoxiang
// 基本ジェネリクス（RFC-011 フェーズ 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// ジェネリック関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 フェーズ 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約は引数の型によって運ばれる

// Const ジェネリクス（RFC-011 フェーズ 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：

- RFC-011 フェーズ 1（基本ジェネリクス）は RFC-010 の**強依存**である
- 基本ジェネリクスがなければ、RFC-010 のジェネリクスの例はコンパイルできない
- 推奨：RFC-011 フェーズ 1 を RFC-010 と同期して実装する

## 提案

### 核心原則：型コンストラクタ vs 関数/変数

**これは重要な設計選択であり、構文の曖昧性解消ルールを決定する：**

| 記述方法            | 意味             | ルール                                       |
| ------------------- | ---------------- | -------------------------------------------- |
| **`x: Type = ...`** | 型コンストラクタ | `: Type` を明示宣言 → 型として強制           |
| **`f = ...`**       | 関数または変数   | `: Type` なし → HM が能動的に関数/変数と推論 |

**なぜこのように設計するのか？**

`{ ... }` 構文自体に曖昧性がある：

- `{ x: Float, y: Float }` は**型リテラル**（レコード型）となり得る
- `{ a = 1 + 1 }` は**コードブロック**（実行文、Void を返す）となり得る

**曖昧性解消のルール**：

- **`: Type` あり** → 型コンストラクタとして強制解釈、`{ ... }` は型リテラル
- **`: Type` なし** → HM が能動的に `{ ... }` をコードブロックとして解釈、関数型と推論

```yaoxiang
# ✅ 型コンストラクタ：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void と推論
main = { println("Hello") }

# ❌ エラー：: Type なしではコンパイラが { ... } を型として解釈できない
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
├── レコード型
│   └── Point: Type = { x: Float, y: Float }  # 戻り型が Type である必要あり
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 戻り型が Type である必要あり
│
├── ジェネリック型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 戻り型が Type である必要あり
│
├── ジェネリック型（複数引数）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 戻り型が Type である必要あり
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示バインディング後にのみドット呼び出し構文が有効
│
└── ジェネリック関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数と推論
```

### メタ型階層（コンパイラ内部）

**コンパイラ内部**は宇宙階層 `level: selfpointnum`
を維持する（文字列で格納し、理論上は無限に拡張可能）。

| Level    | 説明                                |
| -------- | ----------------------------------- |
| `Type0`  | 通常の型（`Int`、`Float`、`Point`） |
| `Type1`  | 型コンストラクタ（`List`、`Maybe`） |
| `Type2+` | 高階コンストラクタ                  |

**ユーザはこれらの数字を見ることはなく、`: Type` のみを見る。**

### Curry-Howard 同型：型は命题、プログラムは証明

YaoXiang の統一構文 `name: type = value`
は恣意的に選ばれたものではない——それはまさに Curry-Howard 同型（Curry-Howard
correspondence）の直接的な写像である。この同型は深い事実を明らかにする：**型システムと論理システムは同じものの二つの側面である**。

| 論理（命題）        | 型システム（YaoXiang）              | 例                                   |
| ------------------- | ----------------------------------- | ------------------------------------ |
| 命題 P              | 型 T                                | `Int`、`Bool`                        |
| P が真である証明    | 型 T の一つの値                     | `42: Int`、`true: Bool`              |
| P → Q（含意）       | 関数型 `(P) -> Q`                   | `(x: Int) -> Bool`                   |
| P ∧ Q（連言）       | レコード型 `{ p: P, q: Q }`         | `{ x: Int, y: Bool }`                |
| ∀x.P(x)（全称量化） | ジェネリック関数 `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q（選言）       | 列挙型 / タグ付きユニオン           | `Maybe: (T: Type) -> Type = { ... }` |

**Curry-Howard 下での `name: type = value` の意味**：

```yaoxiang
// "x: Int = 42" を読む："Int 型の証明が存在し、名前が x、その値が 42 である"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" を読む：
// "含意証明が存在する：Int の証明 a と b を与えられれば、Int の証明を構成できる"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" を読む：
// "Point は命題であり、その証明には Float の証明 x と Float の証明 y の同時提供が必要である"
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要か？**

1. **論理的一貫性 = 型安全性**：もし型システムが型 `T`
   の値の構築を許可するものの、正当な実行時表現が一切存在しないなら、それは論理において偽命題の証明を許可するのに等しい——システムは崩壊する。Curry-Howard が教えてくれる：**型安全な言語は本質的に一貫した論理システムである**。

2. **宇宙階層は必要条件である**：後述するように、もし
   `Type: Type`（すなわち「型の型もまた型である」）を許せば、Russell パラドックス（型論では Girard パラドックスとして現れる）を引き起こす。YaoXiang の
   `Type₀ : Type₁ : Type₂ : ...`
   という階層化により、各型は特定の階層のみに属し、決して閉合しない上昇連鎖を形成し、根本的にパラドックスを回避する。これは YaoXiang の型システムが Curry-Howard 意味で**論理的に一貫している**ことを意味する。

3. **統一構文の理論的基礎**：`name: type = value`
   が一つの構文で変数、関数、型、インターフェース、ジェネリクスのすべてをカバーできるのは、それらが Curry-Howard 下で同じ事——**命題に証明を提供すること**——だからである。変数は命題の証拠、関数は含意の証拠、レコードは連言の証拠、ジェネリクスは全称量化の証拠である。統一構文は人為的に設計された偶然ではなく、Curry-Howard 同型の自然な帰結である。

> **参考文献**：Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM, 58(12),
> 75–84. この論文は Curry-Howard 同型の歴史と意味を平易な言葉で解説している。

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
// 単一式形式（値を直接返し、return 不要）
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// コードブロック形式（return を使って値を返す必要がある）
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

#### 返却ルール

返却値は `=` の右側の形式に依存する：

| 記述方法                  | 返却値                                  |
| ------------------------- | --------------------------------------- |
| `= expr`（中括弧なし）    | `expr` を直接返す                       |
| `= { ... }`（中括弧あり） | `return` が必要、なければ `Void` を返す |

```yaoxiang
# 単一式：値を直接返し、return 不要
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

> **設計理由**：`{ ... }`
> は依存駆動計算ユニット（後述）であり、その返却セマンティクスは単一式とは異なる。中括弧は複数文のコンテキストを導入するため、「最後の式が返り値かどうか」の曖昧性を排除するために明示的な
> `return` が必要となる。

#### `{}` セマンティクス：依存駆動計算ユニット

YaoXiang における `{ ... }`
は単なるコードブロックではない——それは**依存駆動計算ユニット**である。このセマンティクスは関数本体、変数初期化、`spawn`
の間で一貫している：

**核心ルール**：

- `{}` 内の代入文は記述順序ではなく依存関係に従って自動ソートされる
- 依存が満たされれば即座に実行され、不足していればブロッキング待機する
- `return` を使って明示的に値を返す（返却ルール参照）

```yaoxiang
# 依存駆動：b は a に依存し、コンパイラが自動ソート
result: Int = {
    b = a + 1      # a に依存 → a の後に自動配置
    a = 10         # 依存なし → 先に実行可能
    return b       # 11 を返す
}
```

> **単一式との違い**：`= expr`（中括弧なし）は値を直接返す単純なバインディング；`= { ... }`（中括弧あり）は依存駆動計算コンテキストを導入し、複数文と明示的
> `return` を許可する。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang 唯一の並列プリミティブである。`{}`
の依存駆動セマンティクスを活用して自動並列化を実現する：

- `spawn { ... }` 内の直接の代入文は自動的に並列タスクを生成する
- 依存が満たされたタスクは即座に並列実行される
- 呼び出し側はすべてのサブタスクの完了をブロッキング待機する

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a と無依存、並列実行）
    c = process(a, b)         # a, b に依存 → 両方の完了を待って実行
    return c
}
// spawn ブロック内の全タスクが完了するまで呼び出し側をここでブロック
```

> **詳細定義**：`spawn` の完全なセマンティクス、タスク生成ルール、ブロッキングモデルについては
> `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型の定義と生ポインタの操作に使用される。`{}`
の return セマンティクスを活用して型定義を上位スコープに返す：

**核心ルール**：

- `unsafe {}` 内で型を定義し生ポインタを操作できる
- `return` を使って型定義を上位スコープに返す
- 返された型は `unsafe {}` 外で使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 生ポインタ
    }
    return SqliteDb
}

# SqliteDb は unsafe ブロック外で使用可能
db = sqlite3_open("test.db")

# ❌ コンパイルエラー：handle フィールドには unsafe 権限が必要
handle = db.handle

# ✅ メソッド呼び出し経由
db.close()
```

> **詳細定義**：`unsafe` の完全なセマンティクス、FFI 型定義、メソッドバインディングについては
> `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang 統一構文の核心であり、フィールド、デフォルト値、バインドされたメソッド、インターフェース実装を含む：

##### 基本型

**レコード型**：フィールドリスト、フィールドの型は任意の型式を取り得る。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**：フィールドはデフォルト値を持て、構築時にオプションとなる。

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

使用：

```yaoxiang
Point() → Point(x=0, y=0)
Point(x=1) → Point(x=1, y=0)
Point(x=1, y=2) → Point(x=1, y=2)
```

**デフォルト値を持たないフィールド**：構築時に必ず提供しなければならない。

```yaoxiang
Point2: Type = {
    x: Float,
    y: Float
}
```

使用：

```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### 組み込み型

YaoXiang の識別子体系は三層に分かれ、異なるコンパイラ段階で順次認識される：

1. **キーワード**（parser 独立トークン）— 制御構造と宣言キーワード、例えば
   `if`、`match`、`pub`、`return`
2. **リテラル予約語**（parser 独立トークン）—
   `true`、`false`、`void`、`Type`、通常の識別子にはなれない
3. **組み込み型名**（type
   checker 事前登録）— パーサは通常の識別子として扱い、型チェッカが解析を担当。**予約語ではなく、シャドウ可能（非推奨）**

`void`（小文字、リテラル予約語）と `Void`（大文字、組み込み型名）の違い：`void`
は値リテラル（Unit の唯一の値と等しい）であり、`Void`
は型名（Unit 型と等しい、論理 ⊤）である。`let x: Void = void` は合法。

事前定義された組み込み型名：

| 型       | 論理的対応   | 説明                                                                                                                                                                                                                                                |
| -------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、この型に居留できる値はない。「不可能」を表す——発散、panic、デッドコード。`Never <: T` は任意の `T` に対して成立する（爆発原理）。`Never` を返す関数は正常終了しないことを意味する。**キーワードではなく、組み込み型名である。** |
| `Void`   | ⊤（真/Unit） | 正確に一つの居住者を持つ（デフォルトの void 値）。`x: Void = <デフォルト>` は合法。和型の単位元は積型の単位元に対応する——`Void` はゼロフィールド積型（Unit）であり、`Never` はゼロバリアント和型である。                                            |
| `Int`    | —            | 符号付き整数                                                                                                                                                                                                                                        |
| `Float`  | —            | 浮動小数点数                                                                                                                                                                                                                                        |
| `Bool`   | —            | ブール値：`true` / `false`                                                                                                                                                                                                                          |
| `Char`   | —            | Unicode 文字                                                                                                                                                                                                                                        |
| `String` | —            | 文字列                                                                                                                                                                                                                                              |

##### バインドされたメソッド

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

**方法 2：無名関数 + 位置バインディング**

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

**インターフェース名は型体内に記述し、コンパイラが自動的に実装をチェックする**

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

**インターフェース = 全フィールドが関数であるレコード型**

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

**`Type.name`
前置詞は名前空間への帰属**を示すだけで、それ以外の意味はない。暗黙のバインディングは一切発生しない。

```yaoxiang
// 名前空間関数：Point 名前空間下の通常関数
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

> **注意**：`self` はキーワードではなく、引数名の慣習的命名に過ぎない。`p`、`this`、`x`
> と書いても完全に同じ効果。コンパイラは引数名を見ず、型を見る。

##### メソッドバインディング（唯一の方法）

`p.draw(screen)` のような `.`
メソッド呼び出し構文を有効化するには、**明示的なバインディングが必要**。 `[position]`
構文は関数を「メソッド」としてバインドする唯一の機構である（詳細構文は RFC-004 参照）。

```yaoxiang
// 関数を定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示バインディング — これ以降 p.draw(screen) 構文が有効
Point.draw = draw[0]   // 位置 0 の引数（&Point）は呼び出し側が埋める

// 使用
p.draw(screen)          // 構文糖衣 → draw(&p, screen)
Point.draw(p, screen)   // 二つの呼び出し方は等価

// [0] を書かない = バインディングしない。Point.draw は通常の関数別名であり、. 構文はない
Point.draw = draw       // バインディングしない：Point.draw(p, screen) のみ
```

**デフォルト動作**：`[n]`
を書かない = どの引数もバインドしない。ユーザは呼び出し側が埋める引数を明示的に決定しなければならない。

**複数位置バインディング**：

```yaoxiang
// 複数位置をバインド（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドから通常関数へ）：

```yaoxiang
// バインディングから関数を取り出す
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

#### 5. ジェネリック型

```yaoxiang
// 基本ジェネリクス（RFC-011 フェーズ 1）
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

// ジェネリックメソッド（RFC-023 構文：型引数は呼び出し側で自動推論）
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

ジェネリック型とジェネリック関数の呼び出しは統一して `()` 構文を使用する。`[]`
はジェネリックコンテキストでは一切使用されない。

**核心ルール**：

1. **`()` ですべての適用を行う**：型適用、関数呼び出し、値コンストラクタはすべて `()` を使用

```yaoxiang
# 型注釈
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から来る
empty: List(Int) = List()

# ジェネリック関数呼び出し——型は引数から自動流動
strings = map(numbers, f)
// T=Int は numbers: List(Int) から来る
// R=String は f: (Int) -> String から来る
```

2. **Type は左、値は右**：`name: type = value`——Type 引数は左側で宣言され、右側は常に具体値。空コンテナ
   `List()` の `T` は左側の型注釈から取得しなければならない。

3. **型情報は一度だけ書けばよい**——引数宣言時に、コンパイラがそれを運ぶ：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左に一度だけ書く
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String は numbers と f の型から自動
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

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` →
> `List(1,2,3)`。旧 `[]` ジェネリクス構文は完全に削除された。`[]`
> は配列/リストリテラルとインデックスアクセスのみに使用される。

### 例

#### 完全な例

```yaoxiang
// ======== 1. インターフェース定義 ========
// インターフェース = 全フィールドが関数型であるレコード型
// インターフェースには self 引数は不要 — インターフェースは「呼び出し側位置を除去した関数シグネチャ」のみを定義する

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // インターフェース型を返す、具体的な実装は自身の型を返す
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

// ======== 3. メソッド実装（通常関数 + 明示バインディング）========

// 関数を定義（self は慣習的な名前に過ぎず、キーワードではない）
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

// 明示バインディング — バインディング後にのみドット呼び出し構文が有効
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

// インスタンス作成
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

// ジェネリック関数（RFC-023 構文：呼び出し時に型引数を省略、自動推論）
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
            // 比較：self 引数を除去後にマッチする必要がある
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

### インターフェースの直接代入とコンパイル時最適化

インターフェース型は直接代入をサポートし、コンパイラが代入の右辺の型に応じて最適な呼び出し戦略を自動選択する：

```yaoxiang
// 具体型を直接代入 → コンパイル時に具体型を決定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：circle_draw(screen) を直接呼び出し、vtable なし

// 関数戻り値 → コンパイル時に具体型を決定不可、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable 経由でメソッド検索

// 異種コレクション → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable 経由でメソッド検索
}
```

**コンパイル時最適化戦略**：

| シナリオ                         | 推論結果      | 呼び出し方式                       |
| -------------------------------- | ------------- | ---------------------------------- |
| `d: Drawable = Circle(1)`        | 具体型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()`      | 不明          | vtable                             |
| `shapes: List(Drawable) = [...]` | 異種          | vtable                             |

**ルール**：

1. 右辺が具体型コンストラクタでコンパイル時に決定可能な場合、直接呼び出し IR を生成
2. 右辺の型がコンパイル時に決定できない場合、vtable 機構にフォールバック
3. vtable は実行時多態の正確性を保証する最終防壁

### ダック・タイピングサポート

```yaoxiang
// 同じメソッドさえ持っていれば、インターフェース型に代入可能
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

| 以前                                     | 以後                                                                                         |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)`    | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| `impl` キーワードが必要                  | キーワード不要、インターフェース名は型体内に記述                                             |

### 廃止：`|` バリアント構文

> **廃止宣言（2026-07-25、issue #203）**：`|` バリアント構文は正式に廃止され、実装から削除された。

以下の記述は**もはやサポートされない**：

```
type Color = red | green | blue                # ❌ 廃止
type Result(T, E) = ok(T) | err(E)             # ❌ 廃止
type Option(T) = some(T) | none                # ❌ 廃止
```

ユニオン型（sum
type）の表現にはレコード型を統一して使用する。レコード型のフィールドがすべて関数であり、かつすべてが当該型自身を返す場合、それは和型となる：

```yaoxiang
Color: Type = {
    red: () -> Color,
    green: () -> Color,
    blue: () -> Color
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E)
}

Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T)
}
```

**設計理由**：

1. **特殊ケースの排除**：`|` は BNF における唯一の `name: type = value`
   形式でない構文である。削除後、`type_expr`
   生成規則は完全に統一され、parser はバリアント型のために独立したパスと先読みバックトラックを維持する必要がなくなる。
2. **数学的等価性**：Curry-Howard 同型の下で、選言 P ⊕
   Q に対応する和型は、「全フィールドが自身を返す関数である」レコード型と等価である。両者は同じセマンティクスを表現し、二つの構文は不要である。
3. **破壊性ゼロ**：削除前の `|`
   構文は parser で半サポートされていた（引数なしバリアントは解析可能だが、引数型は単相化時に失われる）、ユーザコードの依存は一切ない。
4. **AST 簡素化**：`Type::Variant(Vec<VariantDef>)` ノードを削除、すべてのバリアント型は
   `Type::Struct` パスに統一、下流の typecheck/mono/formatter の特殊分岐がすべて排除される。

> **注**：和型のセマンティック特性（match 網羅性チェック、タグ付きユニオンのメモリレイアウト）は typecheck 層で
> `Type::Struct` 構造から導出され、独立した AST ノードには依存しない。

## 構文設計説明：名前付き関数は本質的に Lambda の構文糖衣

### 核心的理解

**名前付き関数と Lambda 式は同じものである！**
唯一の違いは：名前付き関数は Lambda に名前をつけただけである。

```yaoxiang
// この二つは本質的に完全に同じ
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

**要点**：シグネチャが引数型を完全に宣言している場合、Lambda ヘッダの引数名は冗長となり、省略可能。

### 引数スコープルール

**引数は外側の変数を覆す（シャドウイング）**：シグネチャ内の引数スコープは関数本体を覆い、内部スコープの優先度がより高い。

```yaoxiang
x = 10  // 外側の変数

double: (x: Int) -> Int = x * 2  // ✅ 引数 x が外側の x を覆す、結果は 20
```

### 注釈位置の柔軟性

型注釈は以下の任意の場所に配置可能で、**少なくとも一箇所に注釈すればよい**：

| 注釈位置          | 形式                                     | 説明            |
| ----------------- | ---------------------------------------- | --------------- |
| シグネチャのみ    | `double: (x: Int) -> Int = x * 2`        | ✅ 推奨         |
| Lambda ヘッダのみ | `double = (x: Int) => x * 2`             | ✅ 合法         |
| 両側に注釈        | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャ完全、Lambda ヘッダ省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda ヘッダに型を注釈
double = (x: Int) => x * 2

// ✅ 合法：両側に注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の利点

| 特性       | 利点                                                                |
| ---------- | ------------------------------------------------------------------- |
| **簡潔**   | シグネチャが完全な場合、引数名を重複して書く必要なし                |
| **柔軟**   | Lambda 形式を残し、好みで選べる                                     |
| **一貫**   | 変数宣言 `x: Int = 42` と統一パターンを維持                         |
| **直感的** | `name: Type = body` は直接的に「名前 name、型 Type、値 body」に対応 |

## トレードオフ

### 利点

| 利点             | 説明                                           |
| ---------------- | ---------------------------------------------- |
| 極限の統一性     | 一つの構文規則がすべてのケースをカバー         |
| 理論的優雅さ     | 完全に対称的な `name: type = value`            |
| 新キーワード不要 | 既存の構文要素を再利用                         |
| 実装容易性       | コンパイラはただ一つの宣言形式を処理すればよい |
| 学習容易性       | 一つのパターンを覚えればすべてのコードが書ける |
| 拡張容易性       | 新機能は自然にこのモデルに溶け込む             |

### 欠点

| 欠点     | 説明                                          |
| -------- | --------------------------------------------- |
| 命名規約 | メソッドは `Type.method` 命名に従う必要がある |
| 冗長性   | 完全構文は簡略構文より長いが、推論可能        |
| 学習曲線 | 統一モデルの理解が必要                        |

### 緩和策

```yaoxiang
// 1. 明確なエラーメッセージ
// コンパイルエラー例：
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. 型推論
// 型を省略でき、コンパイラが推論
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE ヒント
// IDE が不足しているメソッドを自動ヒント
```

### リスク

| リスク                       | 影響                                    | 緩和策                   |
| ---------------------------- | --------------------------------------- | ------------------------ |
| 解析複雑性                   | 統一構文が解析複雑性を増大させる可能性  | 再帰下降パーサを使用     |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドの可能性 | コンパイル時単相化最適化 |

---

## 隠し要素 🎮：言語の根源

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
║   此乃爻象之源、语言之边界。                                   ║
║   编译器在此沉默、哲学在此驻足。                               ║
║                                                              ║
║   感谢你触达语言的哲学边界。                                   ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：コンパイラは `Type: Type = Type`
> を正しく処理できない（Type0/Type1 宇宙パラドックスを引き起こす）が、我々はこの「隠し要素」を意図的に残している——コンパイルを試みると、言語の創始者からの禅的なメッセージを受け取る。これは単なる技術的境界ではなく、YaoXiang から型哲学への敬意である。

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

# ジェネリック引数：関数型の一部として、例えば (T: Type, R: Type) -> (...)
# 独立した BNF ルール不要——: Type 引数は通常の関数引数

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

| 用語                   | 定義                                                                                                     |
| ---------------------- | -------------------------------------------------------------------------------------------------------- |
| 宣言                   | `name: type = value` 形式の代入文                                                                        |
| レコード型             | 名前付きフィールドを含む `{ ... }` 型                                                                    |
| インターフェース       | 全フィールドが関数型であるレコード型                                                                     |
| ジェネリック型         | `Name: (T: Type) -> Type = { ... }` として定義された型、型引数を受け取る                                 |
| 名前空間関数           | `Type.name` 形式の関数、Type 名前空間に属する。暗黙のバインディングを含意しない                          |
| メソッドバインディング | `Type.name = func[n]`、func の位置 n を呼び出し側としてバインドし、`obj.name(args)` 構文を利用可能にする |
| ジェネリック関数       | `(T: Type)` 構文を使用する関数、型引数は最初の引数群として機能する                                       |
| メタ型                 | `Type`、言語における唯一の型階層マーカー                                                                 |

---

## ライフサイクルと帰結

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中  │  ← コミュニティでの議論とフィードバックを公開
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
│ (正式設計)  │    │ (原位置保持) │
└─────────────┘    └─────────────┘
```

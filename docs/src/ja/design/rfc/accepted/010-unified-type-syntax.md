---
title: 'RFC-010: 統一型構文 - name: type = value モデル'
status: '承認済み'
author: '晨煦'
updated: '2026-07-14（Never 内蔵型が実装され、#157 がクローズ済み）'
issue: '#127'
---

# RFC-010: 統一型構文 - name: type = value モデル

## 概要

本 RFC は、極めて簡素で統一された型構文モデルを提案する：**すべては `name: type = value`**。

YaoXiang には唯一の宣言形式しかない：

```
identifier : type = expression
```

ここで `type` は任意の型式であり、`expression` は任意の値式である。**`fn` も、`struct` も、`trait`
も、`impl` も、小文字の `type` キーワードも存在しない（ただし `Type`
は元型キーワードとして存在する）**。

> **核心設計**：`Type` 自体が一つのジェネリック型である。`(T: Type) -> Type`
> は「型パラメータ T を受け取る型」を表す。

| 概念             | コード書き方                                                                 |
| ---------------- | ---------------------------------------------------------------------------- |
| 変数             | `x: Int = 42`                                                                |
| 関数             | `add: (a: Int, b: Int) -> Int = a + b`                                       |
| 記録型           | `Point: Type = { x: Float, y: Float }`                                       |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }`                               |
| ジェネリック型   | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                  |
| ジェネリック型   | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`     |
| メソッド         | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| ジェネリック関数 | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`    |

**`Type` は言語における唯一の元型キーワードである**。

> **名前空間 vs メソッドバインド**：`Type.name`
> の接頭辞は**名前空間の所属**を示すだけで、それ以上の意味はない。これは暗黙のバインドを一切トリガーしない。`p.draw(screen)`
> のような `.`
> 呼び出し構文を有効にするには、明示的なバインドが必要である：`Point.draw = draw[0]`。詳細は後述の「名前空間とメソッドバインド」セクションを参照。これは型階層の注記に用いられ、コンパイラが Type0、Type1、Type2... の区別を自動的に処理するため、ユーザーには透過的である。

```yaoxiang
// 核心構文：統一 + 区別

// 変数
x: Int = 42

// 関数（引数名はシグネチャ内で）
add: (a: Int, b: Int) -> Int = a + b

// 記録型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// インターフェース（本質的にはフィールドがすべて関数である記録型）
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
    "Point(${self.x}, ${self.y})"
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
p.draw(screen)           // 構文糖衣 → Point.draw(p, screen)
s: Drawable = p           // 構造的サブタイピング：Point は Drawable を実装
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## 動機

### なぜこの機能が必要なのか？

現在の型システムには、分離された複数の概念が存在する：

- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インターフェース定義構文
- メソッドバインド構文

これらの概念の間には統一性が欠如しており、構文の断片化と学習コストの増大を招いている。

### 設計目標

1. **極限の統一性**：一つの構文ルールですべてをカバー
2. **簡潔でエレガント**：`name: type = value` の対称的な美学
3. **新しいキーワード不要**：既存の構文要素を再利用
4. **理論的な優雅さ**：型自体も `Type` 型の値である
5. **ジェネリック対応**：ジェネリックシステム（RFC-011）とシームレスに統合

### ジェネリックシステムとの統合

RFC-010 の統一構文モデルは、RFC-011 のジェネリックシステム設計と**自然に契合**し、ジェネリックパラメータは統一モデルにシームレスに溶け込む：

```yaoxiang
// 基本ジェネリック（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// ジェネリック関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約は引数型が運ぶ

// Const ジェネリック（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：

- RFC-011 Phase 1（基本ジェネリック）は RFC-010 の**強い依存**である
- 基本ジェネリックなしでは、RFC-010 のジェネリック例はコンパイルできない
- 推奨：RFC-011 Phase 1 と RFC-010 は同期して実装する

## 提案

### 核心原則：型コンストラクタ vs 関数/変数

**これは構文の曖昧性解消ルールを決定する、重要な設計選択である：**

| 書き方              | 意味             | ルール                                       |
| ------------------- | ---------------- | -------------------------------------------- |
| **`x: Type = ...`** | 型コンストラクタ | `: Type` を明示宣言 → 型として強制           |
| **`f = ...`**       | 関数または変数   | `: Type` なし → HM が能動的に関数/変数と推論 |

**なぜこのように設計するのか？**

`{ ... }` 構文自体には曖昧性がある：

- `{ x: Float, y: Float }` は**型リテラル**（記録型）になり得る
- `{ a = 1 + 1 }` は**コードブロック**（実行文、Void を返す）になり得る

**曖昧性解消ルール**：

- **`: Type` がある** → 型コンストラクタとして強制解釈、`{ ... }` は型リテラル
- **`: Type` がない** → HM が能動的に `{ ... }` をコードブロックとして解釈、関数型と推論

```yaoxiang
# ✅ 型コンストラクタ：: Type がある
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void と推論
main: () -> Void = { println("Hello") }

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
├── ジェネリック型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず Type を返す
│
├── ジェネリック型（複数パラメータ）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず Type を返す
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示バインド後にのみドット呼び出し構文が有効
│
└── ジェネリック関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数と推論
```

### 元型階層（コンパイラ内部）

**コンパイラ内部**は宇宙階層
`level: selfpointnum`（文字列で保存、理論上は無限に拡張可能）を維持する。

| Level    | 説明                                |
| -------- | ----------------------------------- |
| `Type0`  | 通常型（`Int`、`Float`、`Point`）   |
| `Type1`  | 型コンストラクタ（`List`、`Maybe`） |
| `Type2+` | 高階コンストラクタ                  |

**ユーザーはこれらの数字を見ることはなく、`: Type` だけを見る**。

### Curry-Howard 同型：型は命题、プログラムは証明

YaoXiang の統一構文 `name: type = value`
は恣意的に選ばれたわけではない——これは Curry-Howard 同型（Curry-Howard
correspondence）の直接的な写像である。この同型は深い事実を明らかにする：**型システムと論理システムは同じものの二つの側面である**。

| 論理（命題）         | 型システム（YaoXiang）              | 例                                   |
| -------------------- | ----------------------------------- | ------------------------------------ |
| 命題 P               | 型 T                                | `Int`、`Bool`                        |
| P が真である証明     | 型 T の一つの値                     | `42: Int`、`true: Bool`              |
| P → Q（含意）        | 関数型 `(P) -> Q`                   | `(x: Int) -> Bool`                   |
| P ∧ Q（連言）        | 記録型 `{ p: P, q: Q }`             | `{ x: Int, y: Bool }`                |
| ∀x.P(x）（全称量化） | ジェネリック関数 `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q（選言）        | 列挙 / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**`name: type = value` の Curry-Howard 下での意味**：

```yaoxiang
// "x: Int = 42" を読み下す：「Int 型の証明が存在し、名前は x、その値は 42」
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" を読み下す：
// 「含意の証明が存在する：Int の証明 a と b を与えれば、Int の証明を構成できる」
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" を読み下す：
// 「Point は命題であり、その証明には Float 証明 x と Float 証明 y の同時提供が必要である」
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要なのか？**

1. **論理的一貫性 = 型安全性**：もし型システムが `T`
   型の値の構築を許すが、正当なランタイム表現が何もないなら、それは論理で偽命題の証明を許すのと同じであり、システムは崩壊する。Curry-Howard は**型安全な言語は本質的に一貫した論理システムである**ことを教えてくれる。

2. **宇宙階層は必要条件である**：後述するように、もし
   `Type: Type`（「型の型も型である」）を許せば、Russell パラドックス（型論では Girard パラドックスとして現れる）を生む。YaoXiang の
   `Type₀ : Type₁ : Type₂ : ...`
   の階層化により、各型は特定の階層のみに属し、決して閉じていない上昇の連鎖を形成し、根本からパラドックスを回避する。これは YaoXiang の型システムが Curry-Howard 意味で**論理的に一貫している**ことを意味する。

3. **統一構文の理論的基礎**：`name: type = value`
   が一つの構文で変数、関数、型、インターフェース、ジェネリックのすべてをカバーできるのは、それらが Curry-Howard 下ではすべて同じこと——**命題に証明を提供すること**——だからである。変数は命題の証拠、関数は含意の証拠、記録は連言の証拠、ジェネリックは全称量化の証拠である。統一構文は人為的に設計された偶然ではなく、Curry-Howard 同型の自然な帰結である。

> **参考文献**：Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM, 58(12),
> 75–84. この記事は平易な言葉で Curry-Howard 同型の歴史と意義を解説している。

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

**ブロックの値 = 末尾式、`return` で関数を抜ける（型 `Never`）**——詳細は
[RFC-010a](010a-tail-expression-and-return.md) を参照。

```yaoxiang
// 単一式形式
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// コードブロック形式：値は末尾式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b                    // 末尾式 → ブロックの値
}

// 複数行コードブロック
calc: (x: Float, y: Float, op: String) -> Float = {
    match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }                    // match を末尾式として
}

// Void 関数：末尾式が Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### 返却ルール

**ブロックの値 = 末尾式（唯一の出口）**：

| 書き方                          | 値                          |
| ------------------------------- | --------------------------- |
| `= expr`（中括弧なし）          | `expr`                      |
| `= { ...; e }`（中括弧あり）    | 末尾式 `e`                  |
| `= { ...; s }`（末尾が文/代入） | `Void`（代入の値は `Void`） |
| `= {}`（空ブロック）            | `Void`                      |

**`return`
の意味**：非局所脱出、**最も近い関数の境界を抜ける**（「ブロックに返す」のではない）、型は
`Never`。`Never <: T` は任意の型に対して成立（爆発原理）なので、`return`
は任意の戻り型の位置に書ける。

```yaoxiang
# 単一式：直接値を返す
add: (a: Int, b: Int) -> Int = a + b

# コードブロック：値は末尾式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# 早期 return：return はブロックを貫通し関数を抜ける（型は Never）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # 末尾式
}
```

**設計根拠**：`{ ... }`
は依存駆動計算ユニット（後述）であり、その評価意味論は単一式とは異なる——中括弧は複数文のコンテキストを導入し、**その値は末尾式で与えられる**ため、「最後の式が戻り値かどうか」という曖昧性がない。

#### `{}` の意味：依存駆動計算ユニット

`{ ... }`
は YaoXiang において単なるコードブロックではない——それは**依存駆動計算ユニット**である。この意味論は関数本体、変数初期化、`spawn`
で一貫している：

**核心ルール**：

- `{}` 内の代入文は記述順序ではなく依存関係で自動ソートされる
- 依存が満たされれば即座に実行、欠けていればブロックして待機
- **ブロックの値 = 末尾式**（返却ルール参照）；`return` は `Never` 型の非局所脱出で、関数を抜ける

```yaoxiang
# 依存駆動：b は a に依存し、コンパイラが自動ソート
result: Int = {
    b = a + 1      # a に依存 → a の後に自動配置
    a = 10         # 依存なし → 先に実行可能
    b              # 末尾式 → ブロックの値は 11
}
```

> **単一式との違い**：`= expr`（中括弧なし）は直接値を返す単純バインディング；`= { ... }`（中括弧あり）は依存駆動計算コンテキストを導入し、複数文を許可し、その値は末尾式で与えられる。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一の並行プリミティブである。`{}`
の依存駆動意味論を利用して自動並列化を実現する：

- `spawn { ... }` 内の直接の代入は自動的に並列タスクを生成する
- 依存が満たされたタスクは即座に並列実行される
- 呼び出し側はすべての子タスクの完了をブロック待機する

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a と依存なし、並列実行）
    c = process(a, b)         # a, b に依存 → 両方の完了を待って実行
    c                         # 末尾式 → spawn の値
}
// 呼び出し側はここでブロックし、spawn ブロック内の全タスク完了を待つ
```

> **詳細定義**：`spawn` の完全な意味論、タスク生成ルール、ブロックモデルについては
> `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型の定義と生ポインタ操作に用いられる。`{}`
の評価意味論を利用して、型定義を一つ外側のスコープに引き渡し（値の出口は末尾式）：

**核心ルール**：

- `unsafe {}` 内で型定義と生ポインタ操作が可能
- **末尾式**が `unsafe {}` の値を与える（型定義は一つ外側のスコープに引き渡される）
- 返される型は `unsafe {}` 外で利用できる
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 生ポインタ
    }
    SqliteDb           # 末尾式 → unsafe ブロックの値
}

# SqliteDb は unsafe ブロック外で利用できる
db = sqlite3_open("test.db")

# ❌ コンパイルエラー：handle フィールドには unsafe 権限が必要
handle = db.handle

# ✅ メソッド呼び出し経由
db.close()
```

> **詳細定義**：`unsafe` の完全な意味論、FFI 型定義、メソッドバインドについては `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang 統一構文の核心であり、フィールド、デフォルト値、バインドメソッド、インターフェース実装を含む：

##### 基本型

**記録型**：フィールドリスト、フィールド型は任意の型式を取れる。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**：フィールドはデフォルト値を持て、構築時にオプションになる。

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

**デフォルト値なしのフィールド**：構築時に必ず提供しなければならない。

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

##### 内蔵型

YaoXiang の識別子体系は三層に分かれ、異なるコンパイラ段階で順次認識される：

1. **キーワード**（parser 独立 token）— 制御構造と宣言キーワード、例：`if`、`match`、`pub`、`return`
2. **リテラル予約語**（parser 独立 token）—
   `true`、`false`、`void`、`Type`、通常の識別子にはできない
3. **内蔵型名**（type
   checker 事前登録）— パーサは通常識別子として扱い、型チェッカが解析を担当。**予約語ではなく、シャドウ可能（非推奨）**

`void`（小文字、リテラル予約語）と `Void`（大文字、内蔵型名）の違い：`void`
は値リテラル（Unit の唯一の値と等しい）、`Void`
は型名（Unit 型と等しい、論理 ⊤）である。`let x: Void = void` は合法。

内蔵型名のプリセット：

| 型       | 論理対応     | 説明                                                                                                                                                                                                                                 |
| -------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、この型に住まう値は全くない。「不可能」を表す——発散、panic、デッドコード。`Never <: T` は任意の `T` に対して成立（爆発原理）。関数が `Never` を返すのは正常終了しないことを示す。**キーワードではなく内蔵型名。** |
| `Void`   | ⊤（真/Unit） | ちょうど一つの住人（デフォルト void 値）。`x: Void = <デフォルト>` は合法。和型の単位元・積型の単位元に対応——`Void` はゼロフィールド積型（Unit）、`Never` はゼロ変体和型。                                                           |
| `Int`    | —            | 符号付き整数                                                                                                                                                                                                                         |
| `Float`  | —            | 浮動小数点数                                                                                                                                                                                                                         |
| `Bool`   | —            | ブール値：`true` / `false`                                                                                                                                                                                                           |
| `Char`   | —            | Unicode 文字                                                                                                                                                                                                                         |
| `String` | —            | 文字列                                                                                                                                                                                                                               |

##### メソッドバインド

**方式 1：型定義体内で直接外部関数をバインド**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置 0 にバインド、カリー化後 method: (b: Point) -> Float
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

**方式 2：匿名関数 + 位置バインド**

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        (dx * dx + dy * dy).sqrt()      # 末尾式
    })
}
// 構文：((params) => body)[position]
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

##### インターフェース実装

**インターフェース名は型体内に書き、コンパイラが自動的に実装をチェック**

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

**`Type.name`
の接頭辞は名前空間の所属を示す**だけで、それ以上の意味はない。暗黙のバインドは一切トリガーされない。

```yaoxiang
// 名前空間関数：Point 名前空間下の通常の関数
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

// 呼び出し：通常の関数呼び出しそのもの
Point.draw(p, screen)
Point.serialize(p)
```

> **注意**：`self` はキーワードではなく、引数名の慣習に過ぎない。`p`、`this`、`x`
> と書いても効果は完全に同じ。コンパイラは引数名を見ず、型を見る。

##### メソッドバインド（唯一の方法）

`p.draw(screen)` のような `.`
メソッド呼び出し構文を有効にするには、**明示的にバインドしなければならない**。`[position]`
構文は関数を「メソッド」としてバインドする唯一の仕組みである（詳細は RFC-004）。

```yaoxiang
// 関数を定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示バインド — これ以降のみ p.draw(screen) 構文が使える
Point.draw = draw[0]   // 位置 0 の引数（&Point）は呼び出し側が埋める

// 使用
p.draw(screen)          // 構文糖衣 → draw(&p, screen)
Point.draw(p, screen)   // 二つの呼び出し方は等価

// [0] を書かない = バインドしない。Point.draw は通常の関数別名であり、. 構文はない
Point.draw = draw       // バインドしない：Point.draw(p, screen) のみ可能
```

**デフォルト動作**：`[n]`
を書かない = どの引数もバインドしない。ユーザーはどの引数を呼び出し側が埋めるかを明示的に決定しなければならない。

**複数位置バインド**：

```yaoxiang
// 複数位置をバインド（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドを通常関数に変換）：

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
    item.serialize()
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

// 具体インスタンス化（RFC-023 構文）
IntList: Type = List(Int)

IntList.push = {
    self.data.append(item)
    self.length = self.length + 1
}

List.push = (type: Type) -> {
    (self: List(type), item: type) -> {
        self.data.append(item)
        self.length = self.length + 1
    }
}

IntList.push(Int)(self, item)  // 呼び出し例

// ジェネリックメソッド（RFC-023 構文：型パラメータは呼び出し点で自動推論）
List.push: (self: List(T), item: T) -> Void = {
    self.data.append(item)
    self.length = self.length + 1
}

List.get: (self: List(T), index: Int) -> Maybe(T) = {
    if index >= 0 and index < self.length {
        Maybe.Just(self.data[index])
    } else {
        Maybe.Nothing
    }
}
```

#### 6. ジェネリック呼び出し構文

ジェネリック型とジェネリック関数の呼び出しは統一して `()` 構文を用いる。`[]`
はいかなるジェネリックコンテキストでも使用されない。

**核心ルール**：

1. **`()` ですべての適用を行う**：型適用、関数呼び出し、値構築はすべて `()`

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

2. **Type が左、値が右**：`name: type = value`——Type パラメータは左で宣言され、右は常に具体値。空コンテナ
   `List()` の `T` は左の型注釈から取得しなければならない。

3. **型情報は一度だけ書けばよい**——引数宣言時、コンパイラがそれを運んで流動する：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左で一度書く
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String は numbers と f の型から自動
```

4. **値構築は要素から型を推論**：

```yaoxiang
x = List(1, 2, 3)       // List(Int) と推論
y = List("a", "b")      // List(String) と推論
z = List()              // ❌ コンパイルエラー：T を推論できない
z: List(Int) = List()   // ✅ T=Int は左の注釈から
```

5. **型エイリアス**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` →
> `List(1,2,3)`。古い `[]` ジェネリック構文は完全に削除された。`[]`
> は配列/リストリテラルとインデックスアクセスのみに使用される。

### 例

#### 完全な例

```yaoxiang
// ======== 1. インターフェース定義 ========
// インターフェース = フィールドがすべて関数型である記録型
// インターフェースには self 引数は不要 — インターフェースは「呼び出し位置を除いた関数シグネチャ」のみを定義する

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

// ======== 3. メソッド実装（通常関数 + 明示バインド）========

// 関数を定義（self は慣習名に過ぎず、キーワードではない）
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

bounding_box: (p: &Point) -> Rect = {
    Rect(p.x - 1, p.y - 1, 2, 2)
}

serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

translate: (p: &Point, dx: Float, dy: Float) -> Point = {
    Point(p.x + dx, p.y + dy)
}

scale: (p: &Point, factor: Float) -> Point = {
    Point(p.x * factor, p.y * factor)
}

distance: (p1: &Point, p2: &Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// 明示バインド — バインド後にのみドット呼び出し構文が有効
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
    "Rect(${r.x}, ${r.y}, ${r.width}, ${r.height})"
}
Rect.serialize = serialize[0]

translate: (r: &Rect, dx: Float, dy: Float) -> Rect = {
    Rect(r.x + dx, r.y + dy, r.width, r.height)
}
Rect.translate = translate[0]

scale: (r: &Rect, factor: Float) -> Rect = {
    Rect(r.x * factor, r.y * factor, r.width * factor, r.height * factor)
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

// ジェネリック関数（RFC-023 構文：呼び出し時に型パラメータを省略、自動推論）
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
            // 比較：self 引数を除いた後が一致するべき
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

インターフェース型は直接代入をサポートし、コンパイラは代入の右辺型に応じて最適な呼び出し戦略を自動選択する：

```yaoxiang
// 具体型を直接代入 → コンパイル時に具体型確定、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：circle_draw(screen) を直接呼び出し、vtable なし

// 関数戻り値 → コンパイル時に具体型不定、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable でメソッド検索

// 異種コレクション → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable でメソッド検索
}
```

**コンパイル時最適化戦略**：

| シナリオ                         | 推論結果      | 呼び出し方式                       |
| -------------------------------- | ------------- | ---------------------------------- |
| `d: Drawable = Circle(1)`        | 具体型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()`      | 不明          | vtable                             |
| `shapes: List(Drawable) = [...]` | 異種          | vtable                             |

**ルール**：

1. 右辺が具体型コンストラクタでコンパイル時確定可能な場合、直接呼び出し IR を生成
2. 右辺型がコンパイル時確定不可能な場合、vtable 機構にフォールバック
3. vtable がランタイム多態性の正確性を保証するセーフティネットとなる

### ダックタイピングサポート

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
| `impl` キーワードが必要                  | キーワード不要、インターフェース名は型体後に記述                                             |

### 廃止：`|` 変体構文

> **廃止宣言（2026-07-25）**：`|` 変体構文は正式に廃止され実装から削除された。

以下の書き方は**もうサポートされない**：

```
type Color = red | green | blue                # ❌ 廃止
type Result(T, E) = ok(T) | err(E)             # ❌ 廃止
type Option(T) = some(T) | none                # ❌ 廃止
```

和型（sum
type）の表現には記録型を統一使用する。記録型のフィールドがすべて関数であり、すべてその型自身を返すとき、それは和型である：

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

**設計根拠**：

1. **特殊ケースの除去**：`|` は BNF における唯一の `name: type = value`
   形式でない構文。削除により、`type_expr`
   生成規則は完全に統一され、parser は変体型のために独立したパスと先読みバックトラックを維持する必要がなくなる。
2. **数学的等価性**：Curry-Howard 同型の下で、選言 P ⊕
   Q に対応する和型は、「フィールドがすべて自身型を返す関数である」記録型と等価である。両者は同じ意味論を表現し、二つの構文は不要である。
3. **ゼロ破壊性**：削除前 `|`
   構文は parser で半サポート（引数なし変体は解析可能だが引数型は単態化時に失われる）であり、ユーザーコードの依存は皆無。
4. **AST 簡略化**：`Type::Variant(Vec<VariantDef>)` ノードを削除し、すべての変体型は `Type::Struct`
   経路に統一、下流の typecheck/mono/formatter の特殊分岐がすべて解消される。

> **注**：和型の意味論的属性（match の網羅性チェック、tagged
> union メモリレイアウト）は typecheck 層で `Type::Struct`
> 構造から推論され、独立した AST ノードには依存しない。

### 論理演算子：`and` / `or` / `!`（権威的定義、Zig 式）

> **定義宣言（2026-08-03）**：論理演算子の権威的形式はキーワード `and` / `or` + シンボリック単項 `!`
> である（SPEC `syntax.md`
> §2.2 優先度表と一致）。本設計は Zig に準拠する：**短絡制御フローにはキーワード、純粋な単項演算にはシンボル**。初期実装が C に流された
> `&&` / `||` および中間状態のキーワード `not` はすべて削除された。

**意味論**：

| 演算子 | 優先度（SPEC §2.2）   | 結合性   | 意味                               |
| ------ | --------------------- | -------- | ---------------------------------- |
| `!`    | 3（単項前置、強結合） | 右から左 | 論理否定（純関数、制御フローなし） |
| `and`  | 10                    | 左から右 | 短絡論理積                         |
| `or`   | 10                    | 左から右 | 短絡論理和                         |

```yaoxiang
# 短絡評価：and の左辺が false / or の左辺が true のとき、右辺は実行されない
if x != 0 and y / x > 1 { ... }   # x == 0 のときゼロ除算にならない

# 強結合：!a == b ≡ (!a) == b（Zig 式；Python の not a == b ≡ not (a == b) と逆）
!3 == 4          # false：(!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs))、呼び出し後に反転
```

以下の書き方は**もうサポートされない**（lexer がエラーで対応する書き方を提示）：

```
x && y     # ❌ 削除済み、x and y を使用
x || y     # ❌ 削除済み、x or y を使用
not x      # ❌ 削除済み、!x を使用（not は通常の識別子に戻る；!= は影響なし）
```

**設計根拠**（Zig に準拠、ziglang/zig#272 / #6625）：

1. **短絡は制御フロー → キーワード；純関数は演算 → シンボル**。`and` / `or`
   は評価順序を変える（右辺を必要に応じてスキップ）、`if` と同性質のためキーワード；`!`
   は既に評価されたオペランドに対して純粋な否定を行う、`-` `+`
   と同性質のためシンボル。YaoXiang のエラー伝播は `?`（§2.11）を使用、`!` に競合はない。
2. **強結合による曖昧性解消**：`!`
   は視覚的にオペランドに「密着」し、高い優先度が一目瞭然；キーワード `not`
   はオペランドとの間にスペースを強制されるため、どこに結合するか（`not a == b`）が脳内で曖昧になりやすい。
3. **曖昧性解消**：`&` は二つの役割を持つ——借用トークン（`&p` /
   `&mut p`、RFC-009）とビット AND（SPEC §2.2 優先度 8）。さらに `&&`
   を導入すると一つのシンボルが三つの意味を担うことになる。`and` / `or` / `!`
   により借用、ビット演算、論理の三つの概念が視覚的に完全に分離される。
4. **先例**：Zig（同じエコシステム位置の現代的システム言語）がまさに `and` / `or` キーワード + `!`
   シンボルの組み合わせ；Python / Lua / Ada / SQL は全キーワード（含
   `not`）、C 族は全シンボルを使用——YaoXiang は Zig のハイブリッドを採用し、両者の利点を享受する。
5. **Curry-Howard 一貫性**：型は命題（前述の同型節参照）、 refinement 型内の論理結合は `and` / `or`
   （例： `{ 0 <= idx and idx < arr.len }`）と書くのが命題の自然な表現；`!`
   は単項否定シンボルとして ¬ に対応する。

> **実装**：`and` / `or`
> は IR 層で短絡ジャンプ列に展開される（`a and b ≡ if a { b } else { false }`）、 `!`
> は単項強結合として解析される（オペランドは `BP_UNARY + 1` で）。回帰テスト：
> `tests/yaoxiang/01-syntax/basics/logical_ops.yx`、`logical_not.yx`。

## 構文設計の説明：名前付き関数は本質的に Lambda の構文糖衣

### 核心的理解

**名前付き関数と Lambda 式は同じものである！**
唯一の違いは、名前付き関数が Lambda に名前をつけたことだけ。

```yaoxiang
// この二つは本質的に完全に同じ
add: (a: Int, b: Int) -> Int = a + b           // 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全等価）
```

### 構文糖衣モデル

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**要点**：シグネチャが引数型を完全に宣言している場合、Lambda 頭部の引数名は冗長になり、省略可能。

### 引数スコープルール

**引数は外側変数を覆い隠す**：シグネチャ内の引数スコープは関数本体を覆い、内部スコープの優先度がより高い。

```yaoxiang
x = 10  // 外側変数

double: (x: Int) -> Int = x * 2  // ✅ 引数 x が外側 x を覆う、結果は 20
```

### 注釈位置の柔軟性

型注釈は以下のいずれの位置にも書け、**少なくとも一箇所に書けばよい**：

| 注釈位置       | 形式                                     | 説明            |
| -------------- | ---------------------------------------- | --------------- |
| シグネチャのみ | `double: (x: Int) -> Int = x * 2`        | ✅ 推奨         |
| Lambda 頭のみ  | `double = (x: Int) => x * 2`             | ✅ 合法         |
| 両方           | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャ完全、Lambda 頭部省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda 頭で型を注釈
double = (x: Int) => x * 2

// ✅ 合法：両方に注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の利点

| 特性       | 利点                                                            |
| ---------- | --------------------------------------------------------------- |
| **簡潔**   | シグネチャ完全時、引数名の重複記述が不要                        |
| **柔軟**   | Lambda 形式を残し、好みで選べる                                 |
| **一貫**   | 変数宣言 `x: Int = 42` と統一パターンを維持                     |
| **直感的** | `name: Type = body` は「名前 name、型 Type、値 body」に直接対応 |

## トレードオフ

### 利点

| 利点             | 説明                                           |
| ---------------- | ---------------------------------------------- |
| 極限の統一性     | 一つの構文ルールですべてをカバー               |
| 理論的優雅さ     | 完全に対称的な `name: type = value`            |
| 新キーワード不要 | 既存の構文要素を再利用                         |
| 実装容易         | コンパイラは一つの宣言形式だけを処理すればよい |
| 学習容易         | 一つのパターンを覚えればすべてのコードが書ける |
| 拡張容易         | 新機能をこのモデルに自然に組み込める           |

### 欠点

| 欠点     | 説明                                        |
| -------- | ------------------------------------------- |
| 命名規約 | メソッドは `Type.method` 命名規則に従う必要 |
| 冗長     | 完全構文は簡略構文より長いが、推論可能      |
| 学習曲線 | 統一モデルの理解が必要                      |

### 緩和策

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
// IDE が不足メソッドを自動ヒント
```

### リスク

| リスク                       | 影響                                    | 緩和策                   |
| ---------------------------- | --------------------------------------- | ------------------------ |
| 解析複雑度                   | 統一構文が解析複雑度を増す可能性        | 再帰下降パーサを使用     |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドの可能性 | コンパイル時単態化最適化 |

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
║   此乃爻象之源、語言之边界。                                   ║
║   编译器在此沉默、哲学在此驻足。                               ║
║                                                              ║
║   感谢你触达语言的哲学边界。                                   ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：コンパイラは `Type: Type = Type`
> を正しく処理できない（Type0/Type1 宇宙パラドックスを引き起こす）が、私たちはこの「イースターエッグ」を意図的に残している——コンパイルを試みると、言語創設者からの禅のメッセージを受け取る。これは技術的境界だけでなく、YaoXiang による型哲学への敬意でもある。

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

# ジェネリック引数：関数型の一部として、例如 (T: Type, R: Type) -> (...)
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

| 用語             | 定義                                                                                                     |
| ---------------- | -------------------------------------------------------------------------------------------------------- |
| 宣言             | `name: type = value` 形式の代入文                                                                        |
| 記録型           | 名前付きフィールドを含む `{ ... }` 型                                                                    |
| インターフェース | フィールドがすべて関数型である記録型                                                                     |
| ジェネリック型   | `Name: (T: Type) -> Type = { ... }` と定義された型、型引数を受け取る                                     |
| 名前空間関数     | `Type.name` 形式の関数、Type 名前空間に属する。暗黙のバインドを含意しない                                |
| メソッドバインド | `Type.name = func[n]`、func の位置 n を呼び出し側としてバインド、`obj.name(args)` 構文を使えるようにする |
| ジェネリック関数 | `(T: Type)` 構文を用いる関数、型引数は最初の引数群として扱う                                             |
| 元型             | `Type`、言語における唯一の型階層マーカー                                                                 |

---

## ライフサイクルと帰趣

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティでの議論とフィードバックを募集中
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

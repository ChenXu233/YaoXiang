---
title: 'RFC-010: 統一型構文 - name: type = value モデル'
status: '承認済み'
author: '晨煦'
updated: '2026-09-25'
issue: '#127'
---

# RFC-010: 統一型構文 - name: type = value モデル

## 概要

本 RFC は極めて簡潔で統一された型構文モデルを提案する：**すべては `name: type = value`**。

YaoXiang には唯一の宣言形式しかない：

```
identifier : type = expression
```

ここで `type` は任意の型式であり、`expression` は任意の値式である。 **`fn` も `struct` も `trait` も
`impl` もないし、小文字の `type` キーワードもない（ただし `Type` をメタ型キーワードとして持つ）**。

> **核心設計**：`Type` 自体が一つの汎用型である。`(T: Type) -> Type`
> は「型引数 T を受け取る型」を表す。

| 概念             | コード記法                                                                   |
| ---------------- | ---------------------------------------------------------------------------- |
| 変数             | `x: Int = 42`                                                                |
| 関数             | `add: (a: Int, b: Int) -> Int = a + b`                                       |
| 記録型           | `Point: Type = { x: Float, y: Float }`                                       |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }`                               |
| 汎用型           | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                  |
| 汎用型           | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`     |
| メソッド         | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| 汎用関数         | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`    |

**`Type` は言語における唯一のメタ型キーワードである**。

> **名前空間 vs メソッドバインド**：`Type.name`
> という前置は**名前空間の所属**を示すだけで、それ以外何の意味も持たない。`p.draw(screen)` という
> `.`
> 呼び出し構文を有効化するには、明示的なバインドが必要である：`Point.draw = draw[0]`。詳細は後述の「名前空間とメソッドバインド」節を参照。型階層の注記に用いられ、コンパイラが Type0、Type1、Type2... の区別を自動的に処理し、ユーザーには透過的である。

```yaoxiang
// 核心構文：統一 + 区別

// 変数
x: Int = 42

// 関数（引数名はシグネチャ内に）
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

// 汎用型（(T: Type) -> Type = 型引数を受け取る汎用型）
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

これらの概念の間には統一性が欠如しており、構文の断片化と学習コストの増大を招いている。

### 設計目標

1. **極限の統一**：一つの構文規則ですべてをカバー
2. **簡潔で優美**：`name: type = value` の対称的美学
3. **新キーワード不要**：既存の構文要素を再利用
4. **理論的優美さ**：型自体も Type 型の値
5. **泛型との親和性**：（RFC-011）とシームレスに統合

### 泛型システムとの統合

RFC-010 の統一構文モデルは、RFC-011 の泛型システム設計と**自然に契合**し、泛型引数は統一モデルにシームレスに溶け込む：

```yaoxiang
// 基本泛型（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// 泛型関数（RFC-023 構文：シグネチャ内の Type 位置は省略可、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約は引数型が運ぶ

// Const 泛型（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：

- RFC-011 Phase 1（基本泛型）は RFC-010 の**強い依存先**
- 基本泛型がなければ、RFC-010 の泛型例はコンパイルできない
- 推奨：RFC-011 Phase 1 と RFC-010 を同期して実装する

## 提案

### 核心原則：型コンストラクタ vs 関数/変数

**これは重要な設計選択であり、構文の曖昧性解消ルールを決定する：**

| 記法                | 意味             | 規則                                         |
| ------------------- | ---------------- | -------------------------------------------- |
| **`x: Type = ...`** | 型コンストラクタ | `: Type` を明示宣言 → 型として強制           |
| **`f = ...`**       | 関数または変数   | `: Type` なし → HM が能動的に関数/変数と推論 |

**なぜこのように設計するのか？**

`{ ... }` 構文自体に曖昧性がある：

- `{ x: Float, y: Float }` は**型リテラル**（記録型）になり得る
- `{ a = 1 + 1 }` は**コードブロック**（実行文、戻り値 Void）になり得る

**曖昧性解消のルール**：

- **`: Type` がある** → 型コンストラクタとして強制解析、`{ ... }` は型リテラル
- **`: Type` がない** → HM が能動的に `{ ... }` をコードブロックとして解析、関数型と推論

```yaoxiang
# ✅ 型コンストラクタ：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void と推論
main: () -> Void = { println("Hello") }

# ❌ エラー：: Type なし、コンパイラが { ... } を型として解析できない
Point = { x: Float, y: Float }  // HM は関数と推論、型ではない！
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
│   └── Point: Type = { x: Float, y: Float }  # 必ず戻り値：Type
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必ず戻り値：Type
│
├── 汎用型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず戻り値：Type
│
├── 汎用型（複数引数）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず戻り値：Type
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示バインド後にのみドット呼び出し構文が有効
│
└── 汎用関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数と推論
```

### メタ型階層（コンパイラ内部）

**コンパイラ内部**は宇宙階層
`level: selfpointnum`（文字列で保存、理論上は無限に拡張可能）を維持する。

| Level    | 説明                                |
| -------- | ----------------------------------- |
| `Type0`  | 日常型（`Int`、`Float`、`Point`）   |
| `Type1`  | 型コンストラクタ（`List`、`Maybe`） |
| `Type2+` | 高階コンストラクタ                  |

**ユーザーはこれらの数字を見ることはなく、`: Type` のみを見る**。

### Curry-Howard 同型：型は命題、プログラムは証明

YaoXiang の統一構文 `name: type = value`
は恣意的に選ばれたわけではない——ちょうど Curry-Howard 同型（Curry-Howard
correspondence）の直接的な写像となっている。この同型は深い事実を明らかにする：**型システムと論理システムは同じものの二つの側面である**。

| 論理（命題）       | 型システム（YaoXiang）      | 例                                   |
| ------------------ | --------------------------- | ------------------------------------ |
| 命題 P             | 型 T                        | `Int`、`Bool`                        |
| P が真である証明   | 型 T の一つの値             | `42: Int`、`true: Bool`              |
| P → Q（含意）      | 関数型 `(P) -> Q`           | `(x: Int) -> Bool`                   |
| P ∧ Q（連言）      | 記録型 `{ p: P, q: Q }`     | `{ x: Int, y: Bool }`                |
| ∀x.P(x（全称量化） | 汎用関数 `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q（選言）      | 列挙 / tagged union         | `Maybe: (T: Type) -> Type = { ... }` |

**`name: type = value` の Curry-Howard における意味**：

```yaoxiang
// "x: Int = 42" を「x という名前を持つ Int 型の証明が存在し、その値は 42 である」と読む
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" を：
//「含意の証明が存在する：Int の証明 a と b を与えれば、Int の証明を構成できる」と読む
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" を：
//「Point は命題であり、その証明には Float 証明 x と Float 証明 y の同時提供が必要である」と読む
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要なのか？**

1. **論理的一貫性 = 型安全性**：もし型システムが `T`
   型の値を正当な実行時表現なしに構築できるなら、それは論理において偽の命題の証明を許すようなもので——システムは崩壊する。Curry-Howard が教えてくれるのは：**型安全な言語は本質的に一貫した論理システムである**ということ。

2. **宇宙階層の必要性**：後述のように、もし
   `Type: Type`（すなわち「型の型もまた型である」）を許すと Russell パラドックス（型論では Girard パラドックスとして現れる）が発生する。YaoXiang の
   `Type₀ : Type₁ : Type₂ : ...`
   の階層化により、各型はいずれかの階層にのみ属し、決して閉じない上昇の連鎖が形成され、根本的にパラドックスを回避する。これは YaoXiang の型システムが Curry-Howard の意味で**論理的に一貫している**ことを意味する。

3. **統一構文の理論的基礎**：`name: type = value`
   が一つの構文で変数、関数、型、インターフェース、泛型のすべてをカバーできるのは、まさにそれらが Curry-Howard の下では同じこと——**命題に対する証明を提供する**——だからである。変数は命題の証拠、関数は含意の証拠、記録は連言の証拠、泛型は全称量化の証拠。統一構文は人為的な設計の偶然ではなく、Curry-Howard 同型の自然な帰結である。

> **参考文献**：Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM, 58(12),
> 75–84. この論文は Curry-Howard 同型の歴史と意味を平易な言葉で解説している。

### 構文定義

#### 1. 変数宣言

```yaoxiang
// 基本構文
x: Int = 42
name: String = "Alice"
flag: Bool = true

// 型推論（省略可）
y = 100  // Int と推論
```

#### 2. 関数定義

**ブロックの値 = 末尾式、`return` は関数を抜ける（型 `Never`）**——詳細は
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

#### 返却規則

**ブロックの値 = 末尾式（唯一の出口）**：

| 記法                            | 値                          |
| ------------------------------- | --------------------------- |
| `= expr`（中括弧なし）          | `expr`                      |
| `= { ...; e }`（中括弧あり）    | 末尾式 `e`                  |
| `= { ...; s }`（末尾が文/代入） | `Void`（代入の値は `Void`） |
| `= {}`（空ブロック）            | `Void`                      |

**`return`
の意味**：非局所脱出、**最も近い関数境界から抜ける**（「ブロックに返す」のではない）、型は
`Never`。`Never <: T` は任意の型に対して成立（爆発原理）なので、`return`
は任意の戻り型位置に現れ得る。

```yaoxiang
# 単一式：直接値を返す
add: (a: Int, b: Int) -> Int = a + b

# コードブロック：値は末尾式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# 早期リターン：return はブロックを貫通し、関数を抜ける（型 Never）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # 末尾式
}
```

**設計理由**：`{ ... }`
は依存駆動計算ユニット（以下参照）であり、その評価意味は単一式とは異なる——中括弧は複数文の文脈を導入し、**その値は末尾式によって決定される**ため、「最後の式が戻り値か」という曖昧性が生じない。

#### `{}` の意味：依存駆動計算ユニット

YaoXiang における `{ ... }`
は単なるコードブロックではない——**依存駆動計算ユニット**である。この意味は関数本体、変数初期化、`spawn`
において一貫している：

**核心ルール**：

- `{}` 内の代入文は記述順序ではなく依存関係に従って自動的に並べられる
- 依存が揃えば即座に実行され、欠けていればブロックして待機する
- **ブロックの値 = 末尾式**（返却規則参照）；`return` は `Never` 型の非局所脱出で、関数を抜ける

```yaoxiang
# 依存駆動：b は a に依存し、コンパイラが自動的に並べ替える
result: Int = {
    b = a + 1      # a に依存 → a の後に自動配置
    a = 10         # 依存なし → 先に実行可能
    b              # 末尾式 → ブロックの値 11
}
```

> **単一式との違い**：`= expr`（中括弧なし）は直接値を返す単純な束縛；`= { ... }`（中括弧あり）は依存駆動計算コンテキストを導入し、複数文を許し、その値は末尾式で決定される。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang 唯一の並行プリミティブである。`{}`
の依存駆動意味論を利用して自動並列化を実現する：

- `spawn { ... }` 内の直接の子代入は自動的に並列タスクを生成する
- 依存の整ったタスクは即座に並列実行される
- 呼び出し側はすべての子タスクの完了をブロックして待機する

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a と依存なし、並列実行）
    c = process(a, b)         # a, b に依存 → 両方完了後に実行
    c                         # 末尾式 → spawn の値
}
// 呼び出し側は spawn ブロック内すべてのタスクが完了するまでブロック
```

> **詳細定義**：`spawn` の完全な意味、タスク生成規則、ブロックモデルについては
> `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型の定義と生ポインタ操作に使用される。`{}`
の評価意味論を利用して型定義を上位スコープに委ねる（値出口は末尾式）：

**核心ルール**：

- `unsafe {}` 内で型を定義し、生ポインタを操作できる
- **末尾式**が `unsafe {}` の値を与える（型定義は上位スコープに委ねられる）
- 返される型は `unsafe {}` 外でも使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 生ポインタ
    }
    SqliteDb           # 末尾式 → unsafe ブロックの値
}

# SqliteDb は unsafe ブロック外で使用可能
db = sqlite3_open("test.db")

# ❌ コンパイルエラー：handle フィールドには unsafe 権限が必要
handle = db.handle

# ✅ メソッド呼び出し経由
db.close()
```

> **詳細定義**：`unsafe` の完全な意味、FFI 型定義、メソッドバインドについては `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang 統一構文の中核であり、フィールド、デフォルト値、束縛メソッド、インターフェース実装を含む：

##### 基本型

**記録型**：フィールドリスト、フィールド型は任意の型式を取り得る。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**：フィールドはデフォルト値を持つことができ、構築時にはオプション。

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

**デフォルト値のないフィールド**：構築時に必ず提供しなければならない。

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
   `true`、`false`、`void`、`Type`、通常の識別子にはできない
3. **組み込み型名**（type
   checker 事前登録）— パーサは通常の識別子として扱い、型チェッカが解析を担当。**予約語ではなく、シャドウ可能（推奨されない）**

`void`（小文字、リテラル予約語）と `Void`（大文字、組み込み型名）の違い：`void`
は値リテラル（Unit 唯一の値に等しい）、`Void`
は型名（Unit 型に等しい、論理 ⊤）。`let x: Void = void` は合法。

組み込み型名：

| 型       | 論理対応     | 説明                                                                                                                                                                                                                                         |
| -------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、いかなる値もこの型に居住できない。「不可能」を表す——発散、パニック、デッドコード。`Never <: T` は任意の `T` に対して成立（爆発原理）。`Never` を返す関数は正常に返らないことを示す。**キーワードではなく組み込み型名。** |
| `Void`   | ⊤（真/Unit） | ちょうど一つの居住者（デフォルト void 値）。`x: Void = <デフォルト>` は合法。直積の単位元に、和の零元に——`Void` は零フィールド直積（Unit）、`Never` は零変体和。                                                                             |
| `Int`    | —            | 符号付き整数                                                                                                                                                                                                                                 |
| `Float`  | —            | 浮動小数点数                                                                                                                                                                                                                                 |
| `Bool`   | —            | ブール値：`true` / `false`                                                                                                                                                                                                                   |
| `Char`   | —            | Unicode 文字                                                                                                                                                                                                                                 |
| `String` | —            | 文字列                                                                                                                                                                                                                                       |

##### メソッドの束縛

**方法 1：型定義体内で直接外部関数を束縛**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置 0 に束縛、カリー化後 method: (b: Point) -> Float
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

**方法 2：無名関数 + 位置束縛**

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

**インターフェース名は型体内に書き、コンパイラが自動的に実装を検査する**

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
前置は名前空間の所属を示すだけで、それ以外何の意味も持たない。暗黙のバインドは一切トリガーしない。**

```yaoxiang
// 名前空間関数：Point 名前空間下の通常関数
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
> と書いても全く同じ効果。コンパイラは引数名を見ず、型を見る。

##### メソッドバインド（唯一の方法）

`p.draw(screen)` という `.`
メソッド呼び出し構文を有効にするには、**明示的なバインドが必要**。`[position]`
構文は関数を「メソッド」としてバインドする唯一の仕組みである（詳細構文は RFC-004 を参照）。

```yaoxiang
// 関数定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示バインド — これ以降で p.draw(screen) 構文が有効になる
Point.draw = draw[0]   // 位置 0 の引数（&Point）が呼び出し側から埋められる

// 使用
p.draw(screen)          // 構文糖 → draw(&p, screen)
Point.draw(p, screen)   // 二つの呼び出し方は等価

// [0] を書かない = バインドしない。Point.draw は通常の関数エイリアスで、. 構文はない
Point.draw = draw       // バインドなし：Point.draw(p, screen) のみ可能
```

**デフォルト動作**：`[n]`
を書かない = どの引数もバインドしない。ユーザーが明示的にどの引数を呼び出し側から埋めるかを決定しなければならない。

**複数位置バインド**：

```yaoxiang
// 複数位置をバインド（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドを通常関数へ）：

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

#### 5. 汎用型

```yaoxiang
// 基本泛型（RFC-011 Phase 1）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T:Type)-((self: List(T), item: T) -> Void),
    get: (T:Type)->((self: List(T), index: Int) -> Maybe(T))
}

// 具象インスタンス化（RFC-023 構文）
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

// 汎用メソッド（RFC-023 構文：型引数は呼び出し位置で自動推論）
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

#### 6. 汎用呼び出し構文

汎用型と汎用関数の呼び出しは統一して `()` 構文を用いる。`[]`
はどの汎用コンテキストでも使用されない。

**核心ルール**：

1. **`()` ですべての適用を行う**：型適用、関数呼び出し、値構築はすべて `()`

```yaoxiang
# 型注釈
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から来る
empty: List(Int) = List()

# 汎用関数呼び出し——型は引数から自動流入
strings = map(numbers, f)
// T=Int は numbers: List(Int) から
// R=String は f: (Int) -> String から
```

2. **Type は左、値は右**：`name: type = value`——Type 引数は左で宣言され、右は常に具体的な値。空コンテナ
   `List()` の `T` は左側の型注釈から取得しなければならない。

3. **型情報は一度だけ書けばよい**——引数宣言時に、コンパイラがそれを運ぶ：

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
> `List(1,2,3)`。古い `[]` 汎用構文は完全に削除された。`[]`
> は配列/リストリテラルとインデックスアクセスのみに使用される。

### 例

#### 完全な例

```yaoxiang
// ======== 1. インターフェース定義 ========
// インターフェース = フィールドがすべて関数型である記録型
// インターフェースでは self 引数は不要 — インターフェースは「呼び出し側位置を除いた関数シグネチャ」のみを定義する

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // インターフェース型を返す、具体実装は自分の型を返す
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

// 関数定義（self は単なる慣習名であってキーワードではない）
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

// 汎用関数（RFC-023 構文：呼び出し時に型引数を省略、自動推論）
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
    // インターフェースの各フィールド（関数フィールド）について
    for (field_name, iface_field) in &iface.fields {
        // 型に同名のメソッドがあるか確認
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換か確認
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

インターフェース型は直接代入をサポートし、コンパイラが代入の右辺の型に応じて最適な呼び出し戦略を自動的に選択する：

```yaoxiang
// 具体型を直接代入 → コンパイル時に具体型を決定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：直接 circle_draw(screen) を呼び出し、vtable なし

// 関数戻り値 → コンパイル時に具体型を決定できない、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable でメソッド検索

// 異種コレクション → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable でメソッド検索
}
```

**コンパイル時最適化戦略**：

| シナリオ                         | 推論結果      | 呼び出し方式               |
| -------------------------------- | ------------- | -------------------------- |
| `d: Drawable = Circle(1)`        | 具体型 Circle | 直接呼び出し（ゼロコスト） |
| `d: Drawable = get_shape()`      | 不明          | vtable                     |
| `shapes: List(Drawable) = [...]` | 異種          | vtable                     |

**ルール**：

1. 右辺が具体型コンストラクタでコンパイル時に決定可能な場合、直接呼び出し IR を生成
2. 右辺の型がコンパイル時に決定できない場合、vtable 機構にフォールバック
3. vtable がフォールバックとしてランタイム多態の正確性を保証

### ダックタイピングサポート

```yaoxiang
// 同じメソッドを持つ限り、インターフェース型に代入可能
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

| 以前                                     | 以後                                                                                         |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)`    | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| `impl` キーワードが必要                  | キーワード不要、インターフェース名は型体内に記述                                             |

### 廃止済み：`|` 変体構文

> **廃止宣言（2026-07-25）**：`|` 変体構文を正式に廃止し、実装から削除。

以下の記法は**もはやサポートされない**：

```
type Color = red | green | blue                # ❌ 廃止
type Result(T, E) = ok(T) | err(E)             # ❌ 廃止
type Option(T) = some(T) | none                # ❌ 廃止
```

和型（sum
type）の表現には記録型を統一使用する。記録型のフィールドがすべて関数であり、かつすべてがその型自身を返す場合、それが和型である：

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

1. **特殊ケースの除去**：`|` は BNF における唯一の `name: type = value`
   形式でない構文。これを取り除くことで、 `type_expr`
   生成規則は完全に統一され、parser は変体型のために独立した経路と先読みバックトラックを維持する必要がなくなる。
2. **数学的等価性**：Curry-Howard 同型の下では、選言 P ⊕
   Q に対応する和型は、「フィールドがすべて自身型を返す関数」である記録型と等価である。両者は同じ意味を表現し、二つの構文は不要。
3. **破壊性ゼロ**：削除前の `|`
   構文は parser で半ばサポートされていた（引数なしの変体は解析可能だが、引数型は単態化時に失われる）ものの、これに依存するユーザーコードは存在しなかった。
4. **AST 簡略化**：`Type::Variant(Vec<VariantDef>)` ノードを削除し、すべての変体型は統一して
   `Type::Struct` 経路を辿る。下流の typecheck/mono/formatter の特殊分岐をすべて除去。

> **注**：和型の意味的属性（変体構築、match 網羅性検査、tagged
> union メモリレイアウト）は typecheck 層が `Type::Struct`
> 構造から推論し、独立した AST ノードに依存しない。変体構築の完全な意味については次節「記録式和型の変体構築（権威ある定義）」を参照。

### 記録式和型の変体構築（権威ある定義）

> 本節は「廃止済み：`|` 変体構文」節の識別基準を完全な意味論として展開する（2026-09-25 確定、issue
> #341
> RFC-011b フェーズ 2 の前提）。四つの決定：型限定呼び出し、零ペイロード変体の関数呼び出し形式、型の自己充足推論、変体名の全か無かの昇格。

#### 判定規則：全か無か

記録型は以下の二条件を満たすとき**和型**と判定される：

1. 型体のすべてのフィールド型が関数型である；
2. 各フィールドの戻り型が型引数（`Self`/汎用型実引数）で置換された後、すべてその記録型自身である。

判定は**全か無か**である：判定が成立すれば、すべての関数フィールドが同時に変体コンストラクタに昇格する；満たさなければその型は通常の記録型であり（関数フィールドは関数値を格納するデータフィールド）、「部分的に変体、部分的にデータ」という混合形態は存在しない。データと変体意味論を同時に携える必要がある場合は、データ記録と和型の二つの型としてモデル化し、宣言をマージしない。

#### 呼び出し形式：型限定、裸名なし

変体コンストラクタの**唯一**の呼び出し形式は型限定呼び出し——型値上で変体メンバーを取得して呼び出す：

```yaoxiang
r1 = Result(Int, String).ok(5)   // Result(Int, String)、ペイロード 5
c  = Color.red()                 // 零ペイロード変体：フィールドシグネチャと一致する関数呼び出し形式
```

**裸名形式は提供されない**（`ok(5)`
はコンストラクタ呼び出しではない）。理由：裸名は文脈の期待型からの逆推に依存して所属和型と型実引を決定するが、本言語の型は明示的に書けなければならず、推論は期待位置から単方向に流れる（RFC-011a「Self は明示的な型引数、魔法なし」と同一規律）。限定名形式では型が完全に自己充足しており、文脈が不要。将来裸名を導入する場合でも、「文脈上唯一に決定可能なときの展開糖」としての限定であり、意味論の基準は本節の限定名。

#### 推論：型の自己充足

限定名が完全な型実引を与えると、変体コンストラクタのシグネチャは**フィールドシグネチャを型実引で置換した**結果となる：

```yaoxiang
Result(Int, String).ok   // : (Int) -> Result(Int, String)
```

ペイロード型検査は通常の関数呼び出しの実引検査（不一致は E1002 報告）であり、新たな推論規則はない。汎用和型のコンストラクタは型のインスタンス化と共に自然に単態化され、別個の仕組みは不要。

#### フィールド昇格：変体名はデータフィールドではない

和型と判定された後、変体名はデータフィールド空間から除去される——和型の**値**に対する変体名フィールドアクセスはコンパイル時に拒否される：

```yaoxiang
r1.ok    // コンパイルエラー：ok は Result の変体コンストラクタであり、データフィールドではない
```

理由：ランタイムの和型値は tagged
union（後述）であり、変体宣言表を持たない；フィールドアクセスを許せば必ず静かに誤訳される。これは RFC-011a
§1.2（フィールド/メソッド統一名前空間、衝突報告）と同じ規律の延長。構築は型限定（`Result.ok`）で行い、取り出しは match 分解（RFC-039）で行う。

#### 実行時表現と等価性

和型値の実行時表現は tagged union である：`Enum { 型アイデンティティ, variant_id, payload }`。

- **型アイデンティティ**は具体和型を運ぶ（グローバルなプレースホルダではない）——異なる和型の値は比較不能；
- **variant_id**
  は変体の型体内での**宣言順**で番号付けされ、この順は同時に RFC-039 網羅性検査の変体集合入力となる；
- **等価性**（`==`/`!=`）：型アイデンティティが同じ、variant_id が同じ、payload が値ごとに等しい（再帰的）。

#### std 移行

`std.result` は本節の機構を用いて `ok` / `err` を定義するように変更され（native な `result_ok` /
`result_err` は退く）、 `std` の変体構築はユーザー和型と同じ規則を辿り、特権通路を持たない。`Result`
/ `Option` の型位置における parser 特例処理と `make_result` の名前特例処理の撤去は
[RFC-011b](./011b-operator-overloading.md) フェーズ 2 が担う——`?` のインターフェース化後、`Result`
は通常の std 和型となる。

### 論理演算子：`and` / `or` / `!`（権威ある定義、Zig 式）

> **定義宣言（2026-08-03）**：論理演算子の権威的形式はキーワード `and` / `or` + 記号単項 `!`
> である（SPEC `syntax.md`
> §2.2 優先度表と一致）。本設計は Zig に整合する：**短絡制御フローはキーワード、純粋な単項演算は記号**。かつて C に漂った実装の
> `&&` / `||` や中間状態のキーワード `not` はすべて削除された。

**意味**：

| 演算子 | 優先度（SPEC §2.2）   | 結合性   | 意味                                 |
| ------ | --------------------- | -------- | ------------------------------------ |
| `!`    | 3（単項前置、密結合） | 右から左 | 論理否定（純粋関数、制御フローなし） |
| `and`  | 10                    | 左から右 | 短絡論理 AND                         |
| `or`   | 10                    | 左から右 | 短絡論理 OR                          |

```yaoxiang
# 短絡評価：and の左辺が false / or の左辺が true のとき、右辺は実行されない
if x != 0 and y / x > 1 { ... }   # x == 0 のときゼロ除算にならない

# 密結合：!a == b ≡ (!a) == b（Zig 式；Python の not a == b ≡ not (a == b) とは逆）
!3 == 4          # false：(!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs))、呼び出し後に否定
```

以下の記法は**もはやサポートされない**（lexer がエラーを報告し、対応する記法を提示）：

```
x && y     # ❌ 削除済み、x and y を使用
x || y     # ❌ 削除済み、x or y を使用
not x      # ❌ 削除済み、!x を使用（not は通常の識別子に戻る；!= には影響なし）
```

**設計理由**（Zig に整合、ziglang/zig#272 / #6625）：

1. **短絡は制御フロー → キーワード；純粋関数は演算 → 記号**。`and` / `or`
   は評価順序を変更し（右辺を必要に応じてスキップ）、`if` と同じ性質を持つためキーワード；`!`
   はすでに評価された被演算子に対して純粋な否定を行い、`-` `+`
   と同じ性質を持つため記号。YaoXiang のエラー伝播は `?`（§2.11）を用い、`!` と衝突しない。
2. **密結合による曖昧性解消**：`!`
   は視覚的に被演算子に「密着」し、高い優先度が一目で分かる；キーワード `not`
   は被演算子との間に空白を強制され、どちらに結合するか（`not a == b`）が脳内で曖昧になりやすい。
3. **曖昧性解消**：`&` は二つの役割——借用トークン（`&p` / `&mut p`、RFC-009）とビット AND（SPEC
   §2.2 優先度 8）——を担う。これに `&&` を加えると一つの記号が三つの意味を担うことになる。`and` /
   `or` / `!` は借用、ビット演算、論理の三つの概念を視覚的に完全に分離する。
4. **先例**：Zig（同じ生態系の現代的なシステム言語）こそ `and` / `or` キーワード + `!`
   記号の組み合わせ；Python / Lua / Ada / SQL は全キーワード（含
   `not`）、C 系統は全記号——YaoXiang は Zig のミックスを採用し、両者の利点を得る。
5. **Curry-Howard との一貫性**：型は命題（前述の同型節参照）であり、 refinied
   type における論理接続は `and` / `or` と書かれる（例えば
   `{ 0 <= idx and idx < arr.len }`）のが命題の自然な表現；`!` は単項否定記号として ¬ に対応する。

> **実装**：`and` / `or`
> は IR 層で短絡ジャンプ列に展開され（`a and b ≡ if a { b } else { false }`）、`!`
> は単項密結合として解析される（被演算子は `BP_UNARY + 1` で取得）。回帰テスト：
> `tests/yaoxiang/01-syntax/basics/logical_ops.yx`、`logical_not.yx`。

## 構文設計説明：名前付き関数は本質的に Lambda の構文糖衣

### 核心的理解

**名前付き関数と Lambda 式は同じものである！**
唯一の違いは：名前付き関数は Lambda に名前を付けただけ。

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

**要点**：シグネチャが引数型を完全に宣言している場合、Lambda 头部の引数名は冗長になり、省略可能。

### 引数スコープ規則

**引数は外側の変数を覆い隠す**：シグネチャ内の引数スコープは関数本体を覆い、内部スコープの優先度がより高い。

```yaoxiang
x = 10  // 外側の変数

double: (x: Int) -> Int = x * 2  // ✅ 引数 x が外側の x を覆う、結果は 20
```

### 注釈位置は柔軟

型注釈は以下のいずれの位置でも可能で、**少なくとも一箇所に注釈すればよい**：

| 注釈位置       | 形式                                     | 説明            |
| -------------- | ---------------------------------------- | --------------- |
| シグネチャのみ | `double: (x: Int) -> Int = x * 2`        | ✅ 推奨         |
| Lambda 頭のみ  | `double = (x: Int) => x * 2`             | ✅ 合法         |
| 両方に注釈     | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャ完全、Lambda 頭省略
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
| **簡潔**   | シグネチャが完全な場合、引数名を書き直す必要なし                |
| **柔軟**   | Lambda 形式を残し、好みの方を使用可能                           |
| **一貫**   | 変数宣言 `x: Int = 42` と統一パターンを維持                     |
| **直感的** | `name: Type = body` が「名前 name、型 Type、値 body」に直接対応 |

## トレードオフ

### 利点

| 利点             | 説明                                           |
| ---------------- | ---------------------------------------------- |
| 極限の統一       | 一つの構文規則ですべてをカバー                 |
| 理論的優美さ     | 完全に対称的な `name: type = value`            |
| 新キーワード不要 | 既存の構文要素を再利用                         |
| 実装が容易       | コンパイラは唯一の宣言形式を処理するだけでよい |
| 学習が容易       | 一つのパターンを覚えれば全コードが書ける       |
| 拡張が容易       | 新機能は自然にこのモデルに溶け込む             |

### 欠点

| 欠点     | 説明                                    |
| -------- | --------------------------------------- |
| 命名規則 | メソッドは `Type.method` 命名に従う必要 |
| 冗長     | 完全構文は簡略構文より長いが、推論可能  |
| 学習曲線 | 統一モデルの理解が必要                  |

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
// IDE が欠落メソッドを自動提案
```

### リスク

| リスク             | 影響                                    | 緩和策                   |
| ------------------ | --------------------------------------- | ------------------------ |
| 解析複雑度         | 統一構文が解析複雑度を増す可能性        | 再帰下降パーサを使用     |
| 性能オーバーヘッド | vtable 検索に追加オーバーヘッドの可能性 | コンパイル時単態化最適化 |

---

## ボーナス 🎮：言語の源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 型の型を定義してみよう……
Type: Type = Type
```

**警告**：これは**名状しがたい**ものなり！

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二、二生三、三生万物。                                   ║
║   易有太极、是生两仪。                                         ║
║                                                              ║
║   Type: Type = Type                                          ║
║   此乃爻象之源、言語之境界。                                   ║
║   コンパイラはここで沈黙し、哲学はここで足をとめる。            ║
║                                                              ║
║   言語の哲学的境界に到達したことを感謝する。                    ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：コンパイラは `Type: Type = Type`
> を正しく処理できない（Type0/Type1 宇宙パラドックスを引き起こす）が、この「ボーナス」を意図的に残している——コンパイルを試みると、言語の創始者からの禅的なメッセージが届く。これは単なる技術的境界ではなく、YaoXiang からの型哲学への敬意である。

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

# 汎用引数：関数型の一部として、例えば (T: Type, R: Type) -> (...)
# 独立した BNF 規則は不要——: Type 引数は通常の関数引数

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

| 用語             | 定義                                                                                             |
| ---------------- | ------------------------------------------------------------------------------------------------ |
| 宣言             | `name: type = value` 形式の代入文                                                                |
| 記録型           | 名前付きフィールドを含む `{ ... }` 型                                                            |
| インターフェース | フィールドがすべて関数型である記録型                                                             |
| 汎用型           | `Name: (T: Type) -> Type = { ... }` として定義され、型引数を受け取る型                           |
| 名前空間関数     | `Type.name` 形式の関数、Type 名前空間に属する。暗黙のバインドを含まない                          |
| メソッドバインド | `Type.name = func[n]`、func の位置 n を呼び出し側にバインドし、`obj.name(args)` 構文を可能にする |
| 汎用関数         | `(T: Type)` 構文を使用する関数、型引数は最初の引数グループとして                                 |
| メタ型           | `Type`、言語における唯一の型階層マーカー                                                         |

---

## ライフサイクルと帰趣

```
┌─────────────┐
│   ドラフト   │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← オープンなコミュニティ議論とフィードバック
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
│ (正式設計)  │    │ (原位置保存) │
└─────────────┘    └─────────────┘
```

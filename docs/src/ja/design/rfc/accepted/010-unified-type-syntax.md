---
title: 'RFC-010: 統一型構文 - name: type = value モデル'
status: '承認済み'
author: '晨煦'
updated: '2026-07-14（Never 内蔵型が実装され、#157 がクローズ）'
issue: '#127'
---

# RFC-010: 統一型構文 - name: type = value モデル

## 要約

本 RFC は極限まで簡素化された統一型構文モデルを提案する：**すべては `name: type = value`**。

YaoXiang にはただ一つの宣言形式しか存在しない：

```
identifier : type = expression
```

ここで `type` は任意の型式であり、`expression` は任意の値式である。**`fn` も `struct` も `trait` も
`impl` も小文字の `type` キーワードも存在しない（ただし `Type` をメタ型キーワードとして持つ）**。

> **核心設計**：`Type` 自体が一つのジェネリック型である。`(T: Type) -> Type`
> は「型パラメータ T を受け取る型」を表す。

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

> **名前空間 vs メソッドバインド**：`Type.name`
> 接頭辞は**名前空間の帰属**を示すだけであり、それ以外の意味はない。`.`
> 呼び出し構文を有効にするには、`p.draw(screen)`
> を明示的にバインドする必要がある：`Point.draw = draw[0]`。詳細は後述の「名前空間とメソッドバインド」セクションを参照。これは型の階層をラベル付けするために使用され、コンパイラが Type0、Type1、Type2... の区別を自動処理し、ユーザーには透過的である。

```yaoxiang
// 核心構文：統一 + 区別

// 変数
x: Int = 42

// 関数（引数名はシグネチャ内）
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

// 使用
p: Point = Point(1.0, 2.0)
p.draw(screen)           // 構文糖衣 → Point.draw(p, screen)
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
- メソッドバインド構文

これらの概念間には統一性が欠如しており、構文の断片化と高い学習コストを招いている。

### 設計目標

1. **極限の統一**：一つの構文ルールですべてをカバー
2. **簡潔でエレガント**：`name: type = value` の対称美
3. **新しいキーワード不要**：既存の構文要素を再利用
4. **理論的なエレガンス**：型自体も Type 型の値
5. **ジェネリック親和性**：ジェネリックシステム（RFC-011）とシームレスに統合

### ジェネリックシステムとの統合

RFC-010 の統一構文モデルと RFC-011 のジェネリックシステム設計は**自然に契合**し、ジェネリックパラメータは統一モデルにシームレスに溶け込む：

```yaoxiang
// 基本ジェネリック（RFC-011 フェーズ 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// ジェネリック関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 フェーズ 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約は引数の型によって運ばれる

// Const ジェネリック（RFC-011 フェーズ 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：

- RFC-011 フェーズ 1（基本ジェネリック）は RFC-010 の**強い依存先**
- 基本ジェネリックなしでは、RFC-010 のジェネリック例はコンパイルできない
- 推奨：RFC-011 フェーズ 1 は RFC-010 と同期して実装する

## 提案

### 核心原則：型コンストラクタ vs 関数/変数

**これは重要な設計選択であり、構文の曖昧さ解消ルールを決定する：**

| 記述                | 意味             | ルール                                       |
| ------------------- | ---------------- | -------------------------------------------- |
| **`x: Type = ...`** | 型コンストラクタ | `: Type` の明示宣言 → 型として強制           |
| **`f = ...`**       | 関数または変数   | `: Type` なし → HM が能動的に関数/変数と推論 |

**なぜこのように設計するのか？**

`{ ... }` 構文自体には曖昧さがある：

- `{ x: Float, y: Float }` は**型リテラル**（レコード型）になり得る
- `{ a = 1 + 1 }` は**コードブロック**（実行文、 Void を返す）になり得る

**曖昧さ解消のルール**：

- **あり** `: Type` → 型コンストラクタとして強制解析、`{ ... }` は型リテラル
- **なし** `: Type` → HM が能動的に `{ ... }` をコードブロックとして解析、関数型と推論

```yaoxiang
# ✅ 型コンストラクタ：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void と推論
main: () -> Void = { println("Hello") }

# ❌ エラー：: Type なし、コンパイラは { ... } を型として解析できない
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
│   └── Point: Type = { x: Float, y: Float }  # 必ず : Type
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必ず : Type
│
├── ジェネリック型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず : Type
│
├── ジェネリック型（複数パラメータ）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず : Type
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示的バインド後にのみ . 呼び出し構文が有効
│
└── ジェネリック関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数と推論
```

### メタ型階層（コンパイラ内部）

**コンパイラ内部**は宇宙階層 `level: selfpointnum`
を維持する（文字列で保存、理論上は無限に拡張可能）。

| Level    | 説明                                  |
| -------- | ------------------------------------- |
| `Type0`  | 日常的な型（`Int`、`Float`、`Point`） |
| `Type1`  | 型コンストラクタ（`List`、`Maybe`）   |
| `Type2+` | 高階コンストラクタ                    |

**ユーザーはこれらの数字を見ることはなく**、`: Type` だけを見る。

### Curry-Howard 同型：型は命题、プログラムは証明

YaoXiang の統一構文 `name: type = value`
は恣意的に選ばれたわけではない——これはまさに Curry-Howard 対応（Curry-Howard
correspondence）の直接的な写像である。この対応は深い事実を明らかにする：**型システムと論理システムは同じものの二つの側面である**。

| 論理（命題）         | 型システム（YaoXiang）              | 例                                   |
| -------------------- | ----------------------------------- | ------------------------------------ |
| 命題 P               | 型 T                                | `Int`、`Bool`                        |
| P が真である証明     | 型 T の一つの値                     | `42: Int`、`true: Bool`              |
| P → Q（含意）        | 関数型 `(P) -> Q`                   | `(x: Int) -> Bool`                   |
| P ∧ Q（連言）        | レコード型 `{ p: P, q: Q }`         | `{ x: Int, y: Bool }`                |
| ∀x.P(x）（全称量化） | ジェネリック関数 `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q（選言）        | enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**`name: type = value` の Curry-Howard における意味**：

```yaoxiang
// "x: Int = 42" は：「Int 型の証明が存在し、名前は x、値は 42 である」と読む
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" は：
// 「含意証明が存在する：Int の証明 a と b が与えられれば、Int の証明を構成できる」と読む
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" は：
// 「Point は命題であり、その証明には Float の証明 x と Float の証明 y の両方が必要である」と読む
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要なのか？**

1. **論理的一貫性 = 型安全性**：もし型システムが型 `T`
   の値の構築を許しながら、合法的な実行時表現を持たないならば、それは論理において偽の命題の証明を許すのと同じである——システムは崩壊する。Curry-Howard は次のように教える：**型安全な言語は本質的に一貫した論理システムである**。

2. **宇宙階層は必要条件である**：下記で詳述するように、もし
   `Type: Type`（すなわち「型の型も型である」）を許せば、ラッセルのパラドックス（型論では Girard のパラドックスとして現れる）を引き起こす。YaoXiang の
   `Type₀ : Type₁ : Type₂ : ...`
   階層構造は、各型が特定の階層にのみ属することを保証し、決して閉じない上昇連鎖を形成し、根本からパラドックスを回避する。これは YaoXiang の型システムが Curry-Howard の意味で**論理的に一貫している**ことを意味する。

3. **統一構文の理論的基礎**：`name: type = value`
   が変数、関数、型、インターフェース、ジェネリックすべてを一つの構文でカバーできるのは、まさにそれらが Curry-Howard のもとで同じこと——**命題に証明を与える**——であるためである。変数は命題の証拠、関数は含意の証拠、レコードは連言の証拠、ジェネリックは全称量化の証拠である。統一構文は人為的に設計された偶然ではなく、Curry-Howard 対応の自然な帰結である。

> **参考文献**：Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM, 58(12),
> 75–84. この記事は Curry-Howard 対応の歴史と意味を平易な言葉で説明している。

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

> **訂正（2026-09-15、RFC-010a）**：本節および下部の「返却ルール」は元々 `return`
> をブロックの値の出口としていたが、RFC-007 の早期リターン意味論と衝突する（詳細は
> [RFC-010a](010a-tail-expression-and-return.md)
> を参照）。以下に修正する：**ブロックの値 = 末尾式、`return` は関数を退出する（型
> `Never`）**。元の「`return`
> で値を返さなければならない」という誤った例は削除され、本訂正には含めない。

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
    }                    // match を末尾式とする
}

// Void 関数：末尾式が Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### 返却ルール

> **訂正（2026-09-15、RFC-010a）**：元のルール「`= { ... }` は `return`
> を使用しなければならず、そうでなければ `Void`
> を返す」およびその理由「『最後の式が返り値かどうか』の曖昧さを排除するために明示的な `return`
> が必要」は**廃止**された。末尾式はその曖昧さを生じず、`return`
> もそれを妨げない（Rust が同型設計をすでに検証済み）。

**ブロックの値 = 末尾式（唯一の出口）**：

| 記述                            | 値                          |
| ------------------------------- | --------------------------- |
| `= expr`（中括弧なし）          | `expr`                      |
| `= { ...; e }`（中括弧あり）    | 末尾式 `e`                  |
| `= { ...; s }`（末尾が文/代入） | `Void`（代入の値は `Void`） |
| `= {}`（空ブロック）            | `Void`                      |

**`return`
の意味論**：非局所的脱出、**最も近い関数境界を退出する**（「ブロックに返す」のではない）、型は
`Never`。`Never <: T` は任意の型に対して成立する（爆発原理）ため、`return`
は任意の返却型の位置に現れ得る。

```yaoxiang
# 単一式：直接値を返す
add: (a: Int, b: Int) -> Int = a + b

# コードブロック：値は末尾式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# 早期リターン：return はブロックを貫通し関数を退出する（型 Never）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # 末尾式
}
```

> **設計理由（訂正 2026-09-15、RFC-010a）**：`{ ... }`
> は依存駆動計算ユニット（後述）であり、その評価意味論は単一式とは異なる——中括弧は複数文のコンテキストを導入し、
> **その値は末尾式で与えられる**。元の「明示的な `return`
> で曖昧さを排除する必要がある」という論証は廃止された：末尾式はその曖昧さを生じない。

#### `{}` の意味論：依存駆動計算ユニット

`{ ... }`
は YaoXiang では単なるコードブロックではない——**依存駆動計算ユニット**である。この意味論は関数本体、変数初期化、`spawn`
の間で一貫している：

**核心ルール**：

- `{}` 内の代入文は記述順序ではなく依存関係に従って自動的に並べ替えられる
- 依存が揃った時点で即座に実行され、欠落している場合はブロックして待機する
- **ブロックの値 = 末尾式**（返却ルール参照）；`return` は `Never`
  型の非局所的退出であり、関数を退出する

```yaoxiang
# 依存駆動：b は a に依存し、コンパイラが自動的に並べ替える
result: Int = {
    b = a + 1      # a に依存 → a の後に自動配置
    a = 10         # 依存なし → 先に実行可能
    b              # 末尾式 → ブロックの値 11
}
```

> **単一式との違い**：`= expr`（中括弧なし）は直接値を返す単純なバインディング；`= { ... }`（中括弧あり）は依存駆動計算コンテキストを導入し、複数文を許可し、その値は末尾式で与えられる。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一の並行プリミティブである。`{}`
の依存駆動意味論を利用して自動並列化を実現する：

- `spawn { ... }` 内の直接の子代入は自動的に並列タスクを生成する
- 依存の揃ったタスクは即座に並行実行される
- 呼び出し側はすべての子タスクの完了をブロックして待機する

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a と依存なし、並行実行）
    c = process(a, b)         # a, b に依存 → 両方の完了を待機
    c                         # 末尾式 → spawn の値
}
// 呼び出し側はここでブロックし、spawn ブロック内すべてのタスクが完了するまで待機
```

> **詳細な定義**：`spawn` の完全な意味論、タスク生成ルール、ブロックモデルについては
> `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型の定義と生ポインタの操作に使用される。`{}`
の評価意味論を利用して、型定義を上位スコープに引き渡し（値の出口は末尾式）：

**核心ルール**：

- `unsafe {}` 内で型を定義し、生ポインタを操作できる
- **末尾式**が `unsafe {}` の値を与える（型定義は上位スコープに引き渡される）
- 返却された型は `unsafe {}` の外で使用可能
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

> **詳細な定義**：`unsafe` の完全な意味論、FFI 型定義、メソッドバインドについては `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang の統一構文の核心であり、フィールド、デフォルト値、バインドメソッド、インターフェース実装を含む：

##### 基本型

**レコード型**：フィールドリスト。フィールドの型は任意の型式を取り得る。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**：フィールドはデフォルト値を持つことができ、構築時に任意指定。

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

**デフォルト値を持たないフィールド**：構築時に必ず提供しなければならない。

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

1. **キーワード**（parser の独立トークン）— 制御構造と宣言キーワード、例：`if`、`match`、`pub`、`return`
2. **リテラル予約語**（parser の独立トークン）—
   `true`、`false`、`void`、`Type`、通常の識別子には使用不可
3. **内蔵型名**（type
   checker にあらかじめ登録）— パーサは通常の識別子として扱い、型チェッカが解析を担当。**予約語ではない、シャドウ可能（非推奨）**

`void`（小文字、リテラル予約語）と `Void`（大文字、内蔵型名）の違い：`void`
は値リテラル（Unit の唯一の値と等しい）であり、`Void`
は型名（Unit 型と等しい、論理 ⊤）である。`let x: Void = void` は合法である。

組み込み内蔵型名：

| 型       | 論理対応     | 説明                                                                                                                                                                                                                                                                   |
| -------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、この型に居留できる値は存在しない。「不可能」を表す——発散、panic、デッドコード。`Never <: T` は任意の `T` に対して成立する（爆発原理）。関数が `Never` を返すとは、決して正常に return しないことを意味する。**キーワードではなく内蔵型名である。** |
| `Void`   | ⊤（真/Unit） | ちょうど一つの居留者（デフォルトの void 値）。`x: Void = <デフォルト>` は合法。和型の単位元が積型の単位元に対応する——`Void` はゼロフィールド積型（Unit）、`Never` はゼロ変体和型。                                                                                     |
| `Int`    | —            | 符号付き整数                                                                                                                                                                                                                                                           |
| `Float`  | —            | 浮動小数点数                                                                                                                                                                                                                                                           |
| `Bool`   | —            | ブール値：`true` / `false`                                                                                                                                                                                                                                             |
| `Char`   | —            | Unicode 文字                                                                                                                                                                                                                                                           |
| `String` | —            | 文字列                                                                                                                                                                                                                                                                 |

##### メソッドのバインド

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
        (dx * dx + dy * dy).sqrt()      # 末尾式
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

**インターフェース = フィールドがすべて関数であるレコード型**

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
接頭辞は名前空間の帰属を示す**だけであり、それ以外の意味はない。いかなる暗黙的バインドも引き起こさない。

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
> と書いても効果はまったく同じである。コンパイラは引数名を見るのではなく、型を見る。

##### メソッドバインド（唯一の方法）

`p.draw(screen)` のような `.`
メソッド呼び出し構文を有効にするには、**明示的なバインドが必要**である。 `[position]`
構文は関数を「メソッド」としてバインドする唯一の機構である（詳細な構文は RFC-004 を参照）。

```yaoxiang
// 関数の定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示的バインド — これ以降のみ p.draw(screen) 構文が利用可能
Point.draw = draw[0]   // 位置 0 の引数（&Point）は呼び出し側が埋める

// 使用
p.draw(screen)          // 構文糖衣 → draw(&p, screen)
Point.draw(p, screen)   // 二つの呼び出し方は等価

// [0] を書かない = バインドしない。Point.draw は通常の関数別名であり、. 構文はない
Point.draw = draw       // バインドしない：Point.draw(p, screen) のみ可能
```

**デフォルト動作**：`[n]`
を書かない = どの引数もバインドしない。ユーザーは呼び出し側が埋める引数を明示的に決定しなければならない。

**複数位置バインド**：

```yaoxiang
// 複数の位置をバインド（自動カリー化）
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
    item.serialize()
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

// 具体的なインスタンス化（RFC-023 構文）
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

// ジェネリックメソッド（RFC-023 構文：型パラメータは呼び出し側で自動推論）
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

ジェネリック型とジェネリック関数の呼び出しは統一して `()` 構文を使用する。`[]`
はいかなるジェネリックコンテキストでも使用されない。

**核心ルール**：

1. **`()` ですべての適用を行う**：型適用、関数呼び出し、値構築はすべて `()`

```yaoxiang
# 型注釈
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から来る
empty: List(Int) = List()

# ジェネリック関数呼び出し——型は引数から自動的に流れる
strings = map(numbers, f)
// T=Int は numbers: List(Int) から来る
// R=String は f: (Int) -> String から来る
```

2. **Type は左、値は右**：`name: type = value`——Type パラメータは左側で宣言され、右側は常に具体的な値。空コンテナ
   `List()` の `T` は左側の型注釈から取得しなければならない。

3. **型情報は一度書けばよい**——引数宣言時に、コンパイラがそれを運ぶ：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左に一度だけ書く
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int、R=String は numbers と f の型から自動取得
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

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` →
> `List(1,2,3)`。古い `[]` ジェネリック構文は完全に削除された。`[]`
> は配列/リストリテラルとインデックスアクセスのみに使用される。

### 例

#### 完全な例

```yaoxiang
// ======== 1. インターフェース定義 ========
// インターフェース = フィールドがすべて関数型のレコード型
// インターフェースには self 引数は不要 — インターフェースは「呼び出し側位置を除いた関数シグネチャ」のみを定義する

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

// ======== 3. メソッド実装（通常関数 + 明示的バインド）========

// 関数の定義（self は単なる慣習名であり、キーワードではない）
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

// 明示的バインド — バインド後にのみ . 呼び出し構文が有効
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
    // インターフェースの各フィールド（関数フィールド）に対して
    for (field_name, iface_field) in &iface.fields {
        // 型に同名のメソッドがあるかチェック
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換かチェック
            // インターフェースフィールド: (Surface) -> Void
            // メソッドシグネチャ: (Point, Surface) -> Void
            // 比較：self 引数を除去した後が一致する必要がある
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

| シナリオ                         | 推論結果      | 呼び出し方式                       |
| -------------------------------- | ------------- | ---------------------------------- |
| `d: Drawable = Circle(1)`        | 具体型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()`      | 不明          | vtable                             |
| `shapes: List(Drawable) = [...]` | 異種          | vtable                             |

**ルール**：

1. 右辺が具体型コンストラクタでコンパイル時に決定可能な場合、直接呼び出し IR を生成する
2. 右辺の型がコンパイル時に決定できない場合、vtable 機構にフォールバックする
3. vtable は実行時多態の正確性を保証するフォールバックである

### ダックタイピングサポート

```yaoxiang
// 同じメソッドを持つ限り、インターフェース型に代入できる
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

> **廃止宣言（2026-07-25）**：`|` バリアント構文は正式に廃止され、実装から削除された。

以下の記述は**もはやサポートされない**：

```
type Color = red | green | blue                # ❌ 廃止
type Result(T, E) = ok(T) | err(E)             # ❌ 廃止
type Option(T) = some(T) | none                # ❌ 廃止
```

統一してレコード型で和型（sum
type）を表現する。レコード型のフィールドがすべて関数であり、すべてがその型自身を返す場合、それは和型である：

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

1. **特殊ケースの排除**：`|` は BNF における唯一の非 `name: type = value`
   形式の構文である。削除後、`type_expr`
   生成規則は完全に統一され、parser はバリアント型のために独立したパスと先読みバックトラックを維持する必要がなくなる。
2. **数学的等価性**：Curry-Howard 対応のもとで、選言 P ⊕
   Q に対応する和型は、「フィールドがすべて自身を返す関数」であるレコード型と等価である。両者は同じ意味を表現し、二つの構文を必要としない。
3. **破壊性ゼロ**：削除前の `|`
   構文は parser で半ばサポートされていた（引数なしバリアントは解析可能だが、引数型は単態化時に失われる）、いかなるユーザーコードもこれに依存していない。
4. **AST 簡素化**：`Type::Variant(Vec<VariantDef>)` ノードを削除、すべてのバリアント型は統一して
   `Type::Struct` 経路を通り、typecheck/mono/formatter 下流の特殊分岐がすべて解消される。

> **注**：和型の意味的属性（match の網羅性チェック、tagged
> union のメモリレイアウト）は typecheck 層が `Type::Struct`
> 構造から導出し、独立した AST ノードに依存しない。

### 論理演算子：`and` / `or` / `!`（権威ある定義、Zig 流）

> **定義宣言（2026-08-03）**：論理演算子の権威ある形式はキーワード `and` / `or` + シンボル単項 `!`
> である（SPEC `syntax.md`
> §2.2 優先度テーブルと一致）。本設計は Zig に対応する：**短絡制御フローにはキーワード、純粋な単項演算にはシンボル**。初期実装で C に漂流した
> `&&` / `||` および中間状態のキーワード `not` はすべて削除された。

**意味論**：

| 演算子 | 優先度（SPEC §2.2）   | 結合性   | 意味                                 |
| ------ | --------------------- | -------- | ------------------------------------ |
| `!`    | 3（単項前置、密結合） | 右から左 | 論理否定（純粋関数、制御フローなし） |
| `and`  | 10                    | 左から右 | 短絡論理 AND                         |
| `or`   | 10                    | 左から右 | 短絡論理 OR                          |

```yaoxiang
# 短絡評価：and の左辺が false / or の左辺が true の場合、右辺は実行されない
if x != 0 and y / x > 1 { ... }   # x == 0 のときゼロ除算にならない

# 密結合：!a == b ≡ (!a) == b（Zig 流；Python の not a == b ≡ not (a == b) とは逆）
!3 == 4          # false：(!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs))、呼び出し後に否定
```

以下の記述は**もはやサポートされない**（lexer がエラーを報告し対応する記述を提示）：

```
x && y     # ❌ 削除済み、x and y を使用
x || y     # ❌ 削除済み、x or y を使用
not x      # ❌ 削除済み、!x を使用（not は通常の識別子に戻る；!= は影響を受けない）
```

**設計理由**（Zig に対応、ziglang/zig#272 / #6625）：

1. **短絡は制御フロー → キーワード；純粋関数は演算 → シンボル**。`and` / `or`
   は評価順序を変更する（右辺は必要に応じてスキップ）、`if` と同じ性質を持つためキーワード；`!`
   は評価済みのオペランドに純粋な否定を行う、`-` `+`
   と同じ性質を持つためシンボル。YaoXiang のエラー伝播は `?`（§2.11）を使用、`!` に競合はない。
2. **密結合による曖昧さの排除**：`!`
   は視覚的にオペランドに「密着」し、高い優先度が一目でわかる；キーワード `not`
   はオペランドとの間にスペースを強要し、どちらに結合するか（`not a == b`）が脳内で曖昧さを生む。
3. **曖昧さの解消**：`&` は二つの役割を担う——借用トークン（`&p` /
   `&mut p`、RFC-009）とビット AND（SPEC §2.2 優先度 8）。さらに `&&`
   を導入すると、一つのシンボルが三つの意味を持つことになる。`and` / `or` / `!`
   により借用、ビット演算、論理という三つの概念を視覚的に完全に分離する。
4. **先例**：Zig（同じ生態系のモダンなシステム言語）はまさに `and` / `or` キーワード + `!`
   シンボルの組み合わせ；Python / Lua / Ada / SQL は全キーワード（`not`
   を含む）、C ファミリーは全シンボル——YaoXiang は Zig の混合方式を採用、両者の利点を得る。
5. **Curry-Howard との一貫性**：型は命題であり（前述の同型節を参照）、精錬型における論理接続は `and`
   / `or` と記述される（例： `{ 0 <= idx and idx < arr.len }`）ことは命題の自然な表現；`!`
   は単項否定のシンボルとして ¬ に対応する。

> **実装**：`and` / `or`
> は IR 層で短絡ジャンプシーケンスに展開される（`a and b ≡ if a { b } else { false }`）、 `!`
> は単項密結合として解析される（オペランドは `BP_UNARY + 1` で取得）。回帰テスト：
> `tests/yaoxiang/01-syntax/basics/logical_ops.yx`、`logical_not.yx`。

## 構文設計の説明：名前付き関数は本質的に Lambda の構文糖衣

### 核心的理解

**名前付き関数と Lambda 式は同じものである！**
唯一の違いは：名前付き関数は Lambda に名前をつけたこと。

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

**要点**：シグネチャが引数型を完全に宣言している場合、Lambda 頭部の引数名は冗長となり、省略可能。

### 引数スコープルール

**引数は外側の変数を覆い隠す**：シグネチャ内の引数スコープは関数本体を覆い隠し、内部スコープの優先度がより高い。

```yaoxiang
x = 10  // 外側の変数

double: (x: Int) -> Int = x * 2  // ✅ 引数 x が外側の x を覆い隠し、結果は 20
```

### 注釈位置は柔軟

型注釈は以下のいずれの位置でも可能、**少なくとも一箇所に注釈すればよい**：

| 注釈位置       | 形式                                     | 説明            |
| -------------- | ---------------------------------------- | --------------- |
| シグネチャのみ | `double: (x: Int) -> Int = x * 2`        | ✅ 推奨         |
| Lambda 頭のみ  | `double = (x: Int) => x * 2`             | ✅ 合法         |
| 両方とも注釈   | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャが完全、Lambda 頭部を省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda 頭で型を注釈
double = (x: Int) => x * 2

// ✅ 合法：両方とも注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の利点

| 特性       | 利点                                                                      |
| ---------- | ------------------------------------------------------------------------- |
| **簡潔性** | シグネチャが完全な場合、引数名を重複して書く必要がない                    |
| **柔軟性** | Lambda 形式を保持、好きな方を使用可能                                     |
| **一貫性** | 変数宣言 `x: Int = 42` との統一パターンを維持                             |
| **直感性** | `name: Type = body` は直接「名前は name、型は Type、値は body」に対応する |

## トレードオフ

### 利点

| 利点                 | 説明                                           |
| -------------------- | ---------------------------------------------- |
| 極限の統一性         | 一つの構文ルールですべてをカバー               |
| 理論的なエレガンス   | 完全に对称的な `name: type = value`            |
| 新しいキーワード不要 | 既存の構文要素を再利用                         |
| 実装が容易           | コンパイラは一つの宣言形式を処理すればよい     |
| 学習が容易           | 一つのパターンを覚えればすべてのコードが書ける |
| 拡張が容易           | 新機能は自然にこのモデルに溶け込む             |

### 欠点

| 欠点     | 説明                                             |
| -------- | ------------------------------------------------ |
| 命名規則 | メソッドは `Type.method` 命名に従う必要がある    |
| 冗長性   | 完全な構文は簡略化された構文より長いが、推論可能 |
| 学習曲線 | 統一モデルを理解する必要がある                   |

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
// IDE が欠落しているメソッドを自動的にヒント表示
```

### リスク

| リスク                       | 影響                                    | 緩和策                   |
| ---------------------------- | --------------------------------------- | ------------------------ |
| 解析の複雑さ                 | 統一構文が解析の複雑さを増す可能性      | 再帰下降パーサを使用     |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドの可能性 | コンパイル時単態化最適化 |

---

## 隠し要素 🎮：言語の源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 型の型を定義しようと試みる...
Type: Type = Type
```

**警告**：これは**名状しがたい**ものである！

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二、二生三、三生万物。                                   ║
║   易有太极、是生两仪。                                          ║
║                                                              ║
║   Type: Type = Type                                          ║
║   爻象之源、言語之境界。                                        ║
║   コンパイラはここで沈黙し、哲学がここで立ち止まる。              ║
║                                                              ║
║   言語の哲学的境界に到達したことを感謝する。                       ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：コンパイラは `Type: Type = Type`
> を正しく処理できない（Type0/Type1 宇宙パラドックスを引き起こす）が、意図的にこの「隠し要素」を残している——コンパイルを試みると、言語の創始者からの禅的なメッセージを受け取る。これは技術的な境界だけでなく、YaoXiang の型哲学への敬意でもある。

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

# ジェネリックパラメータ：関数型の一部として、例：(T: Type, R: Type) -> (...)
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

| 用語             | 定義                                                                                                     |
| ---------------- | -------------------------------------------------------------------------------------------------------- |
| 宣言             | `name: type = value` 形式の代入文                                                                        |
| レコード型       | 名前付きフィールドを含む `{ ... }` 型                                                                    |
| インターフェース | フィールドがすべて関数型であるレコード型                                                                 |
| ジェネリック型   | `Name: (T: Type) -> Type = { ... }` と定義される型、型パラメータを受け取る                               |
| 名前空間関数     | `Type.name` 形式の関数、Type 名前空間に属する。いかなる暗黙的バインドも含意しない                        |
| メソッドバインド | `Type.name = func[n]`、func の位置 n を呼び出し側としてバインドし、`obj.name(args)` 構文を利用可能にする |
| ジェネリック関数 | `(T: Type)` 構文を使用する関数、型パラメータを最初の引数グループとする                                   |
| メタ型           | `Type`、言語における唯一の型階層マーカー                                                                 |

---

## ライフサイクルと帰着

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中 │  ← オープンなコミュニティ議論とフィードバック
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
│ (正式設計)  │    │ (元の場所)  │
└─────────────┘    └─────────────┘
```

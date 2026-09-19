---
title: 'RFC-010: 統一型構文 - name: type = value モデル'
status: '承認済み'
author: '晨煦'
updated: '2026-07-14（Never 内蔵型が実装済み、#157 がクローズ済み）'
issue: '#127'
---

# RFC-010: 統一型構文 - name: type = value モデル

## 概要

本 RFC は、極限まで簡素化された統一型構文モデルを提案する：**すべては `name: type = value`**。

YaoXiang には一つの宣言形式しかない：

```
identifier : type = expression
```

ここで `type` は任意の型式であり、`expression` は任意の値式である。**`fn` も `struct` も `trait` も
`impl` も、小文字の `type` キーワードも存在しない（ただし `Type`
はメタ型キーワードとして存在する）**。

> **核心設計**：`Type` 自体がジェネリック型である。`(T: Type) -> Type`
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

**`Type` は言語内で唯一のメタ型キーワードである**。

> **名前空間 vs メソッドバインディング**：`Type.name`
> 接頭辞は**名前空間の所属**を表すだけであり、それ以外の意味はない。`p.draw(screen)` のような `.`
> 呼び出し構文を有効にするには、`Point.draw = draw[0]`
> のように明示的にバインドする必要がある。詳細は後述の「名前空間とメソッドバインディング」セクションを参照。これは型階層の注釈として使用され、コンパイラが Type0、Type1、Type2... の区別を自動的に処理するため、ユーザーには透過的である。

```yaoxiang
// 核心構文：統一 + 区別

// 変数
x: Int = 42

// 関数（引数名はシグネチャに）
add: (a: Int, b: Int) -> Int = a + b

// レコード型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// インターフェース（本質的にフィールドがすべて関数のレコード型）
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

### なぜこの特性が必要か？

現在の型システムには複数の分離した概念が存在する：

- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インターフェース定義構文
- メソッドバインディング構文

これらの概念間には統一性が欠如しており、構文の断片化と学習コストの増大を招いている。

### 設計目標

1. **極限まで統一**：一つの構文規則ですべてのケースをカバー
2. **簡潔で優雅**：`name: type = value` の対称的な美学
3. **新規キーワード不要**：既存構文要素を再利用
4. **理論的に優雅**：型自体が Type 型の値
5. **ジェネリクスフレンドリー**：ジェネリクスシステム（RFC-011）とシームレスに統合

### ジェネリクスシステムとの統合

RFC-010 の統一構文モデルは、RFC-011 のジェネリクスシステム設計と**自然に調和する**。ジェネリクスパラメータは統一モデルにシームレスに統合可能である：

```yaoxiang
// 基本ジェネリクス（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// ジェネリック関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約は引数型が持つ

// Const ジェネリクス（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：

- RFC-011 Phase 1（基本ジェネリクス）は RFC-010 の**強い依存関係**
- 基本ジェネリクスなしには、RFC-010 のジェネリクス例はコンパイル不可
- 提案：RFC-011 Phase 1 と RFC-010 を同期して実装

## 提案

### 核心原則：型コンストラクタ vs 関数/変数

**これは重要な設計選択であり、構文の曖昧さ排除規則を決定する：**

| 書き方              | 意味             | 規則                                             |
| ------------------- | ---------------- | ------------------------------------------------ |
| **`x: Type = ...`** | 型コンストラクタ | `: Type` の明示的宣言 → 型として強制             |
| **`f = ...`**       | 関数または変数   | `: Type` なし → HM が能動的に関数/変数として推論 |

**なぜこのように設計するか？**

`{ ... }` 構文自体には曖昧さがある：

- `{ x: Float, y: Float }` は**型リテラル**（レコード型）になり得る
- `{ a = 1 + 1 }` は**コードブロック**（実行文、戻り値 Void）になり得る

**曖昧さ排除の規則**：

- **`: Type` あり** → 型コンストラクタとして強制解釈、`{ ... }` は型リテラル
- **`: Type` なし** → HM が能動的に `{ ... }` をコードブロックとして解釈、関数型として推論

```yaoxiang
# ✅ 型コンストラクタ：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void として推論
main: () -> Void = { println("Hello") }

# ❌ エラー：: Type なし、コンパイラが { ... } を型として解釈不可
Point = { x: Float, y: Float }  // HM は関数として推論し、型ではない！
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
│   └── Point: Type = { x: Float, y: Float }  # 必ず戻す：Type
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必ず戻す：Type
│
├── ジェネリック型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず戻す：Type
│
├── ジェネリック型（複数引数）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず戻す：Type
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示的バインド後にのみ . 呼び出し構文が使える
│
└── ジェネリック関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数として推論
```

### メタ型階層（コンパイラ内部）

**コンパイラ内部**は宇宙階層
`level: selfpointnum`（文字列で格納、理論上は無限に拡張可能）を維持する。

| Level    | 説明                                |
| -------- | ----------------------------------- |
| `Type0`  | 日常型（`Int`、`Float`、`Point`）   |
| `Type1`  | 型コンストラクタ（`List`、`Maybe`） |
| `Type2+` | 高階コンストラクタ                  |

**ユーザーはこれらの数字を見ることはなく、`: Type` だけを見る。**

### カリー・ハワード同型：型は命題、プログラムは証明

YaoXiang の統一構文 `name: type = value`
は恣意的に選ばれたわけではない——まさにカリー・ハワード同型（Curry-Howard
correspondence）の直接的な写像である。この同型は、深い事実を明らかにする：**型システムと論理システムは同じものの二つの側面である**。

| 論理（命題）        | 型システム（YaoXiang）              | 例                                   |
| ------------------- | ----------------------------------- | ------------------------------------ |
| 命題 P              | 型 T                                | `Int`、`Bool`                        |
| P が真である証明    | 型 T の値                           | `42: Int`、`true: Bool`              |
| P → Q（含意）       | 関数型 `(P) -> Q`                   | `(x: Int) -> Bool`                   |
| P ∧ Q（連言）       | レコード型 `{ p: P, q: Q }`         | `{ x: Int, y: Bool }`                |
| ∀x.P(x)（全称量化） | ジェネリック関数 `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q（選言）       | 列挙 / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**`name: type = value` のカリー・ハワードにおける意味**：

```yaoxiang
// "x: Int = 42" を読む：「Int 型の証明が存在し、名前は x、値は 42」
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" を読む：
// 「含意証明が存在する：Int の証明 a と b から Int の証明を構成できる」
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" を読む：
// 「Point は命題であり、その証明には Float 証明 x と Float 証明 y の同時提供が必要」
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要か？**

1. **論理的一貫性 = 型安全性**：もし型システムが `T`
   型の値を構築することを許可しつつ、合法的なランタイム表現を持たないならば、論理において偽の命題を証明することを許すようなものである——システムは崩壊する。カリー・ハワードは、**型安全な言語は本質的に一貫した論理システムである**ことを教えてくれる。

2. **宇宙階層は必要条件**：後述するように、もし
   `Type: Type`（つまり「型の型も型である」）を許せば、Russell のパラドックス（型理論では Girard のパラドックスとして現れる）を生む。YaoXiang の
   `Type₀ : Type₁ : Type₂ : ...`
   の階層は、各型が特定の階層にのみ属することを保証し、決して閉じない上昇の連鎖を形成し、根本的にパラドックスを回避する。これは YaoXiang の型システムがカリー・ハワードの意味で**論理的に一貫している**ことを意味する。

3. **統一構文の理論的基盤**：`name: type = value`
   が一つの構文で変数、関数、型、インターフェース、ジェネリクスのすべての概念をカバーできるのは、それらがカリー・ハワードの下で同一のもの——**命題に証明を提供すること**——だからである。変数は命題の証拠、関数は含意の証拠、レコードは連言の証拠、ジェネリクスは全称量化の証拠である。統一構文は人為的に設計された偶然ではなく、カリー・ハワード同型の自然な帰結である。

> **さらなる読み物**：Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM,
> 58(12), 75–84. この記事はカリー・ハワード同型の歴史と意義を平易な言葉で説明している。

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

> **訂正（2026-09-15、RFC-010a）**：本節および下方の「戻り規則」は、ブロックの値出口として `return`
> を使用していたが、これは RFC-007 の早期リターンセマンティクスと衝突する（詳細は
> [RFC-010a](010a-tail-expression-and-return.md)
> を参照）。以下に修正：**ブロックの値 = 末尾式、`return` は関数を退出（型
> `Never`）**。元の「`return`
> で値を返さなければならない」という誤った例は削除し、本訂正には含めない。

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

#### 戻り規則

> **訂正（2026-09-15、RFC-010a）**：元の規則「`= { ... }` では `return`
> を使わなければならず、さもなければ `Void`
> を返す」およびその理由「『最後の式が戻り値かどうか』の曖昧さを排除するため」は**廃止**。末尾式はその曖昧さを生まず、`return`
> もそれを妨げない（Rust が同等の設計をすでに検証済み）。

**ブロックの値 = 末尾式（唯一の出口）**：

| 書き方                          | 値                          |
| ------------------------------- | --------------------------- |
| `= expr`（波括弧なし）          | `expr`                      |
| `= { ...; e }`（波括弧あり）    | 末尾式 `e`                  |
| `= { ...; s }`（末尾が文/代入） | `Void`（代入の値は `Void`） |
| `= {}`（空ブロック）            | `Void`                      |

**`return`
のセマンティクス**：非局所的退出、**最も近い関数境界を退出**（「ブロックに返す」のではない）、型は
`Never`。`Never <: T` は任意の型で成立するため（爆発原理）、`return`
は任意の戻り型の位置に現れ得る。

```yaoxiang
# 単一式：値を直接返す
add: (a: Int, b: Int) -> Int = a + b

# コードブロック：値は末尾式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# 早期リターン：return がブロックを貫通し、関数を退出（型 Never）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # 末尾式
}
```

> **設計理由（訂正 2026-09-15、RFC-010a）**：`{ ... }`
> は依存駆動計算ユニット（後述）であり、その評価セマンティクスは単一式とは異なる——波括弧は複数文のコンテキストを導入し、
> **その値は末尾式によって与えられる**。元の「`return`
> で曖昧さを排除する必要がある」という論証は廃止：末尾式はそのような曖昧さを生まない。

#### `{}` セマンティクス：依存駆動計算ユニット

YaoXiang における `{ ... }`
は単なるコードブロックではなく、**依存駆動計算ユニット**である。このセマンティクスは関数本体、変数初期化、`spawn`
において一貫している：

**核心規則**：

- `{}` 内の代入文は、記述順ではなく依存関係に従って自動ソート
- 依存が満たされれば即座に実行、不足していればブロックして待機
- **ブロックの値 = 末尾式**（戻り規則参照）；`return` は `Never` 型の非局所的退出、関数を退出

```yaoxiang
# 依存駆動：b は a に依存、コンパイラが自動ソート
result: Int = {
    b = a + 1      # a に依存 → a の後に自動配置
    a = 10         # 依存なし → 先に実行可能
    b              # 末尾式 → ブロックの値 11
}
```

> **単一式との違い**：`= expr`（波括弧なし）は値を直接返すシンプルなバインディング；`= { ... }`（波括弧あり）は依存駆動計算コンテキストを導入し、複数文を許可し、その値は末尾式によって与えられる。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一の並列プリミティブである。`{}`
の依存駆動セマンティクスを利用して自動並列化を実現する：

- `spawn { ... }` 内の直接の子代入は自動的に並列タスクを作成
- 依存が満たされたタスクは即座に並列実行
- 呼び出し側はすべての子タスクの完了をブロックして待機

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a と依存なし、並列実行）
    c = process(a, b)         # a, b に依存 → 両方の完了を待機
    c                         # 末尾式 → spawn の値
}
// 呼び出し側はここでブロック、spawn ブロック内のすべてのタスクが完了するまで
```

> **詳細定義**：`spawn` の完全なセマンティクス、タスク作成規則、ブロッキングモデルについては
> `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型の定義と生ポインタの操作に使用される。`{}`
の評価セマンティクスを利用して、型定義を上位スコープに引き渡し、値出口は末尾式となる：

**核心規則**：

- `unsafe {}` 内で型定義と生ポインタの操作が可能
- **末尾式**が `unsafe {}` の値を与える（型定義は上位スコープに引き渡される）
- 返される型は `unsafe {}` 外で使用可能
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

> **詳細定義**：`unsafe` の完全なセマンティクス、FFI 型定義、メソッドバインディングについては
> `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang 統一構文の中核であり、フィールド、デフォルト値、メソッドバインディング、インターフェース実装を含む：

##### 基本型

**レコード型**：フィールドリスト、フィールド型は任意の型式を取り得る。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**：フィールドはデフォルト値を持つことができ、構築時にオプションとなる。

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

使用法：

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

使用法：

```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### 内蔵型

YaoXiang の識別子体系は三層に分かれ、異なるコンパイラ段階で順に認識される：

1. **キーワード**（parser の独立 token）— 制御構造と宣言キーワード、例：`if`、`match`、`pub`、`return`
2. **リテラル予約語**（parser の独立 token）—
   `true`、`false`、`void`、`Type`、通常の識別子には使用不可
3. **内蔵型名**（type
   checker の事前登録）— パーサは通常の識別子として扱い、型検査器が解析を担当。**予約語ではなく、シャドウ可能（非推奨）**

`void`（小文字、リテラル予約語）と `Void`（大文字、内蔵型名）の違い：`void`
は値リテラル（Unit の唯一の値に等しい）、`Void`
は型名（Unit 型に等しい、論理 ⊤）。`let x: Void = void` は合法。

事前定義された内蔵型名：

| 型       | 論理対応     | 説明                                                                                                                                                                                                                  |
| -------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、この型に留まる値はない。「不可能」を表す——発散、panic、デッドコード。`Never <: T` は任意の `T` で成立（爆発原理）。`Never` を返す関数は正常に戻らないことを示す。**キーワードではなく内蔵型名。** |
| `Void`   | ⊤（真/Unit） | ちょうど一つの居留者（デフォルト void 値）。`x: Void = <デフォルト>` は合法。和型の単位元は積型の単位元に対応する——`Void` は零フィールド積型（Unit）、`Never` は零変種和型。                                          |
| `Int`    | —            | 符号付き整数                                                                                                                                                                                                          |
| `Float`  | —            | 浮動小数点数                                                                                                                                                                                                          |
| `Bool`   | —            | ブール値：`true` / `false`                                                                                                                                                                                            |
| `Char`   | —            | Unicode 文字                                                                                                                                                                                                          |
| `String` | —            | 文字列                                                                                                                                                                                                                |

##### メソッドのバインディング

**方法1：型定義体内で直接外部関数をバインド**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置 0 にバインド、カリー化後 method: (b: Point) -> Float
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

**方法2：無名関数 + 位置バインディング**

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

**インターフェース名は型体内に記述、コンパイラが自動的に実装をチェック**

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

##### 名前空間関数定義

**`Type.name`
接頭辞は名前空間の所属を表す**だけであり、それ以外の意味はない。暗黙のバインディングは一切トリガーしない。

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

> **注意**：`self` はキーワードではなく、慣習的な引数名にすぎない。`p`、`this`、`x`
> と書いても完全に同じ効果。コンパイラは引数名を見ず、型を見る。

##### メソッドバインディング（唯一の方法）

`p.draw(screen)` のような `.`
メソッド呼び出し構文を有効にするには、**明示的にバインドする必要がある**。 `[position]`
構文は関数を「メソッド」としてバインドする唯一のメカニズムである（詳細構文は RFC-004 を参照）。

```yaoxiang
// 関数を定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示的にバインド — この後でないと p.draw(screen) 構文は使えない
Point.draw = draw[0]   // 位置 0 の引数（&Point）が呼び出し側によって埋められる

// 使用
p.draw(screen)          // 構文糖衣 → draw(&p, screen)
Point.draw(p, screen)   // 二つの呼び出し方は等価

// [0] を書かない = バインドしない。Point.draw は単なる関数別名、. 構文は使えない
Point.draw = draw       // バインドしない：Point.draw(p, screen) のみ可能
```

**デフォルト動作**：`[n]`
を書かない = どの引数もバインドしない。ユーザは明示的にどの引数が呼び出し側によって埋められるかを決定しなければならない。

**複数位置バインディング**：

```yaoxiang
// 複数位置をバインド（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドを通常関数に変換）：

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
    item.serialize()
}
```

#### 5. ジェネリック型

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
    (self: List(type), item: type) -> {
        self.data.append(item)
        self.length = self.length + 1
    }
}

IntList.push(Int)(self, item)  // 呼び出し例

// ジェネリックメソッド（RFC-023 構文：型引数は呼び出し時に自動推論）
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

**核心規則**：

1. **`()` ですべての適用を行う**：型適用、関数呼び出し、値構築はすべて `()` を使用

```yaoxiang
# 型注釈
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から来る
empty: List(Int) = List()

# ジェネリック関数呼び出し——型は引数から自動フロー
strings = map(numbers, f)
// T=Int は numbers: List(Int) から来る
// R=String は f: (Int) -> String から来る
```

2. **Type が左、値が右**：`name: type = value`——Type 引数は左で宣言され、右は常に具体的な値。空コンテナ
   `List()` の `T` は左の型注釈から取得しなければならない。

3. **型情報は一度だけ書けばよい**——引数宣言時に、コンパイラがそれを運ぶ：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左に一度だけ書く
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String は numbers と f の型から自動
```

4. **値構築は要素から型を推論**：

```yaoxiang
x = List(1, 2, 3)       // List(Int) として推論
y = List("a", "b")      // List(String) として推論
z = List()              // ❌ コンパイルエラー：T を推論できない
z: List(Int) = List()   // ✅ T=Int は左の注釈から来る
```

5. **型エイリアス**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` →
> `List(1,2,3)`。古い `[]` ジェネリック構文は完全に削除。`[]`
> は配列/リストリテラルとインデックスアクセスのみ。

### 例

#### 完全な例

```yaoxiang
// ======== 1. インターフェース定義 ========
// インターフェース = フィールドがすべて関数型のレコード型
// インターフェースは self 引数を必要としない — インターフェースは「呼び出し位置を除いた関数シグネチャ」のみを定義する

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

// ======== 3. メソッド実装（通常の関数 + 明示的バインディング）========

// 関数を定義（self は単なる慣習名、キーワードではない）
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

// 明示的バインディング — バインドして初めて . 呼び出し構文が使える
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

// インスタンスを作成
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// メソッド呼び出し（構文糖衣）
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
            // 比較：self 引数を除いた後、マッチするはず
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

インターフェース型は直接代入をサポートし、コンパイラは代入の右辺の型に応じて最適な呼び出し戦略を自動的に選択する：

```yaoxiang
// 具体型を直接代入 → コンパイル時に具体型を決定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：直接 circle_draw(screen) を呼び出し、vtable なし

// 関数の戻り値 → コンパイル時に具体型を決定不可、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable でメソッドを検索

// 異種コレクション → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable でメソッドを検索
}
```

**コンパイル時最適化戦略**：

| シナリオ                         | 推論結果      | 呼び出し方式                       |
| -------------------------------- | ------------- | ---------------------------------- |
| `d: Drawable = Circle(1)`        | 具体型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()`      | 不明          | vtable                             |
| `shapes: List(Drawable) = [...]` | 異種          | vtable                             |

**規則**：

1. 右辺が具体型コンストラクタでコンパイル時に決定可能な場合、直接呼び出し IR を生成
2. 右辺の型がコンパイル時に決定できない場合、vtable 機構にフォールバック
3. vtable がフォールバックとして実行時多態性の正確性を保証

### ダックタイピングのサポート

```yaoxiang
// 同じメソッドを持つだけで、インターフェース型に代入可能
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
| `impl` キーワードが必要                  | キーワード不要、インターフェース名は型体の後に記述                                           |

### 廃止：`|` バリアント構文

> **廃止宣言（2026-07-25）**：`|` バリアント構文は正式に廃止され、実装から削除された。

以下の書き方は**サポートされなくなった**：

```
type Color = red | green | blue                # ❌ 廃止
type Result(T, E) = ok(T) | err(E)             # ❌ 廃止
type Option(T) = some(T) | none                # ❌ 廃止
```

和型（sum
type）の表現には統一してレコード型を使用。レコード型のフィールドがすべて関数であり、すべてが当該型自身を返す場合、それは和型である：

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

1. **特殊ケースの除去**：`|` は BNF で唯一の `name: type = value`
   形式でない構文である。削除後、`type_expr`
   生成規則は完全に統一され、パーサはバリアント型のために独立したパスと先読み後退を維持する必要がなくなる。
2. **数学的等価性**：カリー・ハワード同型の下、選言 P ⊕
   Q に対応する和型は、「フィールドがすべて自身を返す関数」のレコード型と等価である。両者は同じセマンティクスを表現し、二つの構文は不要。
3. **破壊性ゼロ**：削除前、`|`
   構文はパーサで半サポート（引数なしバリアントは解析可能だが、引数型は単相化時に失われる）であり、ユーザコードは一切依存していない。
4. **AST 簡素化**：`Type::Variant(Vec<VariantDef>)` ノードを削除、すべてのバリアント型は統一して
   `Type::Struct` パスを通り、下流の typecheck/mono/formatter の特殊分岐がすべて除去される。

> **注**：和型のセマンティック属性（match の網羅性チェック、tagged
> union のメモリレイアウト）は typecheck 層で `Type::Struct`
> 構造から推導され、独立した AST ノードに依存しない。

### 論理演算子：`and` / `or` / `!`（権威ある定義、Zig 流）

> **定義宣言（2026-08-03）**：論理演算子の権威的形式はキーワード `and` / `or` + 記号単項 `!` （SPEC
> `syntax.md`
> §2.2 優先度表と一致）。本設計は Zig と整合する：**短絡制御フローはキーワード、純粋単項演算は記号**。初期実装で C に漂流した
> `&&` / `||`、および中間状態のキーワード `not` はすべて削除された。

**セマンティクス**：

| 演算子 | 優先度（SPEC §2.2）   | 結合性   | セマンティクス                       |
| ------ | --------------------- | -------- | ------------------------------------ |
| `!`    | 3（単項前置、密結合） | 右から左 | 論理否定（純粋関数、制御フローなし） |
| `and`  | 10                    | 左から右 | 短絡論理積                           |
| `or`   | 10                    | 左から右 | 短絡論理和                           |

```yaoxiang
# 短絡評価：and の左辺が false / or の左辺が true のとき、右辺は実行されない
if x != 0 and y / x > 1 { ... }   # x == 0 のとき 0 除算しない

# 密結合：!a == b ≡ (!a) == b（Zig 流；Python の not a == b ≡ not (a == b) と逆）
!3 == 4          # false：(!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs))、呼び出し後否定
```

以下の書き方は**サポートされない**（lexer がエラーで対応する書き方を提示）：

```
x && y     # ❌ 削除済み、x and y を使用
x || y     # ❌ 削除済み、x or y を使用
not x      # ❌ 削除済み、!x を使用（not は通常の識別子に戻る；!= は影響なし）
```

**設計理由**（Zig と整合、ziglang/zig#272 / #6625）：

1. **短絡は制御フロー → キーワード；純粋関数は演算 → 記号**。`and` / `or`
   は評価順序を変更し（右辺はオンデマンドでスキップ）、`if` と同性質のためキーワード；`!`
   はすでに評価されたオペランドに対して純粋な否定を行い、`-` `+`
   と同性質のため記号。YaoXiang のエラー伝播は `?`（§2.11）であり、`!` との衝突はない。
2. **密結合が曖昧さを排除**：`!` は視覚的にオペランドに「密着」し、高優先度が明白；キーワード `not`
   はオペランドとの間にスペースを強制され、どちらに結合するか（`not a == b`）が脳内で曖昧さを生む。
3. **曖昧さ排除**：`&` は二つの役割を持つ——借用トークン（`&p` /
   `&mut p`、RFC-009）とビット AND（SPEC §2.2 優先度 8）。さらに `&&`
   を導入すると、一つの記号が三つの意味を持つことになる。`and` / `or` / `!`
   は借用、ビット演算、論理の三つの概念を視覚的に完全に分離する。
4. **先例**：Zig（同生態系の現代的システム言語）はまさに `and` / `or` キーワード + `!`
   記号の組み合わせ；Python / Lua / Ada / SQL は全キーワード（含
   `not`）、C 系は全記号——YaoXiang は Zig の混合を取り、両者の利点を得る。
5. **カリー・ハワード一貫性**：型は命題（前述の同型節を参照）、精錬型における論理結合は `and` / `or`
   で書かれる（例： `{ 0 <= idx and idx < arr.len }`）のが命題の自然な表現；`!`
   は単項否定記号として ¬ に対応する。

> **実装**：`and` / `or`
> は IR 層で短絡ジャンプシーケンスに展開され（`a and b ≡ if a { b } else { false }`）、 `!`
> は単項密結合で解析（オペランドは `BP_UNARY + 1`）。回帰テスト：
> `tests/yaoxiang/01-syntax/basics/logical_ops.yx`、`logical_not.yx`。

## 構文設計の説明：名前付き関数の本質は Lambda の構文糖衣

### 核心的理解

**名前付き関数と Lambda 式は同じものである！**
唯一の違いは、名前付き関数が Lambda に名前を付けたことである。

```yaoxiang
// この二つは本質的に完全に同じ
add: (a: Int, b: Int) -> Int = a + b           // 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全に等価）
```

### 構文糖衣のモデル

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**要点**：シグネチャが完全に引数型を宣言する場合、Lambda 頭部の引数名は冗長になり、省略可能。

### 引数スコープ規則

**引数は外側の変数を覆い隠す**：シグネチャ内の引数のスコープは関数本体を覆い、内部スコープの優先度が高い。

```yaoxiang
x = 10  // 外側の変数

double: (x: Int) -> Int = x * 2  // ✅ 引数 x は外側の x を覆い隠す、結果は 20
```

### 注釈位置の柔軟性

型注釈は以下の任意の場所に置け、**少なくとも一箇所に注釈が必要**：

| 注釈位置       | 形式                                     | 説明            |
| -------------- | ---------------------------------------- | --------------- |
| シグネチャのみ | `double: (x: Int) -> Int = x * 2`        | ✅ 推奨         |
| Lambda 頭のみ  | `double = (x: Int) => x * 2`             | ✅ 合法         |
| 両方           | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャが完全、Lambda 頭は省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda 頭で型を注釈
double = (x: Int) => x * 2

// ✅ 合法：両方に注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の利点

| 特性     | 利点                                                            |
| -------- | --------------------------------------------------------------- |
| **簡潔** | シグネチャが完全なら引数名を重複して書く必要なし                |
| **柔軟** | Lambda 形式も保持、好みのものが使える                           |
| **一貫** | 変数宣言 `x: Int = 42` と統一パターンを維持                     |
| **直感** | `name: Type = body` が直接「名前 name、型 Type、値 body」に対応 |

## トレードオフ

### 利点

| 利点               | 説明                                           |
| ------------------ | ---------------------------------------------- |
| 極限まで統一       | 一つの構文規則ですべてのケースをカバー         |
| 理論的に優雅       | 完全に対称的な `name: type = value`            |
| 新規キーワードなし | 既存構文要素を再利用                           |
| 実装が容易         | コンパイラは一つの宣言形式を処理するだけでよい |
| 学習が容易         | 一つのパターンを覚えればすべてのコードが書ける |
| 拡張が容易         | 新機能がこのモデルに自然に統合可能             |

### 欠点

| 欠点     | 説明                                        |
| -------- | ------------------------------------------- |
| 命名規則 | メソッドは `Type.method` 命名に従う必要あり |
| 冗長     | 完全な構文は簡略構文より長いが、推論可能    |
| 学習曲線 | 統一モデルを理解する必要がある              |

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
// IDE が不足しているメソッドを自動的に提示
```

### リスク

| リスク                       | 影響                                        | 緩和策                   |
| ---------------------------- | ------------------------------------------- | ------------------------ |
| 解析の複雑度                 | 統一構文が解析の複雑度を増す可能性あり      | 再帰下降パーサを使用     |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドの可能性あり | コンパイル時単相化最適化 |

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
║   これこそ YaoXiang の源であり、言語の境界である。              ║
║   コンパイラはここで沈黙し、哲学はここで立ち止まる。             ║
║                                                              ║
║   言語の哲学的境界に到達していただき、ありがとうございます。       ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：コンパイラは `Type: Type = Type`
> を正しく処理できない（Type0/Type1 の宇宙パラドックスを引き起こす）が、この「イースターエッグ」は意図的に残されている——コンパイルを試みると、言語の創始者からの禅的なメッセージが届く。これは単なる技術的境界ではなく、YaoXiang の型哲学へのオマージュである。

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

# ジェネリクスパラメータ：関数型の一部として、 (T: Type, R: Type) -> (...)
# 独立した BNF 規則は不要——: Type 引数は通常の関数引数そのもの

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

| 用語                   | 定義                                                                                         |
| ---------------------- | -------------------------------------------------------------------------------------------- |
| 宣言                   | `name: type = value` 形式の代入文                                                            |
| レコード型             | 名前付きフィールドを含む `{ ... }` 型                                                        |
| インターフェース       | フィールドがすべて関数型のレコード型                                                         |
| ジェネリック型         | `Name: (T: Type) -> Type = { ... }` として定義される型、型引数を受け取る                     |
| 名前空間関数           | `Type.name` 形式の関数、Type 名前空間に属する。暗黙のバインディングを含意しない              |
| メソッドバインディング | `Type.name = func[n]`、func の位置 n を呼び出し側にバインドし、`obj.name(args)` 構文を有効化 |
| ジェネリック関数       | `(T: Type)` 構文を使用する関数、型引数を最初の引数群として持つ                               |
| メタ型                 | `Type`、言語内で唯一の型階層マーク                                                           |

---

## ライフサイクルと帰結

```
┌─────────────┐
│   ドラフト  │  ← 現在の状態
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
│  承認済み   │    │   却下       │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (原状維持)  │
└─────────────┘    └──────┬──────┘
                          │
                          ▼
                    ┌─────────────┐
                    │  削除      │
                    └─────────────┘
```

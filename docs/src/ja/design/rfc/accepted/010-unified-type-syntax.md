---
title: 'RFC-010: 統一型構文 - name: type = value モデル'
status: '採用済み'
author: '晨煦'
updated: '2026-07-14（Never 組み込み型が実装済み、#157 はクローズ済み）'
issue: '#127'
---

# RFC-010: 統一型構文 - name: type = value モデル

## 概要

本 RFC は、極限まで簡潔で統一された型構文モデルを提案する:**すべては `name: type = value`**。

YaoXiang には、宣言形式がただ 1 つだけ存在する:

```
identifier : type = expression
```

ここで `type` は任意の型式(expression)であり、`expression` は任意の値式(expression)である。**`fn` も
`struct` も `trait` も `impl` も小文字の `type` キーワードも存在しない（ただし `Type`
はメタ型キーワードとして存在する）**。

> **核心設計**:`Type` 自体が一つの汎用型である。`(T: Type) -> Type`
> は「型引数 T を受け取る型」を表す。

| 概念             | コード表記                                                                   |
| ---------------- | ---------------------------------------------------------------------------- |
| 変数             | `x: Int = 42`                                                                |
| 関数             | `add: (a: Int, b: Int) -> Int = a + b`                                       |
| 記録型           | `Point: Type = { x: Float, y: Float }`                                       |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }`                               |
| 泛型型           | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                  |
| 泛型型           | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`     |
| メソッド         | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| 泛型関数         | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`    |

**`Type` は言語における唯一のメタ型キーワードである**。

> **名前空間 vs メソッド束縛**:`Type.name`
> 接頭辞は**名前空間归属**を表すだけで、それ以上の意味はない。暗黙の束縛は一切トリガーされない。`p.draw(screen)`
> のような `.`
> 呼び出し構文を有効にするには、明示的な束縛が必要である:`Point.draw = draw[0]`。詳細は後述の「名前空間とメソッド束縛」セクションを参照。これは型階層の注記に用いられ、コンパイラが Type0、Type1、Type2... の区別を自動処理し、ユーザには透過的である。

```yaoxiang
// 核心语法：统一 + 区分

// 变量
x: Int = 42

// 函数（参数名在签名中）
add: (a: Int, b: Int) -> Int = a + b

// 记录类型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// 接口（本质是字段全为函数的记录类型）
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 方法定义（使用 Type.method 语法）
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point(${self.x}, ${self.y})"
}

// 泛型类型（(T: Type) -> Type = 接受类型参数的泛型类型）
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
p.draw(screen)           // 语法糖 → Point.draw(p, screen)
s: Drawable = p           // 结构子类型：Point 实现 Drawable
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## 動機

### なぜこの機能が必要か？

現在の型システムには、分離された複数の概念が存在する:

- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インターフェース定義構文
- メソッド束縛構文

これらの概念の間に統一性が欠如しており、構文の断片化と学習コストの増大を招いている。

### 設計目標

1. **極限の統一性**:あらゆる状況をカバーする単一の構文規則
2. **簡潔さと優雅さ**:`name: type = value` の対称的な美学
3. **新しいキーワード不要**:既存の構文要素を再利用
4. **理論的な優雅さ**:型自体も Type 型の値である
5. **generics との親和性**:generics システム（RFC-011）とのシームレスな統合

### generics システムとの統合

RFC-010 の統一構文モデルは、RFC-011 の generics システム設計と**自然と調和し**、generics 引数は統一モデルにシームレスに溶け込む:

```yaoxiang
// 基础泛型（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// 泛型函数（RFC-023 语法：签名中 Type 位置可省略，调用时自动推断）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 类型约束（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 约束由参数类型携带

// Const泛型（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**:

- RFC-011 Phase 1（基礎 generics）は RFC-010 の**強い依存先**である
- 基礎 generics がなければ、RFC-010 の generics サンプルはコンパイルできない
- 推奨:RFC-011 Phase 1 は RFC-010 と同期して実装する

## 提案

### 核心原則:型構築子 vs 関数/変数

**これは構文の曖昧性解消ルールを決定する、重要な設計上の選択である:**

| 表記                | 意味           | ルール                                           |
| ------------------- | -------------- | ------------------------------------------------ |
| **`x: Type = ...`** | 型構築子       | `: Type` 明示宣言 → 型として強制                 |
| **`f = ...`**       | 関数または変数 | `: Type` なし → HM が能動的に関数/変数として推論 |

**なぜこのような設計なのか？**

`{ ... }` 構文自体には曖昧性がある:

- `{ x: Float, y: Float }` は**型リテラル**（記録型）になり得る
- `{ a = 1 + 1 }` は**コードブロック**（実行文、Void を返す）になり得る

**曖昧性解消のルール**:

- **`: Type` がある** → 型構築子として強制解析、`{ ... }` は型リテラル
- **`: Type` がない** → HM が能動的に `{ ... }` をコードブロックとして解析し、関数型として推論

```yaoxiang
# ✅ 类型构造器：有 : Type
Point: Type = { x: Float, y: Float }

# ✅ 函数：没有 : Type，HM 推断为 () -> Void
main: () -> Void = { println("Hello") }

# ❌ 错误：没有 : Type，编译器无法将 { ... } 解析为类型
Point = { x: Float, y: Float }  // HM 推断为函数，不是类型！
```

---

**統一モデル:identifier : type = expression**

```
├── 变量
│   └── x: Int = 42
│
├── 函数
│   └── add: (a: Int, b: Int) -> Int = a + b  # 无 : Type，HM 推断为函数
│
├── 记录类型
│   └── Point: Type = { x: Float, y: Float }  # 必须返回： Type
│
├── 接口
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必须返回： Type
│
├── 泛型类型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必须返回： Type
│
├── 泛型类型（多参数）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必须返回： Type
│
├── 命名空间函数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 显式绑定后才有点调用语法
│
└── 泛型函数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # 不返回 Type，HM 推断为函数
```

### メタ型階層（コンパイラ内部）

**コンパイラ内部**は宇宙階層
`level: selfpointnum`（文字列で格納、理論上は無限に拡張可能）を維持する。

| Level    | 説明                              |
| -------- | --------------------------------- |
| `Type0`  | 日常型（`Int`、`Float`、`Point`） |
| `Type1`  | 型構築子（`List`、`Maybe`）       |
| `Type2+` | 高階構築子                        |

**ユーザはこれらの数字を見ることはなく**、`: Type` だけを見る。

### Curry-Howard 同型:型は命題、プログラムは証明

YaoXiang の統一構文 `name: type = value`
は恣意的に選ばれたわけではない——これはまさに Curry-Howard 同型（Curry-Howard
correspondence）の直接的な写像である。この同型は、深い事実を明らかにする:**型システムと論理システムは、同じものの二つの側面に過ぎない**。

| 論理（命題）        | 型システム（YaoXiang）      | 例                                   |
| ------------------- | --------------------------- | ------------------------------------ |
| 命題 P              | 型 T                        | `Int`、`Bool`                        |
| P が真である証明    | 型 T の一つの値             | `42: Int`、`true: Bool`              |
| P → Q（含意）       | 関数型 `(P) -> Q`           | `(x: Int) -> Bool`                   |
| P ∧ Q（連言）       | 記録型 `{ p: P, q: Q }`     | `{ x: Int, y: Bool }`                |
| ∀x.P(x)（全称量化） | 泛型関数 `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q（選言）       | 列挙 / タグ付きユニオン     | `Maybe: (T: Type) -> Type = { ... }` |

**`name: type = value` の Curry-Howard における意味**:

```yaoxiang
// "x: Int = 42" 读作："存在一个 Int 类型的证明，名为 x，其值为 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" 读作：
// "存在一个蕴含证明：给定 Int 的证明 a 和 b，可构造 Int 的证明"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" 读作：
// "Point 是一个命题，其证明需要同时提供 Float 证明 x 和 Float 证明 y"
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要か？**

1. **論理的一貫性 = 型安全性**:もし型システムが、型 `T`
   の値の構築を許可するものの、正当なランタイム表現を持たないとしたら、論理において偽命題の証明を許すようなものだ——システムは破綻する。Curry-Howard が教えてくれるのは:**型安全な言語は、本質的に一貫した論理システムである**ということだ。

2. **宇宙階層は必要条件**:後述のように、もし
   `Type: Type`（つまり「型の型も型である」）を許せば、Russell のパラドックス（型理論では Girard のパラドックスとして現れる）を生む。YaoXiang の
   `Type₀ : Type₁ : Type₂ : ...`
   の階層構造は、各型がただ一つの階層に属することを保証し、決して閉じない上昇の連鎖を形成し、根本的にパラドックスを回避する。これは YaoXiang の型システムが Curry-Howard の意味で**論理的に一貫している**ことを意味する。

3. **統一構文の理論的根拠**:`name: type = value`
   がたった一つの構文で、変数、関数、型、インターフェース、generics というすべての概念をカバーできるのは、それらが Curry-Howard の下ではすべて同じ事柄——**命題に対する証明の提供**——であるためだ。変数は命題の証拠、関数は含意の証拠、記録は連言の証拠、generics は全称量化の証拠である。統一構文は人為的に設計された偶然ではなく、Curry-Howard 同型の自然な帰結である。

> **参考文献**:Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM, 58(12),
> 75–84. この記事は、平易な言葉で Curry-Howard 同型の歴史と意義を解説している。

### 構文定義

#### 1. 変数宣言

```yaoxiang
// 基本语法
x: Int = 42
name: String = "Alice"
flag: Bool = true

// 类型推导（可省略）
y = 100  // 推断为 Int
```

#### 2. 関数定義

> **正誤表（2026-09-15、RFC-010a）**:本節および後述の「返却ルール」は元々 `return`
> をブロックの値出口としていたが、RFC-007 の早期リターン意味論と衝突する（詳細は
> [RFC-010a](010a-tail-expression-and-return.md)
> を参照）。以下に修正する:**ブロックの値 = 末尾式、`return` は関数を退出する（型
> `Never`）**。元の「`return`
> を使って値を返す必要がある」という誤った例は削除し、本正誤表には残さない。

```yaoxiang
// 单表达式形式
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// 代码块形式：值是尾表达式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b                    // 尾表达式 → 块的值
}

// 多行代码块
calc: (x: Float, y: Float, op: String) -> Float = {
    match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }                    // match 作尾表达式
}

// Void 函数：尾表达式为 Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### 返却ルール

> **正誤表（2026-09-15、RFC-010a）**:元のルール「`= { ... }` は `return`
> を使う必要があり、さもなければ `Void`
> を返す」およびその理由「『最後の式が返り値かどうか』の曖昧性を解消するために明示的な `return`
> が必要」は**廃止された**。末尾式はそのような曖昧性を生まず、`return`
> もそれを妨げない（Rust が同等の設計をすでに検証済み）。

**ブロックの値 = 末尾式（唯一の出口）**:

| 表記                            | 値                          |
| ------------------------------- | --------------------------- |
| `= expr`（中括弧なし）          | `expr`                      |
| `= { ...; e }`（中括弧あり）    | 末尾式 `e`                  |
| `= { ...; s }`（末尾が文/代入） | `Void`（代入の値は `Void`） |
| `= {}`（空ブロック）            | `Void`                      |

**`return`
の意味**:非局所退出で、**最も近い関数の境界を退出する**（「ブロックに返す」のではない）、型は
`Never`。`Never <: T` は任意の型 `T` に対して成立する（爆発原理）ため、`return`
は任意の返却型の位置に現れ得る。

```yaoxiang
# 单表达式：直接返回值
add: (a: Int, b: Int) -> Int = a + b

# 代码块：值是尾表达式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# 提前返回：return 穿透块，退出函数（类型 Never）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # 尾表达式
}
```

> **設計理由（正誤表 2026-09-15、RFC-010a）**:`{ ... }`
> は依存駆動の計算ユニット（後述）であり、その評価意味論は単一式とは異なる——中括弧は複数文のコンテキストを導入し、**その値は末尾式によって与えられる**。元の「明示的な
> `return` で曖昧性を解消する必要がある」という論証は廃止された:末尾式はそのような曖昧性を生まない。

#### `{}` の意味:依存駆動計算ユニット

`{ ... }`
は YaoXiang において単なるコードブロックではない——それは**依存駆動計算ユニット**である。この意味論は関数本体、変数初期化、`spawn`
において一貫している:

**核心ルール**:

- `{}` 内の代入文は、記述順序ではなく依存関係に従って自動的に並べ替えられる
- 依存が揃えば即座に実行され、欠けていれば待機する
- **ブロックの値 = 末尾式**（返却ルールを参照）;`return` は `Never` 型の非局所退出で、関数を退出する

```yaoxiang
# 依赖驱动：b 依赖 a，编译器自动排序
result: Int = {
    b = a + 1      # 依赖 a → 自动排在 a 之后
    a = 10         # 无依赖 → 可以先执行
    b              # 尾表达式 → 块的值 11
}
```

> **単一式との違い**:`= expr`（中括弧なし）は直接値を返す単純な束縛である;`= { ... }`（中括弧あり）は依存駆動計算コンテキストを導入し、複数文を許し、その値は末尾式によって与えられる。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一の並列プリミティブである。`{}`
の依存駆動意味論を利用して自動並列化を実現する:

- `spawn { ... }` 内の直接の子代入は自動的に並列タスクを生成する
- 依存の揃ったタスクは即座に並列実行される
- 呼び出し側はすべての子タスクの完了を待機してブロックする

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # 任务 1
    b = fetch_data("url2")    # 任务 2（与 a 无依赖，并行执行）
    c = process(a, b)         # 依赖 a, b → 等待两者完成后执行
    return c                  # 块的值（尾表达式出口尚未实现，见 #365）
}
// 调用方在此阻塞，直到 spawn 块内所有任务完成
```

> **詳細定義**:`spawn` の完全な意味論、タスク生成ルール、ブロッキングモデルについては
> `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型の定義と生ポインタの操作に使用される。`{}`
の評価意味論を利用して、型定義を一つ上のスコープに委ねる（値出口は末尾式）:

**核心ルール**:

- `unsafe {}` 内で型を定義し、生ポインタを操作できる
- **末尾式**が `unsafe {}` の値を与える（型定義は一つ上のスコープに渡される）
- 返却された型は `unsafe {}` の外でも使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# 在 unsafe 块中定义不透明类型
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 裸指针
    }
    SqliteDb           # 尾表达式 → unsafe 块的值
}

# SqliteDb 在 unsafe 块外可用
db = sqlite3_open("test.db")

# ❌ 编译错误：handle 字段需要 unsafe 权限
handle = db.handle

# ✅ 通过方法调用
db.close()
```

> **詳細定義**:`unsafe` の完全な意味論、FFI 型定義、メソッド束縛については `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang の統一構文の中核であり、フィールド、デフォルト値、束縛メソッド、インターフェース実装を含む:

##### 基礎型

**記録型**:フィールドのリスト、フィールドの型は任意の型式でよい。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**:フィールドはデフォルト値を持つことができ、構築時にはオプションである。

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

使用:

```yaoxiang
Point() → Point(x=0, y=0)
Point(x=1) → Point(x=1, y=0)
Point(x=1, y=2) → Point(x=1, y=2)
```

**デフォルト値のないフィールド**:構築時に必ず提供しなければならない。

```yaoxiang
Point2: Type = {
    x: Float,
    y: Float
}
```

使用:

```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### 組み込み型

YaoXiang の識別子体系は 3 層に分かれ、それぞれ異なるコンパイラ段階で認識される:

1. **キーワード**（parser 独立トークン）— 制御構造と宣言のキーワード。例:`if`、`match`、`pub`、`return`
2. **リテラル予約語**（parser 独立トークン）—
   `true`、`false`、`void`、`Type`。通常の識別子には使用不可
3. **組み込み型名**（型チェッカーに事前登録）— パーサは通常の識別子として扱い、型チェッカーが解析を担当する。**予約語ではなく、シャドウ可能（非推奨）**

`void`（小文字、リテラル予約語）と `Void`（大文字、組み込み型名）の違い:`void`
は値リテラル（Unit の唯一の値に等しい）、`Void`
は型名（Unit 型に等しい、論理 ⊤）。`let x: Void = void` は合法である。

組み込み型名のプリセット:

| 型       | 論理的対応   | 説明                                                                                                                                                                                                                                           |
| -------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥（偽/空型） | ゼロ構築子。この型に居住する値はない。「不可能」を表す——発散、panic、デッドコード。`Never <: T` は任意の `T` に対して成立する（爆発原理）。関数が `Never` を返すことは、決して正常に返らないことを示す。**キーワードではなく、組み込み型名。** |
| `Void`   | ⊤（真/Unit） | ちょうど 1 つの居住者を持つ（デフォルトは void 値）。`x: Void = <デフォルト>` は合法。和型の単位元は積型の単位元に対応する——`Void` はゼロフィールド積型（Unit）、`Never` はゼロ変体和型。                                                      |
| `Int`    | —            | 符号付き整数                                                                                                                                                                                                                                   |
| `Float`  | —            | 浮動小数点数                                                                                                                                                                                                                                   |
| `Bool`   | —            | ブール値:`true` / `false`                                                                                                                                                                                                                      |
| `Char`   | —            | Unicode 文字                                                                                                                                                                                                                                   |
| `String` | —            | 文字列                                                                                                                                                                                                                                         |

##### メソッドの束縛

**方法 1:型定義体内で外部関数を直接束縛する**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 绑定到位置0，柯里化后 method: (b: Point) -> Float
}
// 调用：p1.distance(p2) → distance(p1, p2)
```

**方法 2:匿名関数 + 位置束縛**

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        (dx * dx + dy * dy).sqrt()      # 尾表达式
    })
}
// 语法：((params) => body)[position]
// 调用：p1.distance(p2) → distance(p1, p2)
```

##### インターフェース実装

**インターフェース名は型体内に記述し、コンパイラが自動的にその実装をチェックする**

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
    Drawable,          // 实现 Drawable 接口
    Serializable      // 实现 Serializable 接口
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

// 空类型/空接口
EmptyType: Type = {}
Empty: Type = {}
```

##### 名前空間関数の定義

**`Type.name`
接頭辞は名前空間归属を表す**だけで、それ以上の意味はない。暗黙の束縛は一切トリガーされない。

```yaoxiang
// 命名空间函数：在 Point 命名空间下的普通函数
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

// 调用：就是普通函数调用
Point.draw(p, screen)
Point.serialize(p)
```

> **注意**:`self` はキーワードではなく、慣例的なパラメータ名に過ぎない。`p`、`this`、`x`
> と書いても効果はまったく同じである。コンパイラはパラメータ名を見ず、型を見る。

##### メソッド束縛（唯一の方法）

`p.draw(screen)` のような `.`
メソッド呼び出し構文を有効にするには、**明示的な束縛が必要**である。`[position]`
構文は、関数を「メソッド」として束縛する唯一の仕組みである（詳細な構文は RFC-004 を参照）。

```yaoxiang
// 定义函数
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 显式绑定 — 这之后才有 p.draw(screen) 语法
Point.draw = draw[0]   // 位置 0 的参数（&Point）由调用者填充

// 使用
p.draw(screen)          // 语法糖 → draw(&p, screen)
Point.draw(p, screen)   // 两种调用方式等价

// 不写 [0] = 不绑定。Point.draw 就是普通函数别名，没有 . 语法
Point.draw = draw       // 不绑定：只能 Point.draw(p, screen)
```

**デフォルトの動作**:`[n]`
を書かない = どのパラメータも束縛しない。どのパラメータを呼び出し側が埋めるかは、ユーザが明示的に決定しなければならない。

**複数位置の束縛**:

```yaoxiang
// 绑定多个位置（自动柯里化）
Point.transform = transform_points[0, 1]
// 调用：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドを通常関数へ）:

```yaoxiang
// 从绑定中取出函数
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. インターフェース合成

```yaoxiang
// 接口组合 = 类型交集
DrawableSerializable: Type = Drawable & Serializable

// 使用交集类型
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    item.serialize()
}
```

#### 5. 泛型型

```yaoxiang
// 基础泛型（RFC-011 Phase 1）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T:Type)-((self: List(T), item: T) -> Void),
    get: (T:Type)->((self: List(T), index: Int) -> Maybe(T))
}

// 具体实例化（RFC-023 语法）
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

IntList.push(Int)(self, item)  // 调用示例

// 泛型方法（RFC-023 语法：类型参数由调用处自动推断）
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

#### 6. 泛型呼び出し構文

泛型型と泛型関数の呼び出しは、統一して `()` 構文を用いる。`[]`
は泛型のいかなるコンテキストでも使用されない。

**核心ルール**:

1. **`()` ですべての適用を行う**:型適用、関数呼び出し、値構築はすべて `()` を使用

```yaoxiang
# 类型标注
numbers: List(Int) = List(1, 2, 3)

# 空容器：T 从左侧来
empty: List(Int) = List()

# 泛型函数调用——类型从参数自动流动
strings = map(numbers, f)
// T=Int 来自 numbers: List(Int)
// R=String 来自 f: (Int) -> String
```

2. **Type は左、値は右**:`name: type = value`——Type 引数は左で宣言され、右は常に具体的な値である。空コンテナ
   `List()` の `T` は左側の型注釈から取得しなければならない。

3. **型情報は一度だけ書けばよい**——引数宣言時に一度書けば、コンパイラがそれを伝播させる:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int 在左边写一次
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String 自动从 numbers 和 f 的类型来
```

4. **値構築は要素から型を推論する**:

```yaoxiang
x = List(1, 2, 3)       // 推断为 List(Int)
y = List("a", "b")      // 推断为 List(String)
z = List()              // ❌ 编译错误：无法推断 T
z: List(Int) = List()   // ✅ T=Int 来自左侧注解
```

5. **型エイリアス**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**:`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` →
> `List(1,2,3)`。古い `[]` 泛型構文は完全に削除された。`[]`
> は配列/リストリテラルとインデックスアクセスのみに使用される。

### 例

#### 完全な例

```yaoxiang
// ======== 1. 接口定义 ========
// 接口 = 字段全是函数类型的记录类型
// 接口中不需要 self 参数 — 接口只定义"去掉调用者位置后的函数签名"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // 返回接口类型，具体实现返回自己的类型
    scale: (factor: Float) -> Transformable
}

// ======== 2. 类型定义 ========

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

// ======== 3. 方法实现（普通函数 + 显式绑定）========

// 定义函数（self 只是约定名，不是关键字）
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

// 显式绑定 — 绑定后才有点调用语法
Point.draw = draw[0]
Point.bounding_box = bounding_box[0]
Point.serialize = serialize[0]
Point.translate = translate[0]
Point.scale = scale[0]
Point.distance = distance[0]

// Rect 的方法也类似
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

// 创建实例
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// 方法调用（语法糖）
p.draw(screen)
r.draw(screen)

// 普通方法调用（直接调用）
d: Float = distance(p, Point(0.0, 0.0))

// 链式调用
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// 接口赋值
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// 泛型函数（RFC-023 语法：调用时省略类型参数，自动推断）
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
    // 对于接口的每个字段（函数字段）
    for (field_name, iface_field) in &iface.fields {
        // 检查类型是否有同名方法
        if let Some(method) = typ.methods.get(field_name) {
            // 检查方法签名是否兼容
            // 接口字段: (Surface) -> Void
            // 方法签名: (Point, Surface) -> Void
            // 比较：去掉 self 参数后应该匹配
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

インターフェース型は直接代入をサポートし、コンパイラは代入の右辺の型に応じて自動的に最適な呼び出し戦略を選択する:

```yaoxiang
// 直接赋值具体类型 → 编译期可确定具体类型，零开销调用
d: Drawable = Circle(1)
d.draw(screen)  // 编译后：直接调用 circle_draw(screen)，无 vtable

// 函数返回值 → 编译期无法确定具体类型，使用 vtable
d: Drawable = get_shape()
d.draw(screen)  // 通过 vtable 查找方法

// 异构集合 → 使用 vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // 通过 vtable 查找方法
}
```

**コンパイル時最適化戦略**:

| シナリオ                         | 推論結果      | 呼び出し方式                       |
| -------------------------------- | ------------- | ---------------------------------- |
| `d: Drawable = Circle(1)`        | 具体型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()`      | 不明          | vtable                             |
| `shapes: List(Drawable) = [...]` | 異種混在      | vtable                             |

**ルール**:

1. 右辺が具体型の構築子でコンパイル時に確定可能な場合、直接呼び出し IR を生成する
2. 右辺の型がコンパイル時に確定できない場合、vtable 機構にフォールバックする
3. vtable はフォールバックとして実行時多態の正確性を保証する

### ダックタイピングのサポート

```yaoxiang
// 只要有相同方法，就可以赋值给接口类型
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

### 構文の変更

| 以前                                     | 以後                                                                                         |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)`    | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| `impl` キーワードが必要                  | キーワード不要、インターフェース名は型体後に記述                                             |

### 廃止:`|` 変体構文

> **廃止宣言（2026-07-25）**:`|` 変体構文は正式に廃止され、実装からも削除された。

以下の表記は**サポートされない**:

```
type Color = red | green | blue                # ❌ 废弃
type Result(T, E) = ok(T) | err(E)             # ❌ 废弃
type Option(T) = some(T) | none                # ❌ 废弃
```

記録型を使用して和型（sum
type）を統一的に表現する。記録型のフィールドがすべて関数であり、かつすべてが当該型自身を返す場合、それは和型である:

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

**設計理由**:

1. **特殊ケースの排除**:`|` は BNF における唯一の非 `name: type = value`
   形式の構文である。削除後、`type_expr`
   の生成規則は完全に統一され、parser は変体型のために独立したパスや先読みのバックトラックを維持する必要がなくなる。

2. **数学的等価性**:Curry-Howard 同型の下では、選言 P ⊕
   Q に対応する和型は、「フィールドがすべて自身を返す関数である」記録型と等価である。両者は同じ意味を表現するため、二つの構文は不要である。

3. **破壊性なし**:削除前、`|`
   構文は parser で半ばサポートされていた（引数なしの変体は解析可能だが、引数型は単態化時に失われる）ため、依存するユーザコードは存在しない。

4. **AST の簡素化**:`Type::Variant(Vec<VariantDef>)` ノードが削除され、すべての変体型は
   `Type::Struct` パスに統一され、下流の typecheck/mono/formatter の特殊分岐がすべて排除される。

> **注**:和型の意味的属性（match の網羅性チェック、タグ付きユニオンのメモリレイアウトなど）は、typecheck 層が
> `Type::Struct` 構造から推論し、独立した AST ノードには依存しない。

### 論理演算子:`and` / `or` / `!`（権威ある定義、Zig 式）

> **定義宣言（2026-08-03）**:論理演算子の権威ある形式は、キーワード `and` / `or` + 記号単項 `!`
> である（SPEC `syntax.md`
> §2.2 の優先順位表と一致）。本設計は Zig に対応する:**短絡制御フローはキーワード、純粋な単項演算は記号**。初期実装で C に漂流していた
> `&&` / `||`、および中間状態にあったキーワード `not` はすべて削除された。

**意味**:

| 演算子 | 優先度（SPEC §2.2）     | 結合性   | 意味                                 |
| ------ | ----------------------- | -------- | ------------------------------------ |
| `!`    | 3（単項接頭辞、強結合） | 右から左 | 論理否定（純粋関数、制御フローなし） |
| `and`  | 10                      | 左から右 | 短絡論理積                           |
| `or`   | 10                      | 左から右 | 短絡論理和                           |

```yaoxiang
# 短路求值：and 左侧为 false / or 左侧为 true 时，右侧不执行
if x != 0 and y / x > 1 { ... }   # x == 0 时不会除零

# 紧绑定：!a == b ≡ (!a) == b（Zig 式；与 Python 的 not a == b ≡ not (a == b) 相反）
!3 == 4          # false：(!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs))，调用后取反
```

以下の表記は**サポートされない**（lexer がエラーを報告し、対応する表記を提示する）:

```
x && y     # ❌ 已移除，用 x and y
x || y     # ❌ 已移除，用 x or y
not x      # ❌ 已移除，用 !x（not 恢复为普通标识符；!= 不受影响）
```

**設計理由**（Zig に合わせる、ziglang/zig#272 / #6625）:

1. **短絡は制御フロー → キーワード;純粋関数は演算 → 記号**。`and` / `or`
   は評価順序を変更し（必要に応じて右辺をスキップ）、`if` と同じ性質を持つためキーワード;`!`
   は評価済みの被演算子に対して純粋な否定を行い、`-` `+`
   と同じ性質を持つため記号。YaoXiang のエラー伝播は `?`（§2.11）であり、`!` と衝突しない。

2. **強結合による曖昧性解消**:`!`
   は視覚的に「被演算子に密着」し、高い優先順位が一目でわかる;キーワード `not`
   は被演算子との間にスペースを強制され、どちらに結合するか（`not a == b`）が脳内で曖昧になりやすい。

3. **曖昧性解消**:`&` は二つの役割を担う——借用トークン（`&p` / `&mut p`、RFC-009）とビット AND（SPEC
   §2.2 優先度 8）。さらに `&&` を導入すると、一つの記号が三つの意味を持つことになる。`and` / `or` /
   `!` により借用、ビット演算、論理という三つの概念が視覚的に完全に分離される。

4. **先例**:Zig（同じエコシステムの現代的なシステム言語）はまさに `and` / `or` キーワード + `!`
   記号の組み合わせ;Python / Lua / Ada / SQL は全キーワード（`not`
   を含む）、C 系統は全記号——YaoXiang は Zig のミックスを採用し、両者の利点を得る。

5. **Curry-Howard との一貫性**:型は命題であり（前出の同型セクションを参照）、精錬型における論理結合を
   `and` / `or`（例:`{ 0 <= idx and idx < arr.len }`）と書くと命題の自然な表現になる;`!`
   は単項否定記号として ¬ に対応する。

> **実装**:`and` / `or`
> は IR 層で短絡ジャンプ列（`a and b ≡ if a { b } else { false }`）に展開され、`!`
> は単項強結合で解析される（被演算子は `BP_UNARY + 1`
> で取得）。回帰テスト:`tests/yaoxiang/01-syntax/basics/logical_ops.yx`、`logical_not.yx`。

## 構文設計の説明:名前付き関数は本質的に Lambda の構文糖衣

### 核心となる理解

**名前付き関数と Lambda 式は同じものである!**
唯一の違いは:名前付き関数は Lambda に名前をつけた、ということ。

```yaoxiang
// 这两者本质完全相同
add: (a: Int, b: Int) -> Int = a + b           // 具名函数（推荐）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全等价）
```

### 構文糖衣モデル

```
// 具名函数 = Lambda + 名字
name: (Params) -> ReturnType = body

// 本质上是
name: (Params) -> ReturnType = (params) => body
```

**要点**:シグネチャがパラメータ型を完全に宣言している場合、Lambda 頭部のパラメータ名は冗長となり、省略可能である。

### パラメータスコープルール

**パラメータは外側の変数を覆う**:シグネチャ内のパラメータスコープは関数本体を覆い、内部スコープの優先度が高くなる。

```yaoxiang
x = 10  // 外层变量

double: (x: Int) -> Int = x * 2  // ✅ 参数 x 覆盖外层 x，结果为 20
```

### 注釈位置の柔軟性

型注釈は以下のいずれかの位置に置くことができ、**少なくとも一箇所に注釈すればよい**:

| 注釈位置       | 形式                                     | 説明            |
| -------------- | ---------------------------------------- | --------------- |
| シグネチャのみ | `double: (x: Int) -> Int = x * 2`        | ✅ 推奨         |
| Lambda 頭のみ  | `double = (x: Int) => x * 2`             | ✅ 合法         |
| 両方           | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全な例

```yaoxiang
// ✅ 推荐：签名完整，Lambda 头部省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda 头中标注类型
double = (x: Int) => x * 2

// ✅ 合法：两边都标注
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の利点

| 特性     | 利点                                                            |
| -------- | --------------------------------------------------------------- |
| **簡潔** | シグネチャが完全ならパラメータ名を重複して書く必要がない        |
| **柔軟** | Lambda 形式を残し、好みで選べる                                 |
| **一貫** | 変数宣言 `x: Int = 42` と統一パターンを維持                     |
| **直感** | `name: Type = body` は「名前 name、型 Type、値 body」に直接対応 |

## トレードオフ

### 利点

| 利点               | 説明                                           |
| ------------------ | ---------------------------------------------- |
| 極限の統一性       | 一つの構文規則ですべてをカバー                 |
| 理論的な優雅さ     | 完全に左右対称な `name: type = value`          |
| 新規キーワード不要 | 既存の構文要素を再利用                         |
| 実装が容易         | コンパイラが処理する宣言形式は 1 種類のみ      |
| 学習が容易         | 一つのパターンを覚えればすべてのコードが書ける |
| 拡張が容易         | 新機能が自然にこのモデルに統合できる           |

### 欠点

| 欠点     | 説明                                                |
| -------- | --------------------------------------------------- |
| 命名規則 | メソッドは `Type.method` という命名に従う必要がある |
| 冗長     | 完全な構文は簡略構文より長いが、推論可能            |
| 学習曲線 | 統一モデルを理解する必要がある                      |

### 緩和策

```yaoxiang
// 1. 清晰的错误信息
// 编译错误示例：
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. 类型推导
// 可以省略类型，由编译器推导
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE 提示
// IDE 自动提示缺失的方法
```

### リスク

| リスク                       | 影響                                    | 緩和策                   |
| ---------------------------- | --------------------------------------- | ------------------------ |
| 解析の複雑さ                 | 統一構文が解析の複雑さを増す可能性      | 再帰下降パーサを使用     |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドの可能性 | コンパイル時単態化最適化 |

---

## 隠し要素 🎮:言語の源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 尝试定义类型的类型...
Type: Type = Type
```

**警告**:これは**名状しがたいもの**である!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二，二生三，三生万物。                                   ║
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

> **注**:コンパイラは `Type: Type = Type`
> を正しく処理できない（Type0/Type1 宇宙のパラドックスを引き起こす）が、我々はこの「隠し要素」を意図的に残している——コンパイルを試みると、言語の創始者からの禅のメッセージが届く。これは単なる技術的境界ではなく、YaoXiang の型哲学へのオマージュである。

---

## 付録

### 構文 BNF

```bnf
program ::= statement*

statement ::= declaration | expression

# 统一声明：name: Type = expression
declaration ::= identifier ':' type_expr '=' expression

# 类型表达式
type_expr ::= identifier
       | identifier '(' type_expr (',' type_expr)* ')'      # 类型应用
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # 函数类型
       | '{' type_field* '}'                       # 记录/接口类型
       | 'Type'                                    # 元类型

type_field ::= identifier ':' type_expr
             | identifier                           # 接口约束

# 泛型参数：作为函数类型的一部分，如 (T: Type, R: Type) -> (...)
# 无需独立的 BNF 规则——: Type 参数就是普通函数参数

# 表达式
expression ::= literal
              | identifier
              | identifier '(' expression (',' expression)* ')'  # 函数调用 / 构造器调用
              | '(' expression (',' expression)* ')'              # 元组
              | expression '.' identifier '(' arguments? ')'    # 方法调用
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
| 泛型型           | `Name: (T: Type) -> Type = { ... }` と定義される型で、型引数を受け取る                           |
| 名前空間関数     | `Type.name` 形式の関数で、Type 名前空間に属する。いかなる束縛も含意しない                        |
| メソッド束縛     | `Type.name = func[n]`。func の位置 n を呼び出し側に束縛し、`obj.name(args)` 構文を利用可能にする |
| 泛型関数         | `(T: Type)` 構文を使用する関数で、型引数を第一引数群とする                                       |
| メタ型           | `Type`。言語における唯一の型階層マーカー                                                         |

---

## ライフサイクルと帰着

```
┌─────────────┐
│   草案      │  ← 当前状态
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 开放社区讨论和反馈
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```

---
title: 'RFC-010: 統一型構文 - name: type = value モデル'
status: '受領済み'
author: '晨煦'
updated: '2026-07-14（Never 内建型実装済み、#157 クローズ済み）'
issue: '#127'
---

# RFC-010: 統一型構文 - name: type = value モデル

## 摘要

本 RFC は極めて簡潔な統一型構文モデルを提案する：**すべてが `name: type = value`** である。

YaoXiang には1種類の宣言形式のみ存在する：

```
identifier : type = expression
```

ここで `type` は任意の型式、`expression` は任意の値式である。**`fn` も `struct` もなく、`trait` も `impl` もなく、小文字の `type` キーワードもない（しかし `Type` はメタ型キーワードとして存在する）**。

> **核心設計**：`Type` 自体は汎用力型（generic type）である。`(T: Type) -> Type` は「型パラメータ T を受け取る型」を意味する。

| 概念       | コード記述                                                                             |
| ---------- | -------------------------------------------------------------------------------------- |
| 変数       | `x: Int = 42`                                                                          |
| 関数       | `add: (a: Int, b: Int) -> Int = a + b`                                                 |
| 記録型     | `Point: Type = { x: Float, y: Float }`                                                 |
| インタフェース | `Drawable: Type = { draw: (Surface) -> Void }`                                        |
| 汎用力型   | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                           |
| 汎用力型   | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`               |
| メソッド   | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]`          |
| 汎用工関数 | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`             |

**`Type` は言語における唯一のメタ型キーワードである**。

> **名前空間 vs メソッドバインディング**：`Type.name` 接頭辞は**名前空間所属**を意味する。それ以上のものではない。`p.draw(screen)` のような `.` 呼び出し構文を動作させるには、明示的なバインディングが必要である：`Point.draw = draw[0]`。詳細については後述の「名前空間とメソッドバインディング」節を参照されたい。これは型階層を标注するために使用され、コンパイラは Type0、Type1、Type2... の区別を自動的に処理し、ユーザーに透明である。

```yaoxiang
// 核心構文：統一 + 区別

// 変数
x: Int = 42

// 関数（パラメータ名はシグネチャ内に記述）
add: (a: Int, b: Int) -> Int = a + b

// 記録型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// インタフェース（本質的にはフィールドがすべて関数の記録型）
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

// 汎用力型（(T: Type) -> Type = 型パラメータを受け取る汎用力型）
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
s: Drawable = p           // 構造的部分型：Point は Drawable を実装
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## 動機

### なぜこの機能が必要なのか？

現在の型システムは複数の分離した概念を抱えている：

- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インタフェース定義構文
- メソッドバインディング構文

これらの概念間に統一性が欠如しており、構文が断片化しており、学習コストが高い。

### 設計目標

1. **極限の統一性**：1つの構文規則で全ケースをカバー
2. **簡潔で優美**：`name: type = value` の対称的美学
3. **新しいキーワード不要**：既存の構文要素を再利用
4. **理論的に優雅**：型自体も Type 型の値
5. **汎用力友好**：汎用力システム（RFC-011）とシームレス統合

### 汎用力システムとの統合

RFC-010 の統一構文モデルは RFC-011 の汎用力システム設計と**自然に整合**しており、汎用力パラメータは統一モデルにシームレスに統合できる：

```yaoxiang
// 基礎汎用力（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// 汎用工関数（RFC-023 構文：シグネチャ中の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約はパラメータ型が携带

// Const 汎用力（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：

- RFC-011 Phase 1（基礎汎用力）は RFC-010 の**強い依存**
- 基礎汎用力がないと、RFC-010 の汎用力示例はコンパイルできない
- 推奨：RFC-011 Phase 1 と RFC-010 は同時に実装する

## 提案

### 核心原則：型構成子 vs 関数/変数

**これは構文のアンビギュイティ除去規則を決定する重要な設計選択である：**

| 記述             | 意味           | 規則                                     |
| ---------------- | -------------- | ---------------------------------------- |
| **`x: Type = ...`** | 型構成子       | `: Type` が明示的 → 強制的に型           |
| **`f = ...`**     | 関数または変数 | `: Type` なし → HM が関数/変数に推論     |

**なぜこの設計なのか？**

`{ ... }` 構文自体にアンビギュイティがある：

- `{ x: Float, y: Float }` は**型リテラル**（記録型）かもしれない
- `{ a = 1 + 1 }` は**コードブロック**（文を実行し、Void を返す）かもしれない

**アンビギュイティ除去規則**：

- **`: Type` あり** → 強制的に型構成子として解析、`{ ... }` は型リテラル
- **`: Type` なし** → HM が `{ ... }` をコードブロックとして解析し、関数型に推論

```yaoxiang
# ✅ 型構成子：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が () -> Void に推論
main = { println("Hello") }

# ❌ エラー：: Type なし、コンパイラは { ... } を型として解析できない
Point = { x: Float, y: Float }  // HM は関数として推論し、型ではない！
```

---

**統一モデル：identifier : type = expression**

```
├── 変数
│   └── x: Int = 42
│
├── 関数
│   └── add: (a: Int, b: Int) -> Int = a + b  # : Type なし、HM が関数に推論
│
├── 記録型
│   └── Point: Type = { x: Float, y: Float }  # 必ず返り値： Type
│
├── インタフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必ず返り値： Type
│
├── 汎用力型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必ず返り値： Type
│
├── 汎用力型（複数パラメータ）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必ず返り値： Type
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示的バインディング後に . 呼び出し構文が有効
│
└── 汎用工関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数に推論
```

### メタ型レベル（コンパイラ内部）

**コンパイラ内部**では宇宙レベル `level: selfpointnum`（文字列で保存、理論上は無限に延伸可能）を維持する。

| Level    | 説明                                      |
| -------- | ----------------------------------------- |
| `Type0`  | 日常的な型（`Int`、`Float`、`Point`）     |
| `Type1`  | 型構成子（`List`、`Maybe`）               |
| `Type2+` | 高階構成子                                |

**ユーザーはこれらの数字を見ない**。見るのは `: Type` だけである。

### Curry-Howard 同型対応：型は命題、プログラムは証明

YaoXiang の統一構文 `name: type = value` は無作為な選択ではない——これは Curry-Howard 同型対応（Curry-Howard
correspondence）の直接的な写像である。この同型対応は**型システムと論理システムが同じものの両面**であるという深い事実を明らかにする。

| 論理（命題）          | 型システム（YaoXiang）         | 例                                     |
| --------------------- | ------------------------------ | -------------------------------------- |
| 命題 P                | 型 T                           | `Int`、`Bool`                          |
| P が真である証明      | 型 T の1つの値                  | `42: Int`、`true: Bool`                 |
| P → Q（含意）         | 関数型 `(P) -> Q`              | `(x: Int) -> Bool`                     |
| P ∧ Q（合取）         | 記録型 `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                  |
| ∀x.P(x)（全称量化）   | 汎用工関数 `(T: Type) -> ...`  | `map: (T: Type, R: Type) -> ...`       |
| P ⊕ Q（選言）         | enum / tagged union            | `Maybe: (T: Type) -> Type = { ... }`  |

**Curry-Howard における `name: type = value` の意味**：

```yaoxiang
// "x: Int = 42" の読み方：「Int 型の証明が存在し、その名前を x、値が 42」
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" の読み方：
// 「Int の証明 a と b を与えると、Int の証明を構成できることを示す証明が存在する」
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" の読み方：
// 「Point は命題であり、その証明を提供するには Float 証明 x と Float 証明 y の両方が必要である」
Point: Type = { x: Float, y: Float }
```

**なぜこれが重要なのか？**

1. **論理的一貫性 = 型安全性**：型システムが型 `T` の値を許可しつつ合法的な実行時表現がまったくない라면、それは論理において偽の命題の証明を許可するのと同じ——システムは崩壊する。Curry-Howard は教えてくれる：**型安全な言語は本質的に一貫した論理システムである**。

2. **宇宙レベルは必須条件**：以下に詳述するように、`Type: Type`（即ち「型の型も型である」）を許可すると Russell 悖論が発生する（型論では Girard 悖論として現れる）。YaoXiang の
   `Type₀ : Type₁ : Type₂ : ...`
   階層により、各型は某一レベルにのみ属し、永遠に閉じた上昇チェーンを形成し、根本的に悖論を避ける。这意味着 YaoXiang の型システムは Curry-Howard 意味において**論理的に一貫している**。

3. **統一構文の理論的根拠**：`name: type = value` が変数、関数、型、インタフェース、汎用力すべてを1つの構文でカバーできるのは、Curry-Howard においてそれらすべてが同じこと——**命題に証明を提供する**——だからである。変数は命題の証拠、関数は含意の証拠、記録は合取の証拠、汎用力は全称量化の証拠である。統一構文は人為的な設計の偶然ではなく、Curry-Howard 同型対応の自然な帰結である。

> **さらに読むには**：Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM, 58(12),
> 75–84. この記事は Curry-Howard 同型対応のاريخと意義を平易な言葉で説明している。

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

| 記述                     | 返り値                            |
| ------------------------ | --------------------------------- |
| `= expr`（波括弧なし）   | `expr` を直接返す                  |
| `= { ... }`（波括弧あり）| `return` を使用する必要あり、Void を返す |

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

> **設計上の理由**：`{ ... }` は依存駆動計算ユニット（下部参照）であり、その返り値セマンティクスは単一式と異なる。波括弧は複数文脈を導入するため、最後の式が返り値かどうかのアンビギュイティを解除するために明示的な `return` が必要である。

#### `{}` セマンティクス：依存駆動計算ユニット

`{ ... }` は YaoXiang では単なるコードブロックではない——**依存駆動計算ユニット**である。このセマンティクスは関数本体、変数初期化、`spawn` を通じて一貫している：

**核心規則**：

- `{}` 内の代入文は記述順序ではなく依存関係で自動ソートされる
- 依存が揃えば即時実行、欠落すればブロックして待機
- `return` で明示的に返り値を返す（返り値ルール参照）

```yaoxiang
# 依存駆動：b は a に依存、コンパイラが自動ソート
result: Int = {
    b = a + 1      # a に依存 → 自動的に a の後に配置
    a = 10         # 依存なし → 先に実行可能
    return b       # 11 を返す
}
```

> **単一式との違い**：`= expr`（波括弧なし）は値を直接返す単純なバインディング；`= { ... }`（波括弧あり）は依存駆動計算文脈を導入し、複数文と明示的な `return` を許可する。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一の並行プリミティブである。これは `{}` の依存駆動セマンティクスを利用して自動並列化を実現する：

- `spawn { ... }` 内の直接子代入は自動的に並行タスクを生成
- 依存が揃ったタスクは即時並行実行
- 呼び出し元はすべての子タスクの完了をブロックして待機

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a との依存なし、並行実行）
    c = process(a, b)         # a, b に依存 → 両方完了後に実行
    return c
}
// 呼び出し元はここでブロック、spawn ブロック内の全タスク完了まで待機
```

> **詳細定義**：`spawn` の完全セマンティクス、タスク生成規則、ブロッキングモデルについては `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型和裸ポインタの操作を定義するために使用される。これは `{}` の return セマンティクスを利用して型定義を上位スコープに返す：

**核心規則**：

- `unsafe {}` 内で型和裸ポインタの操作を定義可能
- `return` で型定義を上位スコープに返す
- 返された型は `unsafe {}` の外でも使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 裸ポインタ
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

> **詳細定義**：`unsafe` の完全セマンティクス、FFI 型定義、メソッドバインディングについては `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang 統一構文の核心であり、フィールド、デフォルト値、バインディングメソッド、インタフェース実装を含む：

##### 基礎型

**記録型**：フィールドリスト、フィールド型は任意の型式で良い。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値付きフィールド**：フィールドにはデフォルト値を指定でき、構築時に省略可能。

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

**デフォルト値なしフィールド**：構築時に必ず指定する必要がある。

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

##### 内建型

YaoXiang の識別子体系は3層に分かれ、異なるコンパイラ段階で認識される：

1. **キーワード**（パーサ独立トークン）— 制御構造と宣言キーワード、`if`、`match`、`pub`、`return` など
2. **リテラル予約語**（パーサ独立トークン）— `true`、`false`、`void`、`Type`、通常の識別子としては使用不可
3. **内建型名**（型チェッカーが事前登録）— パーサは通常の識別子として扱い、型チェッカーが責任を持って解析。**予約語ではない、上書き可能（非推奨）**

`void`（小文字、リテラル予約語）と `Void`（大文字、内建型名）の違い：`void` は値リテラル（Unit の唯一の値に等しい）、`Void` は型名（Unit 型に等しい、論理 ⊤）。`let x: Void = void` は合法である。

事前設定内建型名：

| 型       | 論理対応           | 説明                                                                                                                                                                                                                         |
| -------- | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Never`  | ⊥（偽/空型）       | 零コンストラクタ、この型に常住できる値はない。「不可能」を意味する——発散、panic、デッドコード。任意の `T` について `Never <: T` が成立（爆発原理）。関数が `Never` を返すとは永久に正常返回しないことを意味する。**キーワードではなく、内建型名である。** |
| `Void`   | ⊤（真/Unit）       | ちょうど1つの常住者（デフォルト void 値）。`x: Void = <デフォルト>` は合法。直和型の単位元に対応積型の単位元——`Void` は零フィールド積型（Unit）、`Never` は零バリアント直和型。                                                 |
| `Int`    | —                  | 符号付き整数                                                                                                                                                                                                                 |
| `Float`  | —                  | 浮動小数点数                                                                                                                                                                                                                 |
| `Bool`   | —                  | 真理値：`true` / `false`                                                                                                                                                                                                     |
| `Char`   | —                  | Unicode 文字                                                                                                                                                                                                                 |
| `String` | —                  | 文字列                                                                                                                                                                                                                       |

##### バインディングメソッド

**方式1：型定義体内で外部関数を直接バインディング**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 位置0にバインディング、カリー化後の method: (b: Point) -> Float
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

##### インタフェース実装

**インタフェース名は型体内記述、コンパイラが自動的に実装をチェック**

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
    Drawable,          // Drawable インタフェースを実装
    Serializable       // Serializable インタフェースを実装
}
```

##### インタフェース定義

**インタフェース = フィールドがすべて関数の記録型**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空型/空インタフェース
EmptyType: Type = {}
Empty: Type = {}
```

##### 名前空間関数定義

**`Type.name` 接頭辞は名前空間所属を意味する**。それ以上のものではない。暗黙的なバインディングをトリガーしない。

```yaoxiang
// 名前空間関数：Point 名前空間内の通常の関数
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

// 呼び出し：単なる関数呼び出し
Point.draw(p, screen)
Point.serialize(p)
```

> **注意**：`self` はキーワードではなく、パラメータ名の慣習的な名前である。`p`、`this`、`x`
> と書いても結果は完全に同じである。コンパイラはパラメータ名を見ず、型を見る。

##### メソッドバインディング（唯一の方法）

`p.draw(screen)` のような `.` メソッド呼び出し構文を動作させるには、**明示的なバインディングが必要である**。 `[position]`
構文は関数を「メソッド」としてバインディングする唯一の方法である（詳細構文は RFC-004 を参照）。

```yaoxiang
// 関数を定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示的バインディング — これにより p.draw(screen) 構文が有効になる
Point.draw = draw[0]   // 位置 0 のパラメータ（&Point）は呼び出し元が埋める

// 使用
p.draw(screen)          // 糖衣構文 → draw(&p, screen)
Point.draw(p, screen)   // 2つの呼び出し方式は同等

// [0] を書かない = バインディングなし。Point.draw は通常の関数エイリアス、. 構文なし
Point.draw = draw       // バインディングなし：Point.draw(p, screen) のみ可能
```

**デフォルト動作**：`[n]` を書かない = すべてのパラメータをバインディングしない。ユーザーはどのパラメータを呼び出し元が埋めるかを明示的に決定する必要がある。

**複数位置バインディング**：

```yaoxiang
// 複数位置をバインディング（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドから通常関数へ）：

```yaoxiang
// バインディングから関数を取り出す
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. インタフェース組合

```yaoxiang
// インタフェース組合 = 型の交差
DrawableSerializable: Type = Drawable & Serializable

// 交差型を使用
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    return item.serialize()
}
```

#### 5. 汎用力型

```yaoxiang
// 基礎汎用力（RFC-011 Phase 1）
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

// 汎用力メソッド（RFC-023 構文：型パラメータは呼び出し箇所で自動推論）
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

#### 6. 汎用力呼び出し構文

汎用力型和汎用工関数の呼び出しはすべて `()` 構文で統一する。`[]` は汎用力文脈では使用しない。

**核心規則**：

1. **`()` ですべてを適用**：型適用、関数呼び出し、値構築はすべて `()` を使用

```yaoxiang
# 型标注
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から来る
empty: List(Int) = List()

# 汎用工関数呼び出し——型はパラメータから自動的に流れる
strings = map(numbers, f)
// T=Int は numbers: List(Int) から
// R=String は f: (Int) -> String から
```

2. **Type は左、値は右**：`name: type = value`——Type パラメータは左側で宣言、右側は常に具体的な値。空コンテナ
   `List()` の `T` は左側の型註釈から取得する必要がある。

3. **型情報は1回だけ記述**——パラメータ宣言時、コンパイラがそれを運ぶ：

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
z: List(Int) = List()   // ✅ T=Int は左側の註釈から
```

5. **型エイリアス**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **旧構文との比較**：`List[Int]` → `List(Int)`、`List[Int]()` → `List()`、`List[Int](1,2,3)` →
> `List(1,2,3)`。旧 `[]` 汎用力構文は完全に削除された。`[]` は配列/リストリテラルとインデックスアクセスのみ使用。

### 示例

#### 完全示例

```yaoxiang
// ======== 1. インタフェース定義 ========
// インタフェース = フィールドがすべて関数型の記録型
// インタフェースでは self パラメータ不要 — インタフェースは「呼び出し元位置除去後の関数シグネチャ」のみを定義

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // インタフェース型を返す、具体的な実装は自身の型を返す
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

// 関数を定義（self は慣習的な名前であり、キーワードではない）
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

// 明示的バインディング — バインディング後に . 呼び出し構文が有効
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

// メソッド呼び出し（糖衣構文）
p.draw(screen)
r.draw(screen)

// 通常のメソッド呼び出し（直接呼び出し）
d: Float = distance(p, Point(0.0, 0.0))

// チェーン呼び出し
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// インタフェース代入
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// 汎用工関数（RFC-023 構文：呼び出し時に型パラメータを省略、自动推論）
process_all: (items: List(T)) -> Void = {
    for item in items {
        print(item.serialize())
    }
}

process_all([p, r])
```

## 詳細設計

### インタフェース検査アルゴリズム

```rust
fn check_type_implements_interface(
    typ: &Type,
    iface: &Type
) -> Result<(), TypeError> {
    // インタフェースの各フィールド（関数フィールド）について
    for (field_name, iface_field) in &iface.fields {
        // 型が同名のメソッドを持つかチェック
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換であるかチェック
            // インタフェースフィールド: (Surface) -> Void
            // メソッドシグネチャ: (Point, Surface) -> Void
            // 比較：self パラメータ除去後は一致すべき
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

### インタフェース直接代入とコンパイル時最適化

インタフェース型は直接代入をサポートし、コンパイラは代入の右辺型に基づいて自動的に最適な呼び出し戦略を選択する：

```yaoxiang
// 具体的な型を直接代入 → コンパイル時に具体的な型を特定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：circle_draw(screen) を直接呼び出し、vtable なし

// 関数返り値 → コンパイル時に具体的な型を特定不可、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable でメソッドを検索

// 異種集合 → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable でメソッドを検索
}
```

**コンパイル時最適化戦略**：

| シナリオ                             | 推論結果        | 呼び出し方式           |
| ------------------------------------ | --------------- | ---------------------- |
| `d: Drawable = Circle(1)`            | 具体的な型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()`          | 不明            | vtable                 |
| `shapes: List(Drawable) = [...]`     | 異種            | vtable                 |

**規則**：

1. 右辺が具体的な型構築子でコンパイル時に特定可能な場合、直接呼び出し IR を生成
2. 右辺の型がコンパイル時に特定できない場合、vtable メカニズムにフォールバック
3. vtable が実行時多態の正確性を保証

### ダックタイピングサポート

```yaoxiang
// 同じメソッドを持っていれば、インタフェース型に代入可能
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

| 以前                                     | 以降                                                                                         |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)`    | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| `impl` キーワードが必要                   | キーワード不要、インタフェース名は型体内記述                                                       |

### 廃止済み：`|` バリアント構文

> **廃止宣言（2026-07-25、issue #203）**：`|` バリアント構文は正式に廃止され実装から削除された。

以下の記述は**もうサポートされない**：

```
type Color = red | green | blue                # ❌ 廃止
type Result(T, E) = ok(T) | err(E)             # ❌ 廃止
type Option(T) = some(T) | none                # ❌ 廃止
```

直和型は記録型で統一して表現する。記録型のフィールドがすべて関数で、かつすべてがその型自体を返す場合、それは直和型である：

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

**設計上の理由**：

1. **特殊ケースの排除**：`|` は BNF における唯一の非 `name: type = value` 形式の構文である。削除後、`type_expr`
   生成規則は完全に統一され、パーサは直和型タイプのために独立したパスと先読みバックトラックを維持する必要がなくなる。
2. **数学的に同等**：Curry-Howard 同型対応において、選言 P ⊕
   Q に対応する直和型は、「フィールドがすべてその型自体を返す関数」である記録型と同等である。両者は同じセマンティクスを表現し、2つの構文は不要。
3. **破壊性ゼロ**：削除前 `|`
   構文はパーサで部分的にサポートされていた（パラメータなしバリアントは解析可能だがパラメータ型は単態化時に丢失）、ユーザーコードの依存はなかった。
4. **AST の簡素化**：`Type::Variant(Vec<VariantDef>)` ノードを削除、すべてのバリアント型は `Type::Struct`
   パスを統一で使用、下流の typecheck/mono/formatter の特殊分岐をすべて排除。

> **注**：直和型のセマンティック属性（match 網羅性检查、tagged union メモリレイアウトなど）は typecheck 層が
> `Type::Struct` 構造から導出し、独立した AST ノードに依存しない。

## 構文設計の説明：名前付き関数は本質的に Lambda の糖衣構文

### 核心的理解

**名前付き関数と Lambda 式は同じものである！** 唯一の違いは：名前付き関数は Lambda に名前を付けていることである。

```yaoxiang
// この2つは本質的に同じ
add: (a: Int, b: Int) -> Int = a + b           // 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全に同等）
```

### 糖衣構文モデル

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**重要な点**：シグネチャがパラメータ型を完全に宣言している場合、Lambda ヘッダーのパラメータ名は冗長になり、省略可能。

### パラメータスコープ規則

**パラメータは外層変数をシャドウする**：シグネチャ内のパラメータスコープは関数体をシャドウし、内側スコープの優先度が高い。

```yaoxiang
x = 10  // 外層変数

double: (x: Int) -> Int = x * 2  // ✅ パラメータ x が外層の x をシャドウ、結果は 20
```

### 标注位置は柔軟

型标注は以下のいずれかの位置に配置可能であり、**少なくとも1箇所に标注すれば良い**：

| 标注位置     | 形式                                     | 説明          |
| ------------ | ---------------------------------------- | ------------- |
| シグネチャのみ | `double: (x: Int) -> Int = x * 2`        | ✅ 推奨       |
| Lambda ヘッダーのみ | `double = (x: Int) => x * 2`             | ✅ 合法       |
| 両方に标注     | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許容 |

### 完全示例

```yaoxiang
// ✅ 推奨：シグネチャ完全、Lambda ヘッダー省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda ヘッダーで型を标注
double = (x: Int) => x * 2

// ✅ 合法：両方に标注
double: (x: Int) -> Int = (x) => x * 2
```

### 設計上の優位性

| 特性       | 優位性                                                          |
| ---------- | ------------------------------------------------------------- |
| **簡潔**   | シグネチャ完全時にパラメータ名の繰り返し記述が不要              |
| **柔軟**   | Lambda 形式を保持、どちらを好むかを選択可能                    |
| **一貫**   | 変数宣言 `x: Int = 42` との統一パターンを維持                  |
| **直観**   | `name: Type = body` は直接的に「名前が name、型が Type、値が body」を対応させる |

## トレードオフ

### 優位性

| 優位性       | 説明                            |
| ------------ | ------------------------------- |
| 極限の統一   | 1つの構文規則で全ケースをカバー  |
| 理論的に優雅 | 完璧に対称な `name: type = value` |
| 新キーワード不要 | 既存の構文要素を再利用          |
| 実装が容易   | コンパイラは1種類の宣言形式のみ処理すれば良い |
| 学習が容易   | 1つのパターンを覚えれば全コードが書ける |
| 拡張が容易   | 新機能は自然にこのモデルに統合できる   |

### 劣位性

| 劣位性     | 説明                           |
| ---------- | ------------------------------ |
| 命名規則   | メソッドは `Type.method` 命名に従う必要がある |
| 冗長       | 完全構文は簡略構文より長いが、推論可能 |
| 学習曲線   | 統一モデルを理解する必要がある               |

### 緩和措置

```yaoxiang
// 1. 明確なエラーメッセージ
// コンパイルエラー示例：
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. 型推論
// 型を省略可能、コンパイラが推論
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE ヒント
// IDE が自動的に欠落しているメソッドをヒント
```

### リスク

| リスク       | 影響                       | 緩和措置           |
| ---------- | -------------------------- | ------------------ |
| 解析複雑度 | 統一構文は解析複雑度を上げる可能性がある | 再帰下降パーサを使用 |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドが発生する可能性がある | コンパイル時単態化最適化 |

---

## 隠し要素 🎮：言語の源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 型の型を定義しようと試みる...
Type: Type = Type
```

**警告**：これは**名状し難い**ものである！

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

> **注**：コンパイラは
> `Type: Type = Type` を正しく処理できない（Type0/Type1 宇宙悖論を引き起こす）が、この「隠し要素」を意図的に残している——コンパイルしようとすると、言語創設者からの禅的メッセージが届く。これは技術の境界であるだけでなく、YaoXiang が型哲学に敬意を表するものである。

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
       | '{' type_field* '}'                       # 記録/インタフェース型
       | 'Type'                                    # メタ型

type_field ::= identifier ':' type_expr
             | identifier                           # インタフェース制約

# 汎用力パラメータ：関数型の一部として、(T: Type, R: Type) -> (...) のように
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

| 用語           | 定義                                                                               |
| -------------- | ---------------------------------------------------------------------------------- |
| 宣言           | `name: type = value` 形式の代入文                                                |
| 記録型         | 名前付きフィールドを含む `{ ... }` 型                                              |
| インタフェース | フィールドがすべて関数型の記録型                                                   |
| 汎用力型       | `Name: (T: Type) -> Type = { ... }` として定義された型、型パラメータを受け取る   |
| 名前空間関数   | `Type.name` 形式の関数、Type 名前空間に属する。暗黙的なバインディングを含まない   |
| メソッドバインディング | `Type.name = func[n]`、func の位置 n を呼び出し元にバインディングし、`obj.name(args)` 構文を有効にする |
| 汎用工関数     | `(T: Type)` 構文を使用した関数、型パラメータは最初の引数グループ                   |
| メタ型         | `Type`、言語における唯一の型レベルマーカー                                        |

---

## ライフサイクルと辿り着く先

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
│  受領済み   │    │  拒否済み   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (元の位置に保存) │
└─────────────┘    └─────────────┘
```

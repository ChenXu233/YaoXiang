```md
---
title: "RFC-010: 統一タイプ構文 - name: type = value モデル"
status: "承認済み"
author: "晨煦"
created: "2025-01-20"
updated: "2026-06-05（返り値ルールと {} 语义を更新）"
---

# RFC-010: 統一タイプ構文 - name: type = value モデル

## 概要

本 RFC は、非常にミニマルな統一的タイプ構文モデルを提案する：**すべてが `name: type = value`**。

YaoXiang には1種類の宣言形式しかない：

```
identifier : type = expression
```

ここで `type` は任意の型式で、`expression` は任意の値式である。
**`fn` もなく、`struct` もなく、`trait` もなく、`impl` もなく、小文字の `type` キーワードもない（ただし、`Type` はメタタイプキーワードとして存在する）**。

> **コア設計**：`Type` はそれ自体がジェネリック型である。`(T: Type) -> Type` は「型パラメータ T を受け取る型」を意味する。

| 概念 | コード表記 | |
|------------|-----------------------------------------------|
| 変数 | `x: Int = 42` | |
| 関数 | `add: (a: Int, b: Int) -> Int = a + b` | |
| 記録型 | `Point: Type = { x: Float, y: Float }` | |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` | |
| ジェネリック型 | `List: (T: Type) -> Type = { data: Array(T), length: Int }` | |
| ジェネリック型 | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` | |
| メソッド | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` | |
| ジェネリック関数 | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` | |

**`Type` は言語内で唯一のメタタイプキーワードである**。

> **名前空間 vs メソッドバインディング**：`Type.name` プレフィックスは**名前空間帰属**を表すに過ぎない。
> これはいかなる暗黙的なバインディングもトリガーしない。`p.draw(screen)` のような `.` 呼び出し構文を動作させるには、
> 明示的にバインディングする必要がある：`Point.draw = draw[0]`。
> 詳細は後述の「名前空間とメソッドバインディング」の節を参照。
> これは型階層を标注するために使用され、コンパイラは Type0、Type1、Type2... の区別を自動的に処理し、ユーザーに透過的である。

```yaoxiang
// コア構文：統一 + 区別

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
s: Drawable = p           // 構造的部分型：Point は Drawable を実装
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## 動機

### この機能が必要な理由

現在の型システムには複数の分離された概念が存在する：

- 変数宣言構文
- 関数定義構文
- 型定義構文（異なる構文）
- インターフェース定義構文
- メソッドバインディング構文

これらの概念間に統一性が欠如しており、構文が断片化されており、学習コストが高い。

### 設計目標

1. **極端な統一**：1つの構文ルールですべての場合をカバー
2. **簡潔で優美**：`name: type = value` の対称的な美感
3. **新しいキーワード不要**：既存の構文要素を再利用
4. **理論的に優雅**：型自体が Type 型の値である
5. **ジェネリックに優しい**：ジェネリックシステム（RFC-011）とシームレスに統合

### ジェネリックシステムとの統合

RFC-010 の統一的構文モデルは RFC-011 のジェネリックシステム設計と**自然に調和**しており、ジェネリックパラメータが統一モデルにシームレスに溶け込む：

```yaoxiang
// 基本ジェネリック（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// ジェネリック関数（RFC-023 構文：シグネチャ内の Type 位置は省略可能、呼び出し時に自動推論）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 型制約（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 制約はパラメータ型が携带

// Constジェネリック（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依存関係**：

- RFC-011 Phase 1（基本ジェネリック）は RFC-010 の**強い依存**
- 基本ジェネリックがないと、RFC-010 のジェネリック例はコンパイルできない
- 推奨：RFC-011 Phase 1 と RFC-010 を同時に実装

## 提案

### 基本原則：型構築子 vs 関数/変数

**これは構文のアンビギュイティ解消ルールを決定する重要な設計選択である：**

| 表記 | 意味 | ルール |
|------|------|--------|
| **`x: Type = ...`** | 型構築子 | `: Type` が明示的に宣言 → 型として強制 |
| **`f = ...`** | 関数または変数 | `: Type` なし → HM が関数/変数として積極的に推論 |

**なぜこの設計なのか？**

`{ ... }` 構文自体にアンビギュイティがある：

- `{ x: Float, y: Float }` は**型リテラル**（記録型）かもしれない
- `{ a = 1 + 1 }` は**コードブロック**（実行文、Void を返す）かもしれない

**アンビギュイティ解消のルール**：

- **`: Type` あり** → 型構築子として強制解析、`{ ... }` は型リテラル
- **`: Type` なし** → HM が `{ ... }` をコードブロックとして積極的に解析し、関数型として推論

```yaoxiang
# ✅ 型構築子：: Type あり
Point: Type = { x: Float, y: Float }

# ✅ 関数：: Type なし、HM が関数として推論
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
│   └── add: (a: Int, b: Int) -> Int = a + b  # : Type なし、HM が関数として推論
│
├── 記録型
│   └── Point: Type = { x: Float, y: Float }  # Type を返さなければならない
│
├── インターフェース
│   └── Drawable: Type = { draw: (Surface) -> Void }  # Type を返さなければならない
│
├── ジェネリック型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # Type を返さなければならない
│
├── ジェネリック型（多パラメータ）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # Type を返さなければならない
│
├── 名前空間関数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 明示的にバインディング后才有点呼び出し構文
│
└── ジェネリック関数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Type を返さない、HM が関数として推論
```

### メタタイプレベル（コンパイラ内部）

**コンパイラ内部**は宇宙レベル `level: selfpointnum` を維持する（文字列で保存、理論上は永久に延長可能）。

| Level | 説明 |
|-------|------|
| `Type0` | 日常の型（`Int`、`Float`、`Point`） |
| `Type1` | 型構築子（`List`、`Maybe`） |
| `Type2+` | 高階構築子 |

**ユーザーはこれらの数字を決して見ない**、`: Type` だけを見る。

> **Curry-Howard 同型**：宇宙レベルの存在はエンジニアリング実装の詳細ではなく、論理的整合性の必要条件である。Curry-Howard 同型は型を命題と同等なものとして扱う。もし `Type: Type`（つまり「型の型も型である」）を許可すれば、「この文は偽である」に似た Russell 悖論的产生につながる——型システムでは Girard 悖論として現れる。YaoXiang の `Type0 / Type1 / Type2…` 分層（つまり Martin-Löf 型理論の累積宇宙）は、各型が特定のレベルに属することを確認し、`Typeₙ : Typeₙ₊₁` が永不に閉じない上昇チェーンを形成し、根本的に悖論を回避する。これは、YaoXiang の型システムが Curry-Howard の意味では **論理的に整合している** ことを意味する。

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

// コードブロック形式（return で値を返さなければならない）
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

返り値は `=` の右辺の形によって决まる：

| 表記 | 返り値 |
|------|--------|
| `= expr`（波括弧なし） | `expr` を直接返す |
| `= { ... }`（波括弧あり） | `return` を使用しなければならない、さもなくば `Void` を返す |

```yaoxiang
# 単一式：直接値を返す、return は不要
add: (a: Int, b: Int) -> Int = a + b

# コードブロック：return で値を返さなければならない
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

# Void 関数：return は不要
print: (msg: String) -> Void = {
    console.write(msg)
}
```

> **設計理由**：`{ ... }` は依存駆動計算ユニット（下記参照）であり、返り値セマンティクスは単一式と異なる。波括弧は複数ステートメントのコンテキストを導入するため、「最後の式が返り値かどうか」というアンビギュイティを解消するために明示的な `return` が必要である。

#### `{}` セマンティクス：依存駆動計算ユニット

`{ ... }` は YaoXiang で単なるコードブロックではない——これは**依存駆動計算ユニット**である。このセマンティクスは関数本体、変数初期化、`spawn` で一貫している：

**基本ルール**：

- `{}` 内の代入ステートメントは記述順ではなく依存関係で自動ソートされる
- 依存が揃えば即座に実行され、欠缺があればブロックして待機
- `return` を使用して明示的に返り値を返す（返り値ルールを参照）

```yaoxiang
# 依存駆動：b は a に依存、コンパイラが自動ソート
result: Int = {
    b = a + 1      # a に依存 → a の後に自動配置
    a = 10         # 依存なし → 先に実行可能
    return b       # 11 を返す
}
```

> **単一式との違い**：`= expr`（波括弧なし）は直接値を返す単純なバインディング；`= { ... }`（波括弧あり）は依存駆動計算コンテキストを導入し、複数ステートメントと明示的な `return` を許可する。

#### `spawn` ブロック

`spawn { ... }` は YaoXiang の唯一の並列プリミティブである。これは `{}` の依存駆動セマンティクスを使用して自動並列化を実現する：

- `spawn { ... }` 内の直接子代入は自動的に並列タスクを生成する
- 依存が揃ったタスクは即座に并发実行される
- 呼び出し元はすべての子タスクの完了までブロックする

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # タスク 1
    b = fetch_data("url2")    # タスク 2（a に依存しない、並列実行）
    c = process(a, b)         # a, b に依存 → 両方の完了を待機してから実行
    return c
}
// 呼び出し元はここでブロック、spawn ブロック内のすべてのタスクが完了するまで待機
```

> **詳細定義**：`spawn` の完全なセマンティクス、タスク生成ルール、ブロッキングモデルは `008-runtime-concurrency-model.md` を参照。

#### `unsafe` ブロック

`unsafe { ... }` は不透明型和裸ポインタの操作を定義するために使用する。これは `{}` の return セマンティクスを使用して型定義を上位スコープに返す：

**基本ルール**：

- `unsafe {}` 内で型と裸ポインタを操作できる
- `return` を使用して型定義を上位スコープに返す
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

# ✅ メソッド呼び出しを通じて
db.close()
```

> **詳細定義**：`unsafe` の完全なセマンティクス、FFI 型定義、メソッドバインディングは `ffi.md` を参照。

#### 3. 型定義

型定義は YaoXiang 統一的構文の中核であり、フィールド、デフォルト値、バインディングメソッド、インターフェース実装を含む：

##### 基本型

**記録型**：フィールドリスト、フィールド型は任意の型式可以使用。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**デフォルト値を持つフィールド**：フィールドにはデフォルト値を持たせることができ、構築時に省略可能。

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

**デフォルト値のないフィールド**：構築時に提供しなければならない。

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

**方式1：型定義体内で直接外部関数をバインディング**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # 位置0にバインディング、カリー化後に method: (b: Point) -> Float
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
    Drawable,          # Drawable インターフェースを実装
    Serializable      # Serializable インターフェースを実装
}
```

##### インターフェース定義

**インターフェース = フィールドがすべて関数の記録型**

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

**`Type.name` プレフィックスは名前空間帰属を表すに過ぎない**。これはいかなる暗黙的なバインディングもトリガーしない。

```yaoxiang
// 名前空間関数：Point 名前空間下の通常の関数
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

// 呼び出し：通常の関数呼び出し
Point.draw(p, screen)
Point.serialize(p)
```

> **注意**：`self` はキーワードではなく、パラメータ名の慣例に過ぎない。`p`、`this`、`x` と書いても結果は完全に同じである。
> コンパイラはパラメータ名を見ず、型を見る。

##### メソッドバインディング（唯一の方法）

`p.draw(screen)` のような `.` メソッド呼び出し構文を動作させるには、**明示的にバインディングしなければならない**。
`[position]` 構文は関数を「メソッド」としてバインディングする唯一の方法である（詳細な構文は RFC-004 を参照）。

```yaoxiang
// 関数を定義
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 明示的にバインディング — これ以降 p.draw(screen) 構文が有効
Point.draw = draw[0]   # 位置 0 のパラメータ（&Point）は呼び出し元が填充

// 使用
p.draw(screen)          # 糖衣構文 → draw(&p, screen)
Point.draw(p, screen)   # 2つの呼び出し方式は同等

// [0] を書かない = バインディングなし。Point.draw は単なる関数エイリアス、. 構文なし
Point.draw = draw       # バインディングなし：Point.draw(p, screen) のみ可能
```

**デフォルト動作**：`[n]` を書かない = パラメータのバインディングなし。ユーザーは呼び出し元が填充するパラメータを明示的に決めなければならない。

**複数位置バインディング**：

```yaoxiang
// 複数の位置をバインディング（自動カリー化）
Point.transform = transform_points[0, 1]
// 呼び出し：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**逆操作**（メソッドから通常関数へ）：

```yaoxiang
// バインディングから関数を取り出す
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. インターフェース組合

```yaoxiang
// インターフェース組合 = 型の交集
DrawableSerializable: Type = Drawable & Serializable

// 交集型を使用
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

// ジェネリックメソッド（RFC-023 構文：型パラメータは呼び出し箇所で自動推論）
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

**基本ルール**：

1. **`()` がすべてを応用する**：型応用、関数呼び出し、値構築はすべて `()` を使用

```yaoxiang
# 型标注
numbers: List(Int) = List(1, 2, 3)

# 空コンテナ：T は左側から得られる
empty: List(Int) = List()

# ジェネリック関数呼び出し——型はパラメータから自動的に流れる
strings = map(numbers, f)
// T=Int は numbers: List(Int) から得られる
// R=String は f: (Int) -> String から得られる
```

2. **Type は左、値は右**：`name: type = value`——Type パラメータは左側で宣言し、右側は常に具体値である。空コンテナ `List()` の `T` は左側の型注釈から取得しなければならない。

3. **型情報は1回だけ記述すればよい**——パラメータ宣言時に記述し、コンパイラがそれを連れて流れる：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int は左側で1回だけ記述
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String は numbers と f の型から自動取得
```

4. **値構築は要素から型を推論する**：

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
> 古い `[]` ジェネリック構文は完全に削除された。`[]` は配列/リストリテラルとインデックスアクセスのみに使用する。

### 例

#### 完全な例

```yaoxiang
// ======== 1. インターフェース定義 ========
// インターフェース = フィールドがすべて関数型の記録型
// インターフェースでは self パラメータは不要 — インターフェースは「呼び出し元位置を削除した後の関数シグネチャ」のみを定義

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

// 関数を定義（self は慣例名であり、キーワードではない）
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

// 明示的にバインディング — バインディング以降は . 呼び出し構文が可能
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

// インスタンスを作成
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// メソッド呼び出し（糖衣構文）
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
    // インターフェースの各フィールド（関数フィールド）に対して
    for (field_name, iface_field) in &iface.fields {
        // 型が同名のメソッドを持つかチェック
        if let Some(method) = typ.methods.get(field_name) {
            // メソッドシグネチャが互換であるかチェック
            // インターフェースフィールド: (Surface) -> Void
            // メソッドシグネチャ: (Point, Surface) -> Void
            // 比較：self パラメータを削除した後、一致する必要がある
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
// 具体的な型を直接代入 → コンパイル時に具体型を特定可能、ゼロオーバーヘッド呼び出し
d: Drawable = Circle(1)
d.draw(screen)  // コンパイル後：circle_draw(screen) を直接呼び出し、vtable なし

// 関数の返り値 → コンパイル時に具体型を特定不可、vtable を使用
d: Drawable = get_shape()
d.draw(screen)  // vtable を使用してメソッドを検索

// 異種集合 → vtable を使用
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // vtable を使用してメソッドを検索
}
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| `d: Drawable = Circle(1)` | 具体型 Circle | 直接呼び出し（ゼロオーバーヘッド） |
| `d: Drawable = get_shape()` | 不明 | vtable |
| `shapes: List(Drawable) = [...]` | 異種 | vtable |

**ルール**：

1. 右辺が具体的な型構築子でコンパイル時に特定可能な場合、直接呼び出し IR を生成する
2. 右辺の型がコンパイル時に特定できない場合、vtable メカニズムにフォールバックする
3. vtable が実行時多形の正確性を保証する

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

### 構文の変更

| 以前 | 以後 |
|------|------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| `impl` キーワードが必要 | キーワード不要、インターフェース名は型の本文の後に記述 |

## 構文設計の説明：名前付き関数は本質的に Lambda の糖衣構文

### コアな理解

**名前付き関数と Lambda 式は同じものである！** 唯一の区別は、名前付き関数が Lambda に名前を付けていることである。

```yaoxiang
// この2つは本質的に同一
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

**重要なポイント**：シグネチャがパラメータ型を完全に宣言している場合、Lambda ヘッダーのパラメータ名は冗長になり、省略可能である。

### パラメータスコープルール

**パラメータは外層変数を覆盖する**：シグネチャ内のパラメータスコープは関数本体を覆盖し、内部スコープの方が優先順位が高い。

```yaoxiang
x = 10  // 外層変数

double: (x: Int) -> Int = x * 2  // ✅ パラメータ x が外層の x を覆盖、結果は 20
```

### 注釈位置は柔軟

型注釈は以下のいずれかの位置に記述でき、**少なくとも1箇所に注釈すればよい**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambda ヘッダーのみ | `double = (x: Int) => x * 2` | ✅ 合法 |
| 両方に記述 | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗長だが許可 |

### 完全な例

```yaoxiang
// ✅ 推奨：シグネチャが完全、Lambda ヘッダーは省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda ヘッダーで型を注釈
double = (x: Int) => x * 2

// ✅ 合法：両方に注釈
double: (x: Int) -> Int = (x) => x * 2
```

### 設計の優位性

| 特性 | 優位性 |
|------|------|
| **簡潔** | シグネチャが完全ならパラメータ名を繰り返し記述する必要がない |
| **柔軟** | Lambda 形式を保持し、好みのものを使用可能 |
| **一貫** | 変数宣言 `x: Int = 42` と統一パターンを維持 |
| **直感的** | `name: Type = body` が直接「name という名前、Type という型、body という値」に対応 |

## トレードオフ

### 优点

| 优点 | 説明 |
|------|------|
| 極端な統一 | 1つの構文ルールですべての場合をカバー |
| 理論的に優雅 | 完璧に対称的な `name: type = value` |
| 新しいキーワード不要 | 既存の構文要素を再利用 |
| 実装が容易 | コンパイラは1つの宣言形式のみを処理すればよい |
| 学習が容易 | 1つのパターンを覚えればすべてのコードが書ける |
| 拡張が容易 | 新機能は自然にこのモデルに溶け込める |

### 欠点

| 欠点 | 説明 |
|------|------|
| 命名規則 | メソッドは `Type.method` の命名に従う必要がある |
| 冗長 | 完全な構文は簡略構文より長いが、推論可能 |
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
// IDE が自動的に不足しているメソッドをヒント
```

### リスク

| リスク | 影響 | 缓解措施 |
|------|------|----------|
| 解析の複雑さ | 統一的構文は解析の複雑さを 증가시킬 가능성이 있는 | 再帰下降構文解析器を使用 |
| パフォーマンスオーバーヘッド | vtable 検索に追加オーバーヘッドが発生する可能性 | コンパイル時単一化最適化 |

---

## 隠し要素 🎮：言語の源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 型の方程式を試みる...
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

> **注**：コンパイラは `Type: Type = Type` を正しく処理できない（Type0/Type1 宇宙悖論が発生する）が、この「隠し要素」を意図的に保持している——コンパイルしようとすると、言語創業者からの禅的なメッセージを受け取る。これは技術的な境界であると同時に、YaoXiang の型哲学への敬意でもある。

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
       | identifier '(' type_expr (',' type_expr)* ')'      # 型応用
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # 関数型
       | '{' type_field* '}'                       # 記録/インターフェース型
       | 'Type'                                    # メタタイプ

type_field ::= identifier ':' type_expr
             | identifier                           # インターフェース制約

# ジェネリックパラメータ：関数型の一部として，例如 (T: Type, R: Type) -> (...)
# 独立した BNF ルールは不要——: Type パラメータは通常の関数パラメータ

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

| 用語 | 定義 |
|------|------|
| 宣言 | `name: type = value` 形式の代入ステートメント |
| 記録型 | 名前付きフィールドを含む `{ ... }` 型 |
| インターフェース | フィールドがすべて関数型の記録型 |
| ジェネリック型 | `Name: (T: Type) -> Type = { ... }` として定義された、型パラメータを受け取る型 |
| 名前空間関数 | `Type.name` 形式の関数、Type 名前空間に属する。いかなるバインディングも暗黙的に含む |
| メソッドバインディング | `Type.name = func[n]`、func の位置 n を呼び出し元にバインディングし、`obj.name(args)` 構文を使用可能にする |
| ジェネリック関数 | `(T: Type)` 構文を使用した関数、型パラメータは最初のパラメータグループ |
| メタタイプ | `Type`、言語内で唯一の型レベルマーカー |

---

## ライフサイクルと归宿

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
│ (正式設計)  │    │ (元の位置に保持) │
└─────────────┘    └─────────────┘
```
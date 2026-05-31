# モジュールシステム仕様

本ドキュメントでは、YaoXiang プログラミング言語のモジュールシステム仕様を定義する。モジュール定義、インポート・エクスポート、およびスコープを含む。

---

## 第1章：モジュール定義

### 1.1 モジュールの基礎

モジュールはファイルを境界として使用する。各 `.yx` ファイルは1つのモジュールである。

```
// ファイル名がモジュール名となる
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 モジュールの命名規則

- モジュール名はファイル名で決まる
- ファイル拡張子 `.yx` はモジュール名に含めない
- モジュール名には PascalCase を使用

---

## 第2章：モジュールのインポート

### 2.1 インポート構文

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= ('{' ImportItems '}') ('as' AliasList)?
              |  'as' AliasList
ImportItems  ::= Identifier (',' Identifier)* ','?
AliasList    ::= Identifier (',' Identifier)*
```

### 2.2 インポート方式

| 構文 | 説明 | 例 |
|------|------|------|
| `use path;` | モジュール全体をインポートし、最後の部分でアクセス | `use std.io;` -> `io.print` |
| `use path.{a, b};` | 指定したアイテムをインポート | `use std.io.{print};` -> `print` |
| `use path as alias;` | インポートして別名を付与 | `use std.io as io;` -> `io.print` |
| `use path.{i1, i2} as a, b;` | 指定したアイテムをインポートして別名を付与 | `use std.io.{print, read} as p, r;` -> `p`, `r` |

### 2.3 インポートの例

```yaoxiang
// モジュール全体をインポート
use std.io
io.print("Hello")

// 指定したアイテムをインポート
use std.io.{print, read}
print("Hello")

// インポートして別名を付与
use std.io as io_module
io_module.print("Hello")

// 指定したアイテムをインポートして別名を付与
use std.io.{print, read} as p, r
p("Hello")
```

---

## 第3章：モジュールのエクスポート

### 3.1 pub キーワード

エクスポートするアイテムは `pub` キーワードで宣言する：

```yaoxiang
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }

// プライベートアイテム（エクスポートされない）
internal_value: Int = 42
```

### 3.2 エクスポート規則

- デフォルトではすべてのアイテムはプライベートである
- `pub` で宣言されたアイテムは他のモジュールからアクセス可能
- プライベートアイテムは現在のモジュール内のみでアクセス可能

### 3.3 pub 自動バインディング

`pub` で宣言された関数には、コンパイラが同ファイル内で定義された型に自動的にバインディングする：

```yaoxiang
// pub で宣言すると、コンパイラが自動的にバインディングする
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// コンパイラの自動推論：
// 1. Point が現在のファイルで定義されている
// 2. 関数引数に合計型が含まれている
// 3. Point.distance = distance[0] を実行

// 呼び出し
d = distance(p1, p2)           // 関数式
d2 = p1.distance(p2)           // OOP 糖衣構文
```

---

## 第4章：スコープ

### 4.1 モジュールスコープ

各モジュールは独自のスコープを持ち、モジュール内のアイテムはデフォルトで外部から不可視である。

### 4.2 ネストされたスコープ

```yaoxiang
// ブロックスコープ
{
    x = 10
    // x はこのスコープ内で可視
}
// x はこのスコープ外では不可視

// 関数スコープ
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result は関数外では不可視
```

### 4.3 遮蔽規則

```yaoxiang
// 変数の遮蔽
x = 10
x = 20  // 前の x を遮蔽

// スコープの遮蔽
x = 10
{
    x = 20  // 外側の x を遮蔽
    // ここでの x は 20
}
// ここでの x は 10
```

---

## 第5章：モジュール構成

### 5.1 ディレクトリ構造

```
src/
├── main.yx          // メイン module
├── math/
│   ├── index.yx     // 数学 module のエントリ
│   ├── vector.yx    // ベクトル module
│   └── matrix.yx    // 行列 module
└── utils/
    ├── index.yx     // ユーティリティ module のエントリ
    └── string.yx    // 文字列ユーティリティ
```

### 5.2 モジュールのエントリポイント

ディレクトリ内の `index.yx` ファイルがモジュールのエントリポイントとなる：

```yaoxiang
// math/index.yx
use math.vector
use math.matrix

pub Vector = vector.Vector
pub Matrix = matrix.Matrix
```

### 5.3 相対インポート

```yaoxiang
// math/vector.yx 内にて
use math.matrix  // 絶対インポート
use .matrix      // 相対インポート（同一ディレクトリ）
```

---

## 付録：モジュール構文早見表

### A.1 module はファイルである

```
// ファイル名.yx がモジュール名となる
Import ::= 'use' ModuleRef
```

### A.2 インポートとエクスポート

```yaoxiang
// インポート
use std.io
use std.io.{print, read}
use std.io as io

// エクスポート
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```
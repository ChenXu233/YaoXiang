# モジュールシステム仕様

本ファイルでは、YaoXiang プログラミング言語のモジュールシステム仕様を定義する。モジュール定義、インポート/エクスポート、スコープを含む。

---

## 第1章：モジュール定義

### 1.1 モジュールの基礎

モジュールはファイルを境界とする。各 `.yx` ファイルが1つのモジュールとなる。

```yaoxiang
// ファイル名がモジュール名となる
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 モジュール命名規則

- モジュール名はファイル名で決定される
- ファイル拡張子 `.yx` はモジュール名に含まない
- モジュール名は PascalCase を使用する

---

## 第2章：モジュールインポート

### 2.1 インポート構文

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= ('{' ImportItems '}') ('as' AliasList)?
              |  'as' AliasList
ImportItems  ::= Identifier (',' Identifier)* ','?
AliasList    ::= Identifier (',' Identifier)*
```

### 2.2 インポート方法

| 構文 | 説明 | 例 |
|------|------|------|
| `use path;` | モジュールをインポートし、最終部分でアクセス | `use std.io;` -> `io.print` |
| `use path.{a, b};` | 指定項目をインポート | `use std.io.{print};` -> `print` |
| `use path as alias;` | インポートして名前変更 | `use std.io as io;` -> `io.print` |
| `use path.{i1, i2} as a, b;` | 指定項目をインポートして名前変更 | `use std.io.{print, read} as p, r;` -> `p`, `r` |

### 2.3 インポート例

```yaoxiang
// モジュール全体をインポート
use std.io
io.print("Hello")

// 指定項目をインポート
use std.io.{print, read}
print("Hello")

// インポートして名前変更
use std.io as io_module
io_module.print("Hello")

// 指定項目をインポートして名前変更
use std.io.{print, read} as p, r
p("Hello")
```

---

## 第3章：モジュールエクスポート

### 3.1 pub キーワード

`pub` キーワードを使用してエクスポート項目を宣言する：

```yaoxiang
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }

// プライベート項目（エクスポートしない）
internal_value: Int = 42
```

### 3.2 エクスポート規則

- デフォルトですべての項目はプライベート
- `pub` で宣言された項目は他のモジュールからアクセス可能
- プライベート項目は現在のモジュール内でのみアクセス可能

### 3.3 pub 自動バインディング

`pub` で宣言された関数について、コンパイラは同ファイル内で定義された型に自動的にバインディングする：

```yaoxiang
// pub で宣言すると、コンパイラが自動バインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// コンパイラの自動推論：
// 1. Point が現在のファイルで定義されている
// 2. 関数引数に関数 Point が含まれている
// 3. Point.distance = distance[0] を実行

// 呼び出し
d = distance(p1, p2)           // 関数スタイル
d2 = p1.distance(p2)           // OOP 糖衣構文
```

---

## 第4章：スコープ

### 4.1 モジュールスコープ

各モジュールは独自のスコープを持ち、モジュール内の項目はデフォルトで外部から不可視である。

### 4.2 ネストスコープ

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

### 4.3 シャドウイング規則

```yaoxiang
// 変数のシャドウイング
x = 10
x = 20  // 前の x をシャドウ

// スコープのシャドウイング
x = 10
{
    x = 20  // 外側の x をシャドウ
    // ここでの x は 20
}
// ここでの x は 10
```

---

## 第5章：モジュール構成

### 5.1 ディレクトリ構造

```
src/
├── main.yx          // メインモジュール
├── math/
│   ├── index.yx     // 数学モジュールエントリ
│   ├── vector.yx    // ベクトルモジュール
│   └── matrix.yx    // 行列モジュール
└── utils/
    ├── index.yx     // ユーティリティモジュールエントリ
    └── string.yx    // 文字列ユーティリティ
```

### 5.2 モジュールエントリ

ディレクトリ内の `index.yx` ファイルがモジュールエントリとなる：

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

### A.1 モジュールはファイル

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
# モジュールシステム仕様

本書は YaoXiang プログラミング言語のモジュールシステム仕様を定義するものであり、モジュールの定義、インポート・エクスポートを含む。

---

## 第1章：モジュールの定義

### 1.1 モジュールの基礎

モジュールはファイルを境界とする。各 `.yx` ファイルが1つのモジュールとなる。

```
// ファイル名がモジュール名となる
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 モジュールの命名規則

- モジュール名はファイル名で決定される
- ファイル拡張子 `.yx` はモジュール名に含まない
- モジュール名には PascalCase を使用する

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
| `use path;` | モジュールをインポートし、最後の部分でアクセス | `use std.io;` -> `io.print` |
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

## 第3章：モジュールのエクスポート

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

- デフォルトではすべての項目はプライベートである
- `pub` で宣言された項目は他のモジュールからアクセス可能
- プライベート項目は現在のモジュール内でのみアクセス可能

### 3.3 pub 自動バインディング

`pub` で宣言された関数について、コンパイラは同ファイル内で定義された型に自動的にバインディングする：

```yaoxiang
// pub で宣言すると、コンパイラが自動バインディングを行う
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// コンパイラの自動推論：
// 1. Point は現在のファイルで定義されている
// 2. 関数の引数に Point が含まれている
// 3. Point.distance = distance[0] を実行

// 呼び出し
d = distance(p1, p2)           // 関数型
d2 = p1.distance(p2)           // OOP シンタックスシュガー
```

---

## 第4章：スコープ

### 4.1 モジュールスコープ

各モジュールは独自のスコープを持ち、モジュール内の項目はデフォルトで外部から参照できない。

### 4.2 ネストスコープ

```yaoxiang
// ブロックスコープ
{
    x = 10
    // x はこのスコープ内で参照可能
}
// x はこのスコープ外では参照不可

// 関数スコープ
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result は関数外では参照不可
```

### 4.3 変数宣言とシャドウイング

YaoXiang には `let` キーワードがない。`x = value` は宣言か代入か？原則は以下の通り：

**代入優先。** 宣言は1回のみだが、代入は百回もある。高頻度操作は最短パスで実行すべき。

```
x = value:
  スコープチェーンを辿って x を探す
    → mut x が見つかった    ：代入、OK（&mut トークン経由で）
    → x（不変）が見つかった：E2010 再代入不可
    → 見つからない          ：現在のスコープで新規宣言（唯一の宣言パス）

mut x = value:
    → 現在のスコープに x が既に存在：E2002 重複定義
    → 外側のスコープに x が存在    ：E2013 シャドウイング禁止（明示的新規宣言は外側と同名不可）
    → 競合なし                      ：新規可変宣言
```

- **同スコープ内**：同じ名前は1回のみ宣言可能（E2002）
- **内側で `mut` なし**：外側を優先的に探索し、代入またはエラー
- **内側で `mut` あり**：明示的に新規宣言し、外側と同名を禁止（E2013）

#### 同スコープ内

```yaoxiang
x = 10
x = 20              // E2002：'x' はこのスコープで既に定義されている

mut y = 10
y = 20              // OK：同一バインディングに再代入
mut y = 30          // E2002：'y' はこのスコープで既に定義されている

z = 10
mut z = 20          // E2002：'z' はこのスコープで既に定義されている（mut は既存宣言をオーバーライド不可）
```

#### 跨ぎスコープ

```yaoxiang
// 外側不変、内側で代入 → 不変変数は再代入不可
x = 10
{
    x = 20          // E2010：'x' は不変なので再代入不可
}
{
    mut x = 20      // E2013：既存の変数 'x' をシャドウイングできない（明示的宣言による新バインディング）
}

// 外側 mut、内側で代入 → 同一バインディングを変更
mut y = 10
{
    y = 20          // OK：同一バインディング、&mut トークン経由で変更
}
print(y)            // 20

// 外側 mut、内側では同名宣言不可
mut z = 10
{
    z = 30          // OK：同一バインディング
}
{
    mut z = 30      // E2013：既存の変数 'z' をシャドウイングできない
}

// 多段ネスト：mut は全レベルを貫通
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// 不変は全レベルを貫いても再代入不可
b = 0
{
    {
        b = 10      // E2010：'b' は不変なので再代入不可
    }
}
```

#### for ループ

```yaoxiang
// ループ変数は各イテレーションで新規バインディングであり、変更ではない
for i in 1..5 {
    print(i)        // OK：各イテレーションで新値をバインディング
    i = 10          // E2010：不変ループ変数は再代入不可
}

for mut i in 1..5 {
    i = 10          // OK：可変ループ変数
}

// ループ変数は外側をシャドウイングできない
i = 0
for i in 1..5 {     // E2013：既存の変数 'i' をシャドウイングできない
}

// mut 外側アキュムレータはループ内で変更可能
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK：同一バインディング、&mut トークン経由で変更
}
print(sum)          // 15

// 不変外側はループ内で変更不可
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010：'sum2' は不変なので再代入不可
}
```

#### 関連エラーコード

| エラーコード | メッセージ | トリガー条件 |
|--------|------|----------|
| E2002 | `'{name}' is already defined in this scope` | 同スコープ内での重複宣言（mut の有無問わない） |
| E2010 | `Cannot assign to immutable variable '{name}'` | 内側で `mut` なしに代入を試みたが、外側変数が不変 |
| E2013 | `Cannot shadow existing variable '{name}'` | 内側で明示的宣言（`mut x` または `x: Type`）を行い、外側と同名にした |

---

## 第5章：モジュールの構成

### 5.1 ディレクトリ構造

```
src/
├── main.yx          // メインモジュール
├── math/
│   ├── index.yx     // 数学モジュール入口
│   ├── vector.yx    // ベクトルモジュール
│   └── matrix.yx    // 行列モジュール
└── utils/
    ├── index.yx     // ユーティリティモジュール入口
    └── string.yx    // 文字列ユーティリティ
```

### 5.2 モジュールエントリ

ディレクトリ内の `index.yx` ファイルがモジュールのエントリとなる：

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

### A.1 モジュールはファイルである

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
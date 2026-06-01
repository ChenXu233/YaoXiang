# モジュールシステム仕様

本文書は YaoXiang プログラミング言語のモジュールシステム仕様を定義ものであり、モジュールの定義、インポート・エクスポート、スコープを含む。

---

## 第1章：モジュールの定義

### 1.1 モジュールの基礎

モジュールはファイルを境界とする。各 `.yx` ファイルが1つのモジュールとなる。

```
// ファイル名がモジュール名になる
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 モジュールの命名規則

- モジュール名はファイル名で決まる
- ファイルの拡張子 `.yx` はモジュール名には含まれない
- モジュール名には PascalCase 命名を使用する

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
| `use path.{a, b};` | 指定した項目をインポート | `use std.io.{print};` -> `print` |
| `use path as alias;` | インポートして名前を変更 | `use std.io as io;` -> `io.print` |
| `use path.{i1, i2} as a, b;` | 指定した項目をインポートして名前を変更 | `use std.io.{print, read} as p, r;` -> `p`, `r` |

### 2.3 インポート例

```yaoxiang
// モジュール全体をインポート
use std.io
io.print("Hello")

// 指定した項目をインポート
use std.io.{print, read}
print("Hello")

// インポートして名前を変更
use std.io as io_module
io_module.print("Hello")

// 指定した項目をインポートして名前を変更
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

// プライベート項目（エクスポートされない）
internal_value: Int = 42
```

### 3.2 エクスポート規則

- デフォルトではすべての項目はプライベートである
- `pub` で宣言された項目は他のモジュールからアクセス可能
- プライベート項目は現在のモジュール内からのみアクセス可能

### 3.3 pub 自動バインディング

`pub` で宣言された関数について、コンパイラは自動的に同じファイルに定義された型にバインドする：

```yaoxiang
// pub で宣言すると、コンパイラが自動的にバインド
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// コンパイラの自動推論：
// 1. Point が現在のファイルで定義されている
// 2. 関数引数に Point が含まれている
// 3. Point.distance = distance[0] を実行

// 呼び出し
d = distance(p1, p2)           // 関数型
d2 = p1.distance(p2)           // OOP 糖衣構文
```

---

## 第4章：スコープ

### 4.1 モジュールのスコープ

各モジュールは独自のスコープを持ち、モジュール内の項目はデフォルトでは外部から不可視である。

### 4.2 ネストしたスコープ

```yaoxiang
// ブロックスコープ
{
    x = 10
    // x はこのスコープ内のみで可視
}
// x はこのスコープ外では不可視

// 関数スコープ
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result は関数外では不可視
```

### 4.3 変数の宣言とシャドウ

YaoXiang には `let` キーワードがない。`x = 値` は宣言か代入か？原則に従う：

**代入を優先。** 宣言は1回しかしないが、代入は何百回もする。高頻度操作は最短経路で。

```
x = 値:
  スコープチェーンを外方向に検索して x を探す
    → mut x が見つかった    ：代入、OK（&mut トークン経由で）
    → x（変更不可）が見つかった：E2010 変更不可変数の再代入不可
    → 見つからない         ：現在のスコープで新規宣言（唯一の宣言パス）

mut x = 値:
    → 現在のスコープに x が既に存在 ：E2002 重复定義
    → 外側スコープに x が存在        ：E2013 シャドウ禁止（明示的な新規宣言は外側と同名不可）
    → 競合なし              ：新規の変更可能な宣言
```

- **同一スコープ**： любой имяは1回のみ宣言可（E2002）
- **内側で `mut` なし**：外側を優先して検索、代入またはエラー
- **内側で `mut` あり**：明示的な新規宣言、外側と同名は禁止（E2013）

#### 同一スコープ

```yaoxiang
x = 10
x = 20              // E2002：'x' は既にこのスコープで定義済み

mut y = 10
y = 20              // OK：同一バインディング、再代入
mut y = 30          // E2002：'y' は既にこのスコープで定義済み

z = 10
mut z = 20          // E2002：'z' は既にこのスコープで定義済み（mut は既存宣言をオーバーライド不可）
```

#### スコープ間

```yaoxiang
// 外側変更不可、内側で代入 → 変更不可変数は再代入不可
x = 10
{
    x = 20          // E2010：'x' は変更不可なので再代入不可
}
{
    mut x = 20      // E2013：既存の変数 'x' をシャドウ不可（明示的な新規バインディング宣言不可）
}

// 外側 mut、内側で代入 → 同一バインディングを変更
mut y = 10
{
    y = 20          // OK：同一バインディング、&mut トークン経由で変更
}
print(y)            // 20

// 外側 mut、内側で同名宣言不可
mut z = 10
{
    z = 30          // OK：同一バインディング
}
{
    mut z = 30      // E2013：既存の変数 'z' をシャドウ不可
}

// 多段ネスト：mut は全レベルを貫通
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// 変更不可は全レベルを貫通しても再代入不可
b = 0
{
    {
        b = 10      // E2010：'b' は変更不可なので再代入不可
    }
}
```

#### for ループ

```yaoxiang
// ループ変数は各イテレーションで新規バインディング、代入ではない
for i in 1..5 {
    print(i)        // OK：各イテレーションで新規バインディング
    i = 10          // E2010：変更不可ループ変数は再代入不可
}

for mut i in 1..5 {
    i = 10          // OK：変更可能ループ変数
}

// ループ変数は外側をシャドウ不可
i = 0
for i in 1..5 {     // E2013：既存の変数 'i' をシャドウ不可
}

// mut 外側アキュムレータはループ本体で変更可
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK：同一バインディング、&mut トークン経由で変更
}
print(sum)          // 15

// 変更不可外側はループ本体で変更不可
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010：'sum2' は変更不可なので再代入不可
}
```

#### 関連エラーコード

| エラーコード | メッセージ | トリガー条件 |
|--------|------|----------|
| E2002 | `'{name}' is already defined in this scope` | 同一スコープでの重複宣言（mut 関係なし） |
| E2010 | `Cannot assign to immutable variable '{name}'` | 内側で `mut` なし代入時に、外側変数が変更不可 |
| E2013 | `Cannot shadow existing variable '{name}'` | 内側で明示的宣言（`mut x` または `x: Type`）が外側と同名 |

---

## 第5章：モジュールの構成

### 5.1 ディレクトリ構造

```
src/
├── main.yx          // メインモジュール
├── math/
│   ├── index.yx     // 数学モジュールエントリポイント
│   ├── vector.yx    // ベクトルモジュール
│   └── matrix.yx    // 行列モジュール
└── utils/
    ├── index.yx     // ユーティリティモジュールエントリポイント
    └── string.yx    // 文字列ユーティリティ
```

### 5.2 モジュールエントリポイント

ディレクトリ内の `index.yx` ファイルがモジュールエントリポイントとなる：

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

### A.1 モジュールのファイル対応

```
// ファイル名.yx がモジュール名
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
# モジュールシステム仕様

この文書は YaoXiang プログラミング言語のモジュールシステム仕様を定義する。モジュール定義、インポート／エクスポート、スコープを含む。

---

## 第一章：モジュール定義

### 1.1 モジュールの基礎

モジュールはファイルを境界とする。各 `.yx` ファイルが 1 つのモジュールとなる。

```
// ファイル名がそのままモジュール名となる
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```

### 1.2 モジュールの命名規則

- モジュール名はファイル名によって決まる
- ファイル拡張子 `.yx` はモジュール名に含まれない
- モジュール名は PascalCase 命名を使用する

---

## 第二章：モジュールのインポート

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
| `use path;` | モジュールをインポートし、最後の部分でアクセス | `use std.io;` → `io.print` |
| `use path.{a, b};` | 指定した項目をインポート | `use std.io.{print};` → `print` |
| `use path as alias;` | インポートして別名を付ける | `use std.io as io;` → `io.print` |
| `use path.{i1, i2} as a, b;` | 指定した項目をインポートして別名を付ける | `use std.io.{print, read} as p, r;` → `p`, `r` |

### 2.3 インポート例

```yaoxiang
// モジュール全体をインポート
use std.io
io.print("Hello")

// 指定した項目をインポート
use std.io.{print, read}
print("Hello")

// インポートして別名を付ける
use std.io as io_module
io_module.print("Hello")

// 指定した項目をインポートして別名を付ける
use std.io.{print, read} as p, r
p("Hello")
```

---

## 第三章：モジュールのエクスポート

### 3.1 pub キーワード

`pub` キーワードでエクスポート項目を宣言する：

```yaoxiang
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }

// 非公開項目（エクスポートされない）
internal_value: Int = 42
```

### 3.2 エクスポート規則

- デフォルトではすべての項目は非公開である
- `pub` で宣言された項目は他のモジュールからアクセス可能
- 非公開項目は現在のモジュール内でのみアクセス可能

### 3.3 pub 自動バインディング

`pub` で宣言された関数は、コンパイラによって同じファイルで定義された型に自動的にバインドされる：

```yaoxiang
// pub で宣言すると、コンパイラが自動的にバインド
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// コンパイラの自動推論：
// 1. Point は現在のファイルで定義されている
// 2. 関数の引数に Point が含まれる
// 3. Point.distance = distance[0] を実行

// 呼び出し
d = distance(p1, p2)           // 関数形式
d2 = p1.distance(p2)           // OOP シンタックスシュガー
```

---

## 第四章：スコープ

### 4.1 モジュールスコープ

各モジュールは独自のスコープを持ち、モジュール内の項目はデフォルトでは外部から参照できない。

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
// result は関数の外では参照不可
```

### 4.3 変数宣言とシャドーイング

YaoXiang には `let` キーワードがない。`x = value` は宣言なのか代入なのか？以下の原則に従う：

**代入を優先する。** 宣言は一度きりだが、代入は何度も行われる。高頻度の操作を最短経路に通す。

```
x = value:
  スコープチェーンに沿って外側に x を探索
    → mut x が見つかった      ：代入、OK（&mut トークン経由）
    → x（moved 済み）が見つかった ：「有効なバインドが見つからない」とみなし、現在のスコープで再宣言
    → x（不変、生存）が見つかった ：E2010 再代入不可
    → 見つからない              ：現在のスコープで新規宣言（唯一の宣言経路）

mut x = value:
    → 現在のスコープに x が既に存在する ：E2002 重複定義
    → 外側スコープに x が存在する       ：E2013 シャドーイング禁止（明示的な新規宣言は外側と同名不可）
    → 衝突なし                        ：新しい可変宣言
```

- **同一スコープ**：任意の名前は 1 回しか宣言できない（E2002）
- **内側に `mut` なし**：外側を優先して参照し、代入またはエラー
- **内側に `mut` あり**：明示的な新規宣言となり、外側と同名は禁止（E2013）

#### 同一スコープ

```yaoxiang
x = 10
x = 20              // E2002：'x' はこのスコープで既に定義されている

mut y = 10
y = 20              // OK：同一バインドへの再代入
mut y = 30          // E2002：'y' はこのスコープで既に定義されている

z = 10
mut z = 20          // E2002：'z' はこのスコープで既に定義されている（mut は既存宣言を上書きできない）
```

#### move 後の再バインド

不変変数が ownership を保有している場合、その値が move（消費）されると、元バインドは **moved** 状態になる。名前はスコープのスロットを占有し続けるが、値にはアクセスできなくなる。このとき `x = value` は古いバインドの変更ではなく、同一スコープ内での `x` の再宣言となる。

```
代入優先探索の「moved 済み」分岐：
  x が現在のスコープに存在するが、moved 状態
    → コンパイラは「有効なバインドが見つからない」とみなす
    → 現在のスコープで x を再宣言（古い moved スロットを上書き）
```

**中核メカニズム：** 古い値が消費されると、バインドは無効化され、名前は「宣言可能」状態に戻る。これはシャドーイングではない。古いバインドはもう存在しない。

```yaoxiang
// パイプライン型のデータフロー：各ステップで古い値を消費し、新しい値を生成
data = fetch()           // 不変、ownership を保有
data = transform(data)   // move data → 古い data は無効化、新しい data が再バインド
data = filter(data)      // 同上
process(data)

// 等価な明示的記述（比較）：
data1 = fetch()
data2 = transform(data1)  // data1 は move され、再利用不可
data3 = filter(data2)     // data2 は move され、再利用不可
process(data3)
```

**セマンティクスの分離：**

| 操作 | 意味 | メカニズム | 構文 |
|------|------|------|------|
| **再バインド** | 古い値が消滅、新しい値が誕生 | move + 再宣言 | `x = f(x)` |
| **インプレース変更** | 同一メモリ位置の値変化 | mut 代入 | `mut x; x = v` |

**なぜこれがシャドーイングと異なるのか：**
- シャドーイング（Rust の `let x = ...`）：古いバインドは存在し続け、新しいバインドに隠されるだけ
- move 後の再バインド：古いバインドは消費済みであり、名前は未初期化状態に戻る。再宣言は唯一の方法

**制約：**
- ownership を保有する値のみが move 可能。参照（`&T`、`&mut T`）はコピーされ、move されない
- move 検査は compile-time で行われ、moved 状態の変数は任意の式で使用されると E2014 を報告する
- IDE は moved 変数の上にグレー表示でヒントを出せる（その名前が未初期化状態であることを示す）

```yaoxiang
// move 後の読み取り → エラー
data = fetch()
result = process(data)   // data は move される
print(data)              // E2014：'data' は move 済み、使用不可

// 参照は move を引き起こさない
ref_data = &value
copy1 = ref_data         // 参照をコピー、ref_data はまだ使用可能
copy2 = ref_data         // OK

// スコープをまたぐ：moved 状態は貫通する
data = fetch()
{
    data = transform(data)  // 外側の data を move → 再バインド（内側で新規宣言）
    print(data)             // OK：内側の data を使用
}
print(data)                 // E2014：外側の data は move 済み
```

#### スコープをまたぐ

```yaoxiang
// 外側不変、内側で代入 → 不変変数は再代入不可
x = 10
{
    x = 20          // E2010：'x' は不変、再代入不可
}
{
    mut x = 20      // E2013：既存変数 'x' をシャドーイング不可（明示的に新バインドを宣言）
}

// 外側 mut、内側で代入 → 同一バインドを変更
mut y = 10
{
    y = 20          // OK：同一バインド、&mut トークン経由で変更
}
print(y)            // 20

// 外側 mut、内側で同名は宣言不可
mut z = 10
{
    z = 30          // OK：同一バインド
}
{
    mut z = 30      // E2013：既存変数 'z' をシャドーイング不可
}

// 多層ネスト：mut はすべての階層を貫通
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// 不変もすべての階層を貫通するが再代入は不可
b = 0
{
    {
        b = 10      // E2010：'b' は不変、再代入不可
    }
}
```

#### for ループ

```yaoxiang
// ループ変数は各イテレーションで新バインドとなり、変更ではない
for i in 1..5 {
    print(i)        // OK：各イテレーションで新しい値にバインド
    i = 10          // E2010：不変ループ変数は再代入不可
}

for mut i in 1..5 {
    i = 10          // OK：可変ループ変数
}

// ループ変数は外側をシャドーイングできない
i = 0
for i in 1..5 {     // E2013：既存変数 'i' をシャドーイング不可
}

// 外側の mut アキュムレータはループ体内で変更可能
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK：同一バインド、&mut トークン経由で変更
}
print(sum)          // 15

// 不変な外側変数はループ体内で変更不可
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010：'sum2' は不変、再代入不可
}
```

#### 関連エラーコード

| エラーコード | メッセージ | 発生シナリオ |
|--------|------|----------|
| E2002 | `'{name}' is already defined in this scope` | 同一スコープでの重複宣言（mut であるかによらない） |
| E2010 | `Cannot assign to immutable variable '{name}'` | 内側に `mut` がない代入時、外側変数が不変かつ未 moved |
| E2013 | `Cannot shadow existing variable '{name}'` | 内側の明示的宣言（`mut x` または `x: Type`）が外側と同名 |
| E2014 | `'{name}' has been moved and cannot be used` | moved 済み変数の読み取り |

---

## 第五章：モジュール構成

### 5.1 ディレクトリ構造

```
src/
├── main.yx          // メインモジュール
├── math/
│   ├── index.yx     // 数学モジュールのエントリ
│   ├── vector.yx    // ベクトルモジュール
│   └── matrix.yx    // 行列モジュール
└── utils/
    ├── index.yx     // ユーティリティモジュールのエントリ
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
// math/vector.yx の中
use math.matrix  // 絶対インポート
use .matrix      // 相対インポート（同一ディレクトリ）
```

---

## 付録：モジュール構文クイックリファレンス

### A.1 モジュールはファイルそのもの

```
// ファイル名.yx がそのままモジュール名
Import ::= 'use' ModuleRef
```

### A.2 インポート／エクスポート

```yaoxiang
// インポート
use std.io
use std.io.{print, read}
use std.io as io

// エクスポート
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```
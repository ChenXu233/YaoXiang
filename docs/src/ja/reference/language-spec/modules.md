# モジュールシステム仕様

本書は YaoXiang プログラミング言語のモジュールシステム仕様を定義ものであり、モジュールの定義>importation/エクスポート、スコープを含む。

---

## 第1章：モジュールの定義

### 1.1 モジュールの基礎

モジュールはファイルを境界として使用する。各 `.yx` ファイルが1つのモジュールである。

```
// ファイル名がモジュール名となる
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 モジュールの命名規則

- モジュール名はファイル名で決定される
- ファイル拡張子 `.yx` はモジュール名に寄与しない
- モジュール名は PascalCase 命名を使用する

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
| `use path.{a, b};` | 指定した項目をインポート | `use std.io.{print};` -> `print` |
| `use path as alias;` | インポートしてリネーム | `use std.io as io;` -> `io.print` |
| `use path.{i1, i2} as a, b;` | 指定した項目をインポートしてリネーム | `use std.io.{print, read} as p, r;` -> `p`, `r` |

### 2.3 インポートの例

```yaoxiang
// モジュール全体をインポート
use std.io
io.print("Hello")

// 指定した項目をインポート
use std.io.{print, read}
print("Hello")

// インポートしてリネーム
use std.io as io_module
io_module.print("Hello")

// 指定した項目をインポートしてリネーム
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

- デフォルトですべての項目はプライベートである
- `pub` で宣言された項目は他のモジュールからアクセス可能
- プライベート項目は現在のモジュール内でのみアクセス可能

### 3.3 pub 自動バインディング

`pub` で宣言された関数は、コンパイラが同じファイル内で定義された型に自動的にバインディングする：

```yaoxiang
// pub で宣言すると、コンパイラが自動的にバインディングする
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// コンパイラの自動推論：
// 1. Point は現在のファイルで定義されている
// 2. 関数パラメータに Point が含まれている
// 3. Point.distance = distance[0] を実行する

// 呼び出し
d = distance(p1, p2)           // 関数形式
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
// result は関数の外では不可視
```

### 4.3 変数の宣言と遮蔽

YaoXiang には `let` キーワードがない。`x = value` は宣言か代入か？1つの原則に従う：

**代入を優先する。** 宣言は1回しかないが、代入は百回ある。高頻度操作最短経路を通す。

```
x = value:
  スコープチェーンを外に向かって x を検索する
    → mut x が見つかった          ：代入、OK（&mut トークン経由）
    → x が見つかった（既に moved）   ："有効なバインディングなし"と見なし、現在のスコープで再宣言する
    → x が見つかった（不変、生存）  ：E2010 不変変数は再代入不可
    → 見つからない              ：現在のスコープで新規宣言（唯一の宣言パス）

mut x = value:
    → 現在のスコープに x が既に存在 ：E2002 重複定義
    → 外側スコープに x が存在   ：E2013 遮蔽禁止（明示的な新規宣言は外側と同名不可）
    → 競合なし              ：新規可変宣言
```

- **同スコープ**：同じ名前は1回しか宣言できない（E2002）
- **内側で `mut` なし**：外側を優先して検索、代入またはエラー
- **内側で `mut` あり**：明示的な新規宣言、外側と同名は不可（E2013）

#### 同スコープ内

```yaoxiang
x = 10
x = 20              // E2002：'x' はこのスコープで既に定義されている

mut y = 10
y = 20              // OK：同じバインディングへの再代入
mut y = 30          // E2002：'y' はこのスコープで既に定義されている

z = 10
mut z = 20          // E2002：'z' はこのスコープで既に定義されている（mut は既存宣言をオーバーライドできない）
```

#### Move 後の再バインディング

不変変数が所有権を持つ場合、値が move（消費）されると、元のバインディングは **moved** 状態になる——名前はスコープスロットにまだ存在し、値にはアクセスできない。このとき `x = value` は古いバインディングを変更するのではなく、同じスコープ内で `x` を再宣言する。

```
代入優先検索の"既に moved"分岐：
  x が現在のスコープに存在するが、moved 状態にある
    → コンパイラは"有効なバインディングなし"として扱う
    → 現在のスコープで x を再宣言する（古い moved スロットを上書き）
```

**コアメカニズム：** 古い値が消費された後、バインディングが無効になり、名前は"未初期化"状態に戻る。再宣言が唯一の道筋である。これは遮蔽ではない——古いバインディングはもう存在しない。

```yaoxiang
// パイプライン式データフロー：各ステップが古い値を消費し、新しい値を生成する
data = fetch()           // 不変、所有権を持つ
data = transform(data)   // data を move → 古い data は無効、新しい data が再バインディングされる
data = filter(data)      // 同上
process(data)

// 等価な明示的記述（比較用）：
data1 = fetch()
data2 = transform(data1)  // data1 が move、使用不可
data3 = filter(data2)     // data2 が move、使用不可
process(data3)
```

**意味的分離：**

| 操作 | 意味 | メカニズム | 構文 |
|------|------|------|------|
| **再バインディング** | 古い値が消え、新しい値が誕生 | move + 再宣言 | `x = f(x)` |
| **インプレース変更** | 同一メモリアドレスの値が変化 | mut 代入 | `mut x; x = v` |

**なぜこれは遮蔽と異なるのか：**
- 遮蔽（Rust の `let x = ...`）：古いバインディングはまだ存在し、ただ新しいバインディングに隠されているだけ
- Move 後の再バインディング：古いバインディングは既に消費されており、名前は未初期化状態に戻り、再宣言は唯一の方法

**制約：**
- 所有権を持つ値だけが move できる。参照（`&T`、`&mut T`）はコピー而不是移动
- Move チェックはコンパイル時完了、moved 状態の変数をどの式かで読むと E2014 が発生
- IDE は moved 変数に灰色ヒントを表示し、その名前が未初期化状態であることを示せる

```yaoxiang
// move 後に読む → エラー
data = fetch()
result = process(data)   // data が move される
print(data)              // E2014：'data' は move されており、使用不可

// 参照は move をトリガーしない
ref_data = &value
copy1 = ref_data         // 参照をコピー、ref_data はまだ使用可能
copy2 = ref_data         // OK

// スコープをまたぐ：moved 状態はスコープを貫通する
data = fetch()
{
    data = transform(data)  // 外側の data を move → 再バインディング（内側の新規宣言）
    print(data)             // OK：内側の data を使用
}
print(data)                 // E2014：外側の data は既に move されている
```

#### スコープをまたぐ

```yaoxiang
// 外側：不変、内側：代入 → 不変変数は再代入できない
x = 10
{
    x = 20          // E2010：'x' は不変で、再代入不可
}
{
    mut x = 20      // E2013：既存の変数 'x' を遮蔽不可（明示的な新規宣言）
}

// 外側：mut、内側：代入 → 同じバインディングを変更する
mut y = 10
{
    y = 20          // OK：同じバインディング、&mut トークンで変更
}
print(y)            // 20

// 外側：mut、内側：同名は宣言不可
mut z = 10
{
    z = 30          // OK：同じバインディング
}
{
    mut z = 30      // E2013：既存の変数 'z' を遮蔽不可
}

// 複数ネスト：mut はすべてのレベルを貫通する
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// 不変はすべてのレベルを貫通しても再代入不可
b = 0
{
    {
        b = 10      // E2010：'b' は不変で、再代入不可
    }
}
```

#### for ループ

```yaoxiang
// ループ変数は各反復で新しいバインディングであり、変更ではない
for i in 1..5 {
    print(i)        // OK：各反復で新しい値をバインディング
    i = 10          // E2010：不変ループ変数は再代入不可
}

for mut i in 1..5 {
    i = 10          // OK：可変ループ変数
}

// ループ変数は外側を遮蔽不可
i = 0
for i in 1..5 {     // E2013：既存の変数 'i' を遮蔽不可
}

// mut の外側累積子はループ本体で変更可能
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK：同じバインディング、&mut トークンで変更
}
print(sum)          // 15

// 不変の外側累積子はループ本体で変更不可
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010：'sum2' は不変で、再代入不可
}
```

#### 関連エラーコード

| エラーコード | メッセージ | トリガーシナリオ |
|--------|------|------|
| E2002 | `'{name}' is already defined in this scope` | 同スコープでの重複宣言（mut かどうかに関わらず） |
| E2010 | `Cannot assign to immutable variable '{name}'` | 内側で `mut` なし、代入時、外側変数が不変で moved でない |
| E2013 | `Cannot shadow existing variable '{name}'` | 内側で明示的な宣言（`mut x` または `x: Type`）が外側と同名 |
| E2014 | `'{name}' has been moved and cannot be used` | 既に moved の変数を読み取る |

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

ディレクトリ内の `index.yx` ファイルはモジュールエントリとして機能する：

```yaoxiang
// math/index.yx
use math.vector
use math.matrix

pub Vector = vector.Vector
pub Matrix = matrix.Matrix
```

### 5.3 相対インポート

```yaoxiang
// math/vector.yx 内
use math.matrix  // 絶対インポート
use .matrix      // 相対インポート（同一ディレクトリ）
```

---

## 付録：モジュール構文早見表

### A.1 モジュール＝ファイル

```
// ファイル名.yx がモジュール名となる
Import ::= 'use' ModuleRef
```

### A.2 インポート・エクスポート

```yaoxiang
// インポート
use std.io
use std.io.{print, read}
use std.io as io

// エクスポート
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```
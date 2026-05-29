# YaoXiang クイックスタート

> このガイドでは、YaoXiang プログラミング言語の基本的な使い方を説明します。
>
> **注意**：このドキュメントのコード例は、YaoXiang 言語仕様に基づいて記述されています。実際の実行で構文の差異が発生した場合は、[言語仕様](../design/language-spec.md)を参照してください。

## インストール

### ソースからのコンパイル（推奨）

```bash
# リポジトリのクローン
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# コンパイル（デバッグバージョン、開発テスト用）
cargo build

# コンパイル（リリースバージョン、本番環境推奨）
cargo build --release

# テストの実行
cargo test

# バージョンの確認
./target/debug/yaoxiang --version
# または
./target/release/yaoxiang --version
```

**インストール成功の確認**：
```bash
./target/debug/yaoxiang --version
# 次のように出力されるはず: yaoxiang x.y.z
```

## 最初のプログラム

ファイル `hello.yx` を作成します：

```yaoxiang
# hello.yx
use std.io

# 関数定義: name: (param: Type, ...) -> return_type = { ... }
main: () -> Void = {
    println("Hello, YaoXiang!")
}
```

実行：

```bash
./target/debug/yaoxiang hello.yx
# または release バージョンを使用
./target/release/yaoxiang hello.yx
```

出力：

```
Hello, YaoXiang!
```

## 基本概念

### 変数と型

```yaoxiang
# 自動型推論
x = 42                    # Int に推論
name = "YaoXiang"         # String に推論
pi = 3.14159              # Float に推論
is_valid = true           # Bool に推論

# 明示的な型注釈（型集約の規則を使用することが推奨）
count: Int = 100

# デフォルトで不変（安全上の特性）
x = 10
x = 20                    # ❌ コンパイルエラー！不変

# 可変変数（明示的な宣言が必要）
mut counter = 0
counter = counter + 1     # ✅ OK
```

### 関数

```yaoxiang
# 関数定義の構文
add: (a: Int, b: Int) -> Int = a + b

# 呼び出し
result = add(1, 2)        # result = 3

# 単一パラメータ関数
inc: (x: Int) -> Int = x + 1
```

### 型定義

YaoXiang は統一された `name: type = value` 構文モデルを使用します：

```yaoxiang
# 変数宣言
x: Int = 42
name: String = "YaoXiang"

# 関数定義
add: (a: Int, b: Int) -> Int = a + b

# 型定義（中括弧を使用）
type Point = { x: Float, y: Float }

# 型の使用
p: Point = Point(x: 1.0, y: 2.0)
p.x  # 1.0
p.y  # 2.0
```

#### 記録型

```yaoxiang
# 構造体型
type Point = { x: Float, y: Float }
type Rect = { x: Float, y: Float, width: Float, height: Float }

# 使用例
p = Point(x: 3.0, y: 4.0)
r = Rect(x: 0.0, y: 0.0, width: 10.0, height: 20.0)
```

#### インターフェース定義

インターフェースとは、すべてのフィールドが関数型である記録型です：

```yaoxiang
# インターフェース定義
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

# 空インターフェース
type EmptyInterface = {}
```

#### 型のメソッド

`Type.method: (Type, ...) -> Return = ...` 構文を使用して型メソッドを定義します：

```yaoxiang
# 型定義
type Point = { x: Float, y: Float }

# 型メソッド定義
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point(${self.x}, ${self.y})"
}

# メソッドの使用（糖衣構文）
p = Point(x: 1.0, y: 2.0)
p.draw(screen)           # → Point.draw(p, screen)
str = p.serialize()      # → Point.serialize(p)
```

#### 自動バインディング

`pub` キーワードで宣言された関数は、同じファイルで定義された型に自動的にバインドされます：

```yaoxiang
type Point = { x: Float, y: Float }

# pub 宣言で Point に自動バインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 使用例
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

# 関数呼び出し
d = distance(p1, p2)           # 3.606...

# OOP 糖衣構文（Point.distance に自動バインディング）
d2 = p1.distance(p2)           # → distance(p1, p2)
```

#### 列挙型

```yaoxiang
# 単純な列挙型
type Color = red | green | blue

# データ付き列挙型
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# ジェネリクスの使用
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### ジェネリック型

```yaoxiang
# ジェネリック型の定義
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

# 具体的なインスタンス化
type IntList = List(Int)
type StringList = List(String)
```

### 制御フロー

```yaoxiang
# 条件式
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

# ループ
for i in 0..5 {
    print(i)
}

# while ループ
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### リストと辞書

```yaoxiang
# リスト
numbers = [1, 2, 3, 4, 5]
first = numbers[0]         # 1

# 辞書
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  # 90

# 要素の追加
mut list = [1, 2, 3]
list.append(4)
```

### パターン照合

```yaoxiang
# match 式
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## 並作プログラミング（非同期）

YaoXiang の特徴的な機能：`spawn` でマークされた関数は自動的に非同期能力を獲得します。

```yaoxiang
# 並作関数の定義（自動的非同期実行）
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# 並作関数の呼び出し（自動的並列実行、await 不要）
main: () -> Void = {
    # 2回の呼び出しは自動的に並列実行
    user = fetch_user(1)     # 自動的並列
    posts = fetch_posts()    # 自動的並列

    # 結果が必要になった時点で自動的に待機
    print(user.name)
    print(posts.length)
}
```

## モジュールシステム

```yaoxiang
# 標準ライブラリのインポート
use std.io
use std.math

# インポートした関数の使用
result = math.sqrt(16)      # 4.0
println("Hello!")
```

## よくある質問

### Q: 変数はデフォルトで不変ですが、変数を変更するにはどうすればいいですか？

```yaoxiang
# mut キーワードを使用して可変変数を宣言
mut x = 10
x = 20                       # ✅ OK
```

### Q: 関数を定義するにはどうすればいいですか？

```yaoxiang
# 完全な形式（推奨）
add: (a: Int, b: Int) -> Int = a + b

# 短い形式（型推論）
add = (a, b) => a + b
```

### Q: エラーを処理するにはどうすればいいですか？

```yaoxiang
# Result 型を使用
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# パターン照合で処理
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## 次のステップ

- 📖 [YaoXiang ガイド](../YaoXiang-book.md) でコア機能を学ぶ
- 📚 [言語仕様](../YaoXiang-language-specification.md) で完全な構文を学ぶ
- 🏗️ [アーキテクチャドキュメント](../architecture/) で実装の詳細を学ぶ
- 💡 [設計マニフェスト](../YaoXiang-design-manifesto.md) で核心理念を学ぶ

## 関連リソース

- [GitHub リポジトリ](https://github.com/yourusername/yaoxiang)
- [Issue フィードバック](https://github.com/yourusername/yaoxiang/issues)
- [貢献ガイド](../guides/dev/)
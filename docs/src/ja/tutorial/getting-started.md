# YaoXiang クイックスタート

> 本ガイドは YaoXiang プログラミング言語を手早く習得するためのものです。
>
> **注意**：本文書内のコード例は YaoXiang 言語仕様に基づいて作成されています。実際の実行時に構文の違いがある場合は、[言語仕様](../design/language-spec.md)を参照してください。

## インストール

### ソースコードからビルド（推奨）

```bash
# リポジトリのクローン
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# ビルド（デバッグ版、開発テスト用）
cargo build

# ビルド（リリース版、本番環境に推奨）
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
# 以下のような出力が出るはず: yaoxiang x.y.z
```

## 最初のプログラム

ファイル `hello.yx` を作成します：

```yaoxiang
// hello.yx
use std.io

// 関数定義: name: (param: Type, ...) -> return_type = { return ... }  # コードブロックは明示的に return が必要
// 式形式: name: (param: Type, ...) -> return_type = expr              # 式は直接値を返す
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

実行：

```bash
./target/debug/yaoxiang hello.yx
# またはリリース版を使用
./target/release/yaoxiang hello.yx
```

出力：

```
Hello, YaoXiang!
```

## 基本概念

### 変数と型

```yaoxiang
// 自動型推論
x = 42  // Int として推論される
name = "YaoXiang"  // String として推論される
pi = 3.14159  // Float として推論される
is_valid = true  // Bool として推論される

// 明示的な型注釈（型集中規約の使用を推奨）
count: Int = 100

// デフォルトで不変（安全機能）
x = 10
x = 20  // ❌ コンパイルエラー！不変です

// 可変変数（明示的な宣言が必要）
mut counter = 0
counter = counter + 1  // ✅ OK
```

### 関数

```yaoxiang
// 関数定義構文
// 式形式：直接値を返し、return は不要
add: (a: Int, b: Int) -> Int = a + b

// コードブロック形式：return で値を返す必要がある
// add: (a: Int, b: Int) -> Int = { return a + b }

// 呼び出し
result = add(1, 2)  // result = 3

// 単一引数関数（式形式）
inc: (x: Int) -> Int = x + 1
```

### 型定義

YaoXiang は統一された `name: type = value` 構文モデルを採用しています：

```yaoxiang
// 変数宣言
x: Int = 42
name: String = "YaoXiang"

// 関数定義
add: (a: Int, b: Int) -> Int = a + b

// 型定義（中括弧を使用）
Point: Type = { x: Float, y: Float }

// 型の使用
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### 記録型（レコード型）

```yaoxiang
// 構造体型
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 使用例
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### インターフェース定義

インターフェースは、フィールドがすべて関数型である記録型です：

```yaoxiang
// インターフェースの定義
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空インターフェース
EmptyInterface: Type = {}
```

#### 型メソッド

`Type.method: (Type, ...) -> Return = ...` 構文を使用して型メソッドを定義します：

```yaoxiang
// 型定義
Point: Type = { x: Float, y: Float }

// 型メソッドの定義
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point({self.x}, {self.y})"
}

// メソッドの使用（糖衣構文）
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### 自動バインディング

`pub` キーワードで宣言された関数は、同じファイルで定義された型に自動的にバインドされます：

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub 宣言により Point に自動バインド
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// 使用例
p1 = Point(x=3.0, y=4.0)
p2 = Point(x=1.0, y=2.0)

// 関数呼び出し形式
d = distance(p1, p2)  // 3.606...

// OOP 糖衣構文（Point.distance に自動バインド）
d2 = p1.distance(p2)  // → distance(p1, p2)
```

#### 列挙型

```yaoxiang
// 単純な列挙
Color: Type = { red | green | blue }

// データを持つ列挙
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// ジェネリクスの使用
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### ジェネリック型

```yaoxiang
// ジェネリック型の定義
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

// 具体的なインスタンス化
IntList: Type = List(Int)
StringList: Type = List(String)
```

### 制御フロー

```yaoxiang
// 条件式
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

// ループ
for i in 0..5 {
    print(i)
}

// while ループ
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### リストと辞書

```yaoxiang
// リスト
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// 辞書
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// 要素の追加
mut list = [1, 2, 3]
list.append(4)
```

### パターン照合

```yaoxiang
// match 式
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## 並行プログラミング（非同期）

YaoXiang 独自の機能：`spawn` でマークされた関数は自動的に非同期機能を獲得します。

```yaoxiang
// 並行関数の定義（自動的に非同期実行される）
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

// 並行関数の呼び出し（自動的に並列実行、await 不要）
main: () -> Void = {
    // 2 回の呼び出しは自動的に並列実行される
    user = fetch_user(1)  // 自動的に並列
    posts = fetch_posts()  // 自動的に並列

    // 結果が必要になった時点で自動的に待機
    print(user.name)
    print(posts.length)
}
```

## モジュールシステム

```yaoxiang
// 標準ライブラリのインポート
use std.io
use std.math

// インポートした関数の使用
result = math.sqrt(16)  // 4.0
print("Hello!")
```

## よくある質問

### Q: 変数はデフォルトで不変ですが、変数を変更するにはどうすればよいですか？

```yaoxiang
// mut キーワードを使って可変変数を宣言
mut x = 10
x = 20  // ✅ OK
```

### Q: 関数を定義するにはどうすればよいですか？

```yaoxiang
// 完全形式（推奨）
add: (a: Int, b: Int) -> Int = a + b

// 短縮形式（型推論）
add = (a, b) => a + b
```

### Q: エラーはどのように処理しますか？

```yaoxiang
// Result 型を使用
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// パターン照合で処理
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## 次のステップ

- 📖 [YaoXiang ガイド](../YaoXiang-book.md)を読んでコア機能を学ぶ
- 📚 [言語仕様](../YaoXiang-language-specification.md)で完全な構文を確認
- 🏗️ [アーキテクチャドキュメント](../architecture/)で実装の詳細を閲覧
- 💡 [デザインマニフェスト](../YaoXiang-design-manifesto.md)で中心的な理念を学ぶ

## 関連リソース

- [GitHub リポジトリ](https://github.com/yourusername/yaoxiang)
- [Issue フィードバック](https://github.com/yourusername/yaoxiang/issues)
- [コントリビューションガイド](../guides/dev/)
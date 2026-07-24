# YaoXiang クイックスタート

> このガイドは YaoXiang プログラミング言語をすぐに使い始められるようサポートします。
>
> **注意**: このドキュメントのコード例は YaoXiang 言語仕様に基づいています。実際の実行時に構文の違いがある場合は、[言語仕様](../reference/language-spec/index.md)を参照してください。

## インストール

### ソースからビルド（推奨）

```bash
# リポジトリのクローン
git clone https://github.com/ChenXu233/YaoXiang.git
cd yaoxiang

# ビルド（デバッグ版、開発テスト用）
cargo build

# ビルド（リリース版、本番環境推奨）
cargo build --release

# テスト実行
cargo test

# バージョン確認
./target/debug/yaoxiang --version
# または
./target/release/yaoxiang --version
```

**インストールの成功を確認**:

```bash
./target/debug/yaoxiang --version
# 以下のような出力が表示されるはず: yaoxiang x.y.z
```

## 最初のプログラム

ファイル `hello.yx` を作成します:

```yaoxiang
// hello.yx
use std.io

// 関数定義: name: (param: Type, ...) -> return_type = { return ... }  # コードブロックは明示的に return が必要
// 式形式: name: (param: Type, ...) -> return_type = expr           # 式は直接値を返す
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

実行:

```bash
./target/debug/yaoxiang hello.yx
# または release 版を使用
./target/release/yaoxiang hello.yx
```

出力:

```
Hello, YaoXiang!
```

## 基本概念

### 変数と型

```yaoxiang
// 自動型推論
x = 42  // Int として推論
name = "YaoXiang"  // String として推論
pi = 3.14159  // Float として推論
is_valid = true  // Bool として推論

// 明示的な型注釈（一元化された型規約の使用を推奨）
count: Int = 100

// デフォルトで不変（安全性のため）
x = 10
x = 20  // ❌ コンパイルエラー！不変

// 可変変数（明示的な宣言が必要）
mut counter = 0
counter = counter + 1  // ✅ OK
```

### 関数

```yaoxiang
// 関数定義構文
// 式形式: 直接値を返し、return は不要
add: (a: Int, b: Int) -> Int = a + b

// コードブロック形式: return を使って値を返す必要がある
// add: (a: Int, b: Int) -> Int = { return a + b }

// 呼び出し
result = add(1, 2)  // result = 3

// 単一引数関数（式形式）
inc: (x: Int) -> Int = x + 1
```

### 型定義

YaoXiang は統一された `name: type = value` 構文モデルを採用しています:

```yaoxiang
// 変数宣言
x: Int = 42
name: String = "YaoXiang"

// 関数定義
add: (a: Int, b: Int) -> Int = a + b

// 型定義（波括弧を使用）
Point: Type = { x: Float, y: Float }

// 型の使用
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### 記録型（record type）

```yaoxiang
// 構造体型
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 使用例
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### インターフェース定義

インターフェースは、フィールドがすべて関数型である記録型です:

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

`Type.method: (Type, ...) -> Return = ...` 構文を使って型メソッドを定義します:

```yaoxiang
// 型定義
Point: Type = { x: Float, y: Float }

// 型メソッド定義
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point({self.x}, {self.y})"
}

// メソッドの使用（シンタックスシュガー）
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### 自動バインディング

`pub` キーワードで宣言された関数は、同じファイルで定義された型に自動的にバインドされます:

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

// 関数呼び出し
d = distance(p1, p2)  // 3.606...

// OOP シンタックスシュガー（自動的に Point.distance にバインド）
d2 = p1.distance(p2)  // → distance(p1, p2)
```

#### 列挙型（enum type）

```yaoxiang
// シンプルな列挙型
Color: Type = { red | green | blue }

// データ付き列挙型
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// ジェネリクスの使用
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### ジェネリック型

```yaoxiang
// ジェネリック型定義
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

### パターンマッチング

```yaoxiang
// match 式
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## 並行プログラミング（spawn）

YaoXiang の並行モデルは `spawn <expr>` プリミティブを中心に構築されています。これが唯一の並列エントリポイントです。

```yaoxiang
// spawn は任意の式を修飾し、自動的に並列実行される
main: () -> Void = {
    user = spawn fetch_user(1)   // バックグラウンドで実行
    posts = spawn fetch_posts()  // 並行する別ステップ

    // 結果が必要なときに自動的にブロックして待機
    print(user.name)
    print(posts.length)
}
```

**核心ルール**: `spawn` で修飾された式はバックグラウンドで実行され、外側の同期コードは結果を待機します。依存関係のないタスクは自動的に並列実行され、ランタイムの GMP モデルによってスケジュールされます。

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

### Q: 変数はデフォルトで不変ですが、変数を変更するには？

```yaoxiang
// mut キーワードを使って可変変数を宣言
mut x = 10
x = 20  // ✅ OK
```

### Q: 関数を定義するには？

```yaoxiang
// 完全な形式（推奨）
add: (a: Int, b: Int) -> Int = a + b

// 短い形式（型推論）
add = (a, b) => a + b
```

### Q: エラーを処理するには？

```yaoxiang
// Result 型を使用
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// パターンマッチングで処理
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## 次のステップ

- 📚 完全な構文について[言語仕様](../YaoXiang-language-specification.md)を参照
- 🏗️ 実装の詳細について[アーキテクチャドキュメント](../architecture/)を閲覧
- 💡 核となる理念について[設計マニフェスト](../YaoXiang-design-manifesto.md)を確認

## 関連リソース

- [GitHub リポジトリ](https://github.com/yourusername/yaoxiang)
- [Issue フィードバック](https://github.com/yourusername/yaoxiang/issues)
- [コントリビューションガイド](../guides/dev/)
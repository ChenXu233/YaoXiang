# 名前空間呼び出しサポート

## 概要

`std.module.function` 形式の名前空間呼び出し構文を実装し、`std.io.print` や `std.math.abs` のようなモジュール関数を呼び出せるようにする。

## 現在の状態

- **問題**：`use std.io.*` で短い名前をインポートできるが、`std.io.print` 形式の呼び出しは「Unknown variable: 'std'」エラーを返す
- **期待される動作**：ユーザーが `std.<module>.<function>` 形式で関数を呼び出せること

## 変更が必要なモジュール

### 1. コンパイラフロントエンド - パーサー

ファイル：`src/frontend/parser/`

`a.b.c` 形式の名前式を認識し、名前空間パスを正しく解析できるようにする必要がある。

### 2. コンパイラフロントエンド - 型検査

ファイル：`src/frontend/typecheck/`

名前空間パスに遭遇した場合、以下のことを行う必要がある：

1. `std` を組み込み名前空間として認識する
2. 後続のモジュール名（`io`、`math`、`net` など）を解決する
3. モジュール内の関数を見つけ、型を検証する

### 3. IR生成

ファイル：`src/middle/passes/codegen/`

IR生成時に、名前空間パスを対象関数の参照に変換する必要がある。

### 4. インタプリタ/ランタイム

ファイル：`src/backends/interpreter/executor.rs`

実行時に名前空間パスを正しくFFI handlerに解決できるようにする。

## 実装手順

1. **パーサー変更**：`a.b` 形式のメンバーアクセス式を認識する
2. **意味解析**：名前空間解決ロジックを実装する
3. **コード生成**：正しい関数呼び出し命令を生成する
4. **テスト**：`std.io.print` などの呼び出しを検証するテストケースを追加する

## テストケース

```yaoxiang
use std.io

// 動作する必要がある
std.io.println("Hello")

// 短い名前も動作する必要がある
use std.io.*
println("World")
```

## 関連ファイル

- `src/frontend/parser/` - パーサー
- `src/frontend/typecheck/` - 型検査
- `src/middle/passes/codegen/` - コード生成
- `src/backends/interpreter/` - インタプリタ
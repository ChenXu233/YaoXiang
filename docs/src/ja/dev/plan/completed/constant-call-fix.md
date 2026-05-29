# 定数呼び出し問題修正

## 概要

`std.math.PI` などの定数使用時に `unit` と表示される問題を修正する。

## 現状

- **問題**：`PI` 使用時に期待される浮動小数点数値ではなく `unit` が返される
- **原因**：定数が引数なし関数呼び出しとして扱われているが、FFI handler が正しく実行されていない

## 問題分析

現在のコードでは：

```rust
// FFI登録
registry.register("std.math.PI", |_args| {
    Ok(RuntimeValue::Float(std::f64::consts::PI))
});
```

しかし、定数呼び出し（`PI` など）は関数呼び出しとは異なる命令にコンパイルされている可能性がある。

## 修正が必要なモジュール

### 1. コンパイラ - コード生成

ファイル：`src/middle/passes/codegen/`

定数参照（`PI` など）を native 関数呼び出しとして正しく認識し、対応する bytecode 命令を生成する必要がある。

### 2. インタプリタ/エグゼキュータ

ファイル：`src/backends/interpreter/executor.rs`

定数参照が正しく FFI handler にルーティングされるようにする。

## 実装方針

### 方針 A：translator で定数名を登録

```rust
// src/middle/passes/codegen/translator.rs
// native_functionsに定数を追加
native_functions.insert("std.math.PI".to_string());
native_functions.insert("std.math.E".to_string());
native_functions.insert("std.math.TAU".to_string());
```

### 方針 B：FFIで特殊プレフィックスを使用

定数と関数を区別するために `__const__std.math.PI` のような約束事を使用する。

## テストケース

```yaoxiang
use std.math.*

// 期待出力 3.14159...
println(PI)

// 期待出力 2.71828...
println(E)
```

## 関連ファイル

- `src/middle/passes/codegen/translator.rs` - コード生成
- `src/backends/interpreter/executor.rs` - インタプリタ
- `src/backends/interpreter/ffi.rs` - FFI登録
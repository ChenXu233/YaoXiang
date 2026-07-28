---
title: 'RFC-014b: ビルドシステムとバイナリ配布'
status: '審査中'
author: '晨煦'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
issue: '#91'
impl: '0%'
impl_status: 'not-started'
---

# RFC-014b: ビルドシステムとバイナリ配布

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC です。

## 抄録

YaoXiang パッケージ管理システムのビルドメカニズムを定義する：宣言的ビルド設定、ビルド戦略（cargo/cmake/custom/none）、事前コンパイルバイナリ配布、システム依存関係チェック。

## 動機

一部のパッケージは純粋な `.yx` コードであり、ビルドが不要である。一部は FF
I バインディング（cargo、CMake などの呼び出し）のコンパイルが必要である。パッケージ作者がビルド要件を宣言し、パackage マネージャーが自動的に処理するための統一的なメカニズムが必要である。

### 現在の問題点

- ビルド設定の宣言がない（`yaoxiang.toml` に `[build]` セクションがない）
- 事前コンパイルバイナリ配布メカニズムがない
- FF
I パッケージのビルドは完全にユーザーの手動操作に依存している
- システム依存関係チェックがない

## 提案

### コア設計：宣言的ビルド + 事前コンパイル優先

パッケージ作者は `yaoxiang.toml` でビルド要件を宣言し、パackage マネージャーは宣言に基づいて自動的に意思決定を行う。

### ビルド戦略

```rust
enum BuildStrategy {
    None,          // 純粋な .yx パッケージ、ビルド不要
    Cargo,         // cargo build を呼び出し、[build.cargo] 設定を読み取る
    Cmake,         // cmake を呼び出す
    Custom,        // build.yx スクリプトを実行
}
```

注意：`Precompiled` バリアントは削除された。`[binaries]` の存在は自動的に事前コンパイル優先動作をトリガーするため、明示的な strategy 宣言は不要である。

### yaoxiang.toml でのビルド宣言

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # ビルド戦略
headers = ["include/sqlite3.h"] # 任意：yx-bindgen が自動的に処理する C ヘッダーファイル

[build.cargo]
features = ["ffi"]             # cargo build --features ffi
target = "release"             # cargo build --release

[build.requirements]
cargo = ">= 1.70"              # ビルド時に必要なツール
cmake = ">= 3.20"

[build.platforms]              # プラットフォーム固有の上書き
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### インストール決定ツリー

```
yaoxiang install foo
    │
    ├─ 1. [binaries] に現在のプラットフォームのエントリがあるか？
    │     → あり：ダウンロード、SHA-256 検証、直接インストール（ビルドをスキップ）
    │     → なし：継続
    │
    ├─ 2. ソースパッケージをダウンロード
    │
    ├─ 3. [build].headers に値があるか？
    │     → あり：自動的に yx-bindgen を実行してバインディングファイルを生成
    │
    ├─ 4. [build].strategy を読み取る
    │     → "none"：直接インストール
    │     → "cargo"：[build.cargo] 設定を読み取り、cargo build コマンドを拼む
    │     → "cmake"：cmake を呼び出す
    │     → "custom"：build.yx スクリプトを実行
    │
    └─ 5. vendor/ にインストール
```

**事前コンパイル優先、ソースコードでフォールバック。** `[binaries]` の存在は明示的な strategy なしに事前コンパイルチェックを自動的にトリガーする。

### cargo 戦略の詳細

`strategy = "cargo"` の場合、`[build.cargo]` 設定を読み取り、コマンドを拼む：

```toml
[build]
strategy = "cargo"

[build.cargo]
features = ["ffi"]             # → cargo build --features ffi
target = "release"             # → cargo build --release

[build.platforms]              # プラットフォーム上書き
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

実際に実行されるコマンド：

```bash
# 基本
cargo build --release --features ffi

# プラットフォーム上書きがある場合（Linux を例に）
cargo build --release --features ffi,linux-ffi
```

### 事前コンパイルバイナリ宣言

```toml
# yaoxiang.toml
[binaries]
"x86_64-unknown-linux-gnu" = { url = "releases/download/v1.0.0/foo-linux-x86_64.tar.gz", sha256 = "abc123" }
"x86_64-pc-windows-msvc" = { url = "https://example.com/foo-win-x86_64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-macos-aarch64.tar.gz", sha256 = "ghi789" }
```

**URL 形式：** 絶対 URL と相対パスの両方をサポート。相対パスはパッケージのリポジトリアドレス（GitHub repo URL または Registry ルート URL）相对于える。

**ビルドをスキップする条件：**

1. `[binaries]` に現在のプラットフォームのエントリがある
2. SHA-256 検証が通る
3. ダウンロードに成功する

3つの条件すべてを満たす → ビルドをスキップ。そうでない場合 → ソースコードビルドにフォールバック。

### build.yx ビルドスクリプト

`strategy = "custom"` の場合、`build.yx` を実行する。

**実行モデル（最小仕様）：**

- スクリプトは通常の `.yx` コードであり、完全な `std` アクセス權を持つ
- 作業ディレクトリ：パッケージルート（`vendor/<pkg>-<ver>/`）
- 成功：終了コード 0
- 失敗：非 0 終了コード、インストール中止
- パッケージマネージャーはスクリプト動作を制約せず、終了コードのみをチェック

```yx
# build.yx — パッケージのビルドスクリプト
use std.os
use std.io

fn main() {
    let platform = os.platform()
    let arch = os.arch()

    if os.file_exists("Cargo.toml") {
        io.println("Building native extension via Cargo...")
        let result = os.exec("cargo build --release")
        if result.exit_code != 0 {
            io.println("Build failed!")
            os.exit(1)
        }
    }

    io.println("Build complete!")
}
```

### システム依存関係チェック

インストール前に `[build.requirements]` をすべて自動的にチェックし、満足しない場合はエラー：

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen 統合（headers フィールド）

`[build].headers` は yx-bindgen で処理する C ヘッダーファイルを宣言する。ビルドシステムは自動的に yx-bindgen を実行して `.yx` バインディングファイルを生成する。

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

ビルドフロー：

```
1. [binaries] に事前コンパイルがあるか？→ ビルド全体をスキップ
2. [build].headers に値があるか？→ yx-bindgen が自動的にバインディングを生成
3. [build].strategy を実行（cargo/cmake/custom）
4. インストール
```

yx-bindgen は C ヘッダーファイル（`.h`）から関数シグネチャと型定義を解析し、自動的に `.yx` バインディング宣言を生成する。ユーザーが手動で実行する必要はない——ビルドシステムは `headers` 設定を検出すると自動的に処理する。

**RFC-026 との関係：** RFC-026 は `yx-bindgen` の言語レベルセマンティクス（`native("symbol")` 構文、unsafe 型）を定義している。RFC-014b はビルドフローでの統合方式（`headers` 設定）を定義している。両者は補完関係にある。

### Cargo Workspace との統合

パッケージに FF
I コードがある場合、同時に Cargo workspace を定義できる：

```
my-package/
├── yaoxiang.toml          # YaoXiang パッケージ設定
├── Cargo.toml             # Cargo workspace（FFI 部分）
├── src/
│   └── lib.yx             # YaoXiang コード
└── native/
    ├── Cargo.toml          # Rust FFI コード
    └── src/
        └── lib.rs
```

`yaoxiang build` は自動的に検出し、`cargo build` を呼び出して native 部分をコンパイルする。

## 詳細な設計

### プラットフォーム識別子

Rust target triple 形式（`arch-vendor-os-env`）を使用：

| プラットフォーム              | 識別子                       |
| ---------------------- | --------------------------- |
| Linux x86_64 (glibc)   | `x86_64-unknown-linux-gnu`  |
| Linux x86_64 (musl)    | `x86_64-unknown-linux-musl` |
| Linux ARM64            | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC)  | `x86_64-pc-windows-msvc`    |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu`     |
| macOS ARM64            | `aarch64-apple-darwin`      |
| macOS x86_64           | `x86_64-apple-darwin`       |

簡略化された形式ではなく Rust target triple を使用する理由：

1. 同一 OS 上の異なる AB
I を区別できる（gnu vs musl、msvc vs gnu）
2. Rust/Cargo エコシステムと整合し、マッピングエラーを減らす
3. 将来の拡張で形式を変更する必要がない

### ビルド成果物のディレクトリ構造

```
build/
└── native/
    ├── x86_64-unknown-linux-gnu/
    │   └── libfoo.so
    ├── x86_64-pc-windows-msvc/
    │   └── foo.dll
    └── aarch64-apple-darwin/
        └── libfoo.dylib
```

### 事前コンパイルパッケージの完全なライフサイクル

```
開発者：
  1. .yx コード + FFI バインディングを書く
  2. yaoxiang.toml で [build] + [binaries] を宣言
  3. yaoxiang publish
     → 自動的に CI でマルチプラットフォームバイナリをビルド
     → ソースコード + 事前コンパイル成果物をアップロード

ユーザー：
  yaoxiang add native-foo
    → 事前コンパイル成果物を検出 → 直接ダウンロード（秒単位）
    → 事前コンパイル成果物なし → ソースコードダウンロード + ビルド実行（分単位）
```

## トレードオフ

### メリット

- 宣言的設定により、ユーザーはビルドの詳細を理解する必要がない
- 事前コンパイル優先で、インストール速度が極めて速い
- マルチプラットフォームをサポートし、自動的に選択
- Cargo エコシステムとシームレスに統合

### デメリット

- 事前コンパイル成果物には CI サポートが必要
- マルチプラットフォームビルドがリリースの複雑さを増す
- build.yx スクリプトにはサンドボックスセキュリティメカニズムが必要

## 代替案

| 方案                           | なぜ選択しなかったか                     |
| ------------------------------ | ----------------------------------- |
| 純粋なソースコード配布                     | ユーザーはビルドツールチェーンをインストールする必要があり、ハードルが高い    |
| Python wheel のようなバイナリ形式 | 複雑すぎ、YaoXiang エコシステムの初期段階では不要 |
| FFI ビルドをサポートしない              | 言語の拡張能力を制限する                      |

## 実装戦略

### フェーズ分け

| フェーズ     | 内容                                        |
| -------- | ------------------------------------------- |
| Phase 5a | `[build]` 設定解析 + `BuildStrategy` 列挙型   |
| Phase 5b | システム依存関係チェック                                |
| Phase 5c | Cargo ビルド統合（`[build.cargo]` を読み取りコマンドを拼む） |
| Phase 5d | 事前コンパイルバイナリダウンロード + 検証                     |
| Phase 5e | build.yx スクリプト実行                           |
| Phase 5f | yx-bindgen 統合（`headers` フィールド）           |

### 依存関係

- RFC-014a に依存（Registry プロトコル、事前コンパイル成果物のダウンロードに使用）
- `sha2` crate に依存（完全性検証）

## 開放問題

- [ ] build.yx スクリプトにはサンドボックス分離が必要か？
- [ ] ビルド成果物の最大サイズ制限は？
- [ ] クロスコンパイルをサポートするか（Linux 上で Windows 成果物をビルド）？
- [ ] Cargo バージョンの非互換性はどのように処理するか？

---

## 参考文献

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)

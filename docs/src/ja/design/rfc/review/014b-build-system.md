---
title: "RFC-014b: ビルドシステムとバイナリ配布"
status: "レビュー中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014b: ビルドシステムとバイナリ配布

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC である。

## 概要

YaoXiang パッケージ管理システムのビルド機構を定義する：宣言的ビルド設定、ビルド戦略（cargo/cmake/custom/none）、プリコンパイル済みバイナリ配布、システム依存チェック。

## 動機

純粋な `.yx` コードのみで構成され、ビルド不要なパッケージもあれば、FFI バインディング（呼び出しに Cargo や CMake などを使用するもの）のコンパイルが必要なパッケージもある。パッケージ作者がビルド要件を宣言し、パッケージマネージャーが自動的に処理する統一的な仕組みが必要である。

### 現状の問題点

- ビルド設定の宣言が存在しない（`yaoxiang.toml` に `[build]` セクションがない）
- プリコンパイル済みバイナリの配布機構がない
- FFI パッケージのビルドは完全にユーザーの手動操作に依存している
- システム依存チェックがない

## 提案

### コア設計：宣言的ビルド + プリコンパイル優先

パッケージ作者は `yaoxiang.toml` にビルド要件を宣言し、パッケージマネージャーは宣言に基づいて自動的に判断する。

### ビルド戦略

```rust
enum BuildStrategy {
    None,          // 純粋な .yx パッケージ、ビルド不要
    Cargo,         // cargo build を呼び出し、[build.cargo] 設定を読み取る
    Cmake,         // cmake を呼び出す
    Custom,        // build.yx スクリプトを実行する
}
```

注意：`Precompiled` バリアントは削除済み。`[binaries]` の存在により自動的にプリコンパイル優先動作がトリガーされるため、明示的な strategy の宣言は不要。

### yaoxiang.toml 内のビルド宣言

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # ビルド戦略
headers = ["include/sqlite3.h"] # オプション：yx-bindgen が自動処理する C ヘッダファイル

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

### インストール判断ツリー

```
yaoxiang install foo
    │
    ├─ 1. [binaries] に現在のプラットフォームのエントリがあるか？
    │     → ある：ダウンロード、SHA-256 検証、直接インストール（ビルドをスキップ）
    │     → ない：続行
    │
    ├─ 2. ソースパッケージをダウンロード
    │
    ├─ 3. [build].headers に値があるか？
    │     → ある：yx-bindgen を自動実行してバインディングファイルを生成
    │
    ├─ 4. [build].strategy を読み取る
    │     → "none"：直接インストール
    │     → "cargo"：[build.cargo] 設定を読み取り、cargo build コマンドを組み立て
    │     → "cmake"：cmake を呼び出す
    │     → "custom"：build.yx スクリプトを実行
    │
    └─ 5. vendor/ にインストール
```

**プリコンパイル優先、ソースはフォールバック。** `[binaries]` の存在により自動的にプリコンパイルチェックがトリガーされるため、明示的な strategy は不要。

### cargo 戦略の詳細

`strategy = "cargo"` の場合、`[build.cargo]` 設定を読み取ってコマンドを組み立てる：

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

# プラットフォーム上書きがある場合（linux の例）
cargo build --release --features ffi,linux-ffi
```

### プリコンパイル済みバイナリの宣言

```toml
# yaoxiang.toml
[binaries]
"x86_64-unknown-linux-gnu" = { url = "releases/download/v1.0.0/foo-linux-x86_64.tar.gz", sha256 = "abc123" }
"x86_64-pc-windows-msvc" = { url = "https://example.com/foo-win-x86_64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-macos-aarch64.tar.gz", sha256 = "ghi789" }
```

**URL 形式：** 絶対 URL と相対パスの両方をサポート。相対パスはパッケージのリポジトリアドレス（GitHub repo URL または Registry ルート URL）からの相対パス。

**ビルドをスキップする条件：**
1. `[binaries]` に現在のプラットフォームのエントリがある
2. SHA-256 検証に成功する
3. ダウンロードが成功する

3 つの条件すべてを満たす → ビルドをスキップ。それ以外 → ソースビルドにフォールバック。

### build.yx ビルドスクリプト

`strategy = "custom"` の場合、`build.yx` を実行する。

**実行モデル（最小限の仕様）：**
- スクリプトは通常の `.yx` コードであり、完全な `std` アクセス権を持つ
- 作業ディレクトリ：パッケージルートディレクトリ（`vendor/<pkg>-<ver>/`）
- 成功：終了コード 0
- 失敗：0 以外の終了コード、インストール中止
- パッケージマネージャーはスクリプトの振る舞いを制約せず、終了コードのみをチェックする

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

### システム依存チェック

インストール前にすべての `[build.requirements]` を自動チェックし、満たされない場合はエラーを出力する：

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen 統合（headers フィールド）

`[build].headers` は yx-bindgen が処理する必要のある C ヘッダファイルを宣言する。ビルドシステムが自動的に yx-bindgen を実行し、`.yx` バインディングファイルを生成する。

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

ビルドフロー：

```
1. [binaries] にプリコンパイル済みがあるか？→ ビルド全体をスキップ
2. [build].headers に値があるか？→ yx-bindgen を自動実行してバインディングを生成
3. [build].strategy を実行（cargo/cmake/custom）
4. インストール
```

yx-bindgen は C ヘッダファイル（`.h`）から関数シグネチャと型定義を解析し、`.yx` バインディング宣言を自動生成する。ユーザーが手動で実行する必要はなく、ビルドシステムが `headers` 設定を検出した時点で自動処理される。

**RFC-026 との関係：** RFC-026 は yx-bindgen の言語レベルセマンティクス（`native("symbol")` 構文、unsafe 型）を定義する。RFC-014b はビルドフローにおける統合方法（`headers` 設定）を定義する。両者は補完関係にある。

### Cargo Workspace との統合

パッケージに FFI コードが含まれる場合、Cargo workspace を同時に定義できる：

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

`yaoxiang build` は自動的にこれを検出し、`cargo build` を呼び出して native 部分をコンパイルする。

## 詳細設計

### プラットフォーム識別子

Rust target triple 形式（`arch-vendor-os-env`）を使用：

| プラットフォーム | 識別子 |
|------|------|
| Linux x86_64 (glibc) | `x86_64-unknown-linux-gnu` |
| Linux x86_64 (musl) | `x86_64-unknown-linux-musl` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC) | `x86_64-pc-windows-msvc` |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu` |
| macOS ARM64 | `aarch64-apple-darwin` |
| macOS x86_64 | `x86_64-apple-darwin` |

簡略化形式ではなく Rust target triple を使用する理由：
1. 同一 OS 上の異なる ABI を区別できる（gnu vs musl、msvc vs gnu）
2. Rust/Cargo エコシステムと整合し、マッピングエラーを削減
3. 将来拡張時に形式変更が不要

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

### プリコンパイル済みパッケージの完全なライフサイクル

```
開発者：
  1. .yx コードと FFI バインディングを記述
  2. yaoxiang.toml で [build] と [binaries] を宣言
  3. yaoxiang publish
     → CI 上で自動的にマルチプラットフォームバイナリをビルド
     → ソースとプリコンパイル済み成果物をアップロード

ユーザー：
  yaoxiang add native-foo
    → プリコンパイル済み成果物を検出 → 直接ダウンロード（秒単位）
    → プリコンパイル済み成果物なし → ソースをダウンロード + ビルド実行（分単位）
```

## トレードオフ

### 利点

- 宣言的設定により、ユーザーがビルドの詳細を理解する必要がない
- プリコンパイル優先により、インストール速度が極めて高速
- マルチプラットフォーム対応、自動選択
- Cargo エコシステムとシームレスに統合

### 欠点

- プリコンパイル済み成果物に CI サポートが必要
- マルチプラットフォームビルドにより、リリースの複雑性が増大
- build.yx スクリプトにはサンドボックスによるセキュリティ機構が必要

## 代替案

| 代替案 | 不採用の理由 |
|------|---------|
| 純粋なソース配布 | ユーザーがビルドツールチェーンをインストールする必要があり、参入障壁が高い |
| Python wheel のようなバイナリ形式 | 複雑すぎ、YaoXiang エコシステムの初期段階では不要 |
| FFI ビルド非対応 | 言語の拡張能力を制限する |

## 実装戦略

### フェーズ区分

| フェーズ | 内容 |
|------|------|
| Phase 5a | `[build]` 設定解析 + `BuildStrategy` enum |
| Phase 5b | システム依存チェック |
| Phase 5c | Cargo ビルド統合（`[build.cargo]` を読み取ってコマンドを組み立て） |
| Phase 5d | プリコンパイル済みバイナリのダウンロード + 検証 |
| Phase 5e | build.yx スクリプト実行 |
| Phase 5f | yx-bindgen 統合（`headers` フィールド） |

### 依存関係

- RFC-014a（Registry プロトコル、プリコンパイル済み成果物のダウンロードに使用）に依存
- `sha2` crate（完全性検証）に依存

## 未解決の問題

- [ ] build.yx スクリプトにサンドボックスによる隔離は必要か？
- [ ] ビルド成果物の最大サイズ制限は？
- [ ] クロスコンパイル（Linux 上で Windows 向け成果物をビルド）をサポートするか？
- [ ] Cargo バージョンに互換性がない場合の処理方法は？

---

## 参考文献

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)
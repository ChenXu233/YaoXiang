```markdown
---
title: "RFC-014b: ビルドシステムとバイナリ配布"
status: "ドラフト案"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014b: ビルドシステムとバイナリ配布

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) のサブ RFC です。

## 要約

YaoXiang パッケージ管理システムのビルド機構を定義する：宣言的ビルド構成、ビルド戦略（cargo/cmake/custom/none）、プリコンパイル済みバイナリ配布、システム依存関係チェック。

## 動機

中には純粋な `.yx` コードのみで構成されるパッケージもあれば、コンパイルが必要な FFI バインディング（cargo、cmake などの呼び出し）を必要とするパッケージもある。パッケージ作者がビルド要件を宣言し、パッケージマネージャが自動的に処理する統一的な仕組みが必要である。

### 現状の問題点

- ビルド構成の宣言が存在しない（`yaoxiang.toml` に `[build]` セクションがない）
- プリコンパイル済みバイナリ配布の仕組みがない
- FFI パッケージのビルドが完全にユーザーの手動操作に依存している
- システム依存関係チェックがない

## 提案

### 中核設計：宣言的ビルド + プリコンパイル優先

パッケージ作者は `yaoxiang.toml` にビルド要件を宣言し、パッケージマネージャは宣言に基づいて自動的に判断する。

### ビルド戦略

```rust
enum BuildStrategy {
    None,          // 純粋な .yx パッケージ、ビルド不要
    Cargo,         // cargo build を呼び出す
    Cmake,         // cmake を呼び出す
    Custom,        // build.yx スクリプトを実行する
    Precompiled,   // プリコンパイル済み成果物を直接使用する
}
```

### yaoxiang.toml 内のビルド宣言

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # ビルド戦略
script = "build.yx"            # strategy = "custom" の場合のみ使用

[build.cargo]
features = ["ffi"]             # cargo build --features ffi
target = "release"             # cargo build --release

[build.requirements]
cargo = ">= 1.70"              # ビルド時に必要なツール
cmake = ">= 3.20"

[build.platforms]              # プラットフォーム固有の上書き
"linux-x86_64" = { cargo-features = ["linux-ffi"] }
"windows-x86_64" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### インストール決定木

```
yaoxiang install foo
    │
    ├─ 1. Registry/GitHub Release に現在プラットフォーム向けのプリコンパイル済み成果物があるか？
    │     → ある：ダウンロードし、SHA-256 を検証して直接インストール（ビルドをスキップ）
    │     → ない：続行
    │
    ├─ 2. ソースコードパッケージをダウンロード
    │
    ├─ 3. yaoxiang.toml の [build] セクションを読む
    │     → strategy = "none"：直接インストール
    │     → その他：requirements をチェックし、ビルドを実行
    │
    └─ 4. vendor/ にインストール
```

**プリコンパイル済み優先、ソースコードがフォールバック。** Python の wheel と sdist と同様。

### プリコンパイル済みバイナリの宣言

```toml
# yaoxiang.toml
[binaries]
"linux-x86_64" = { url = "releases/download/v1.0.0/foo-linux-x64.tar.gz", sha256 = "abc123" }
"windows-x86_64" = { url = "releases/download/v1.0.0/foo-win-x64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-mac-arm.tar.gz", sha256 = "ghi789" }
```

**ビルドをスキップする条件：**
1. `[binaries]` に現在プラットフォームのエントリがある
2. SHA-256 検証に成功する
3. ダウンロードが成功する

3 つの条件すべてを満たす → ビルドをスキップ。それ以外 → ソースコードからのビルドにフォールバック。

### build.yx ビルドスクリプト

`strategy = "custom"` の場合、実行される：

```yx
# build.yx — パッケージのビルドスクリプト
use std.os
use std.io

fn main() {
    let platform = os.platform()       # "linux", "windows", "macos"
    let arch = os.arch()               # "x86_64", "aarch64"

    if os.file_exists("Cargo.toml") {
        io.println("Building native extension via Cargo...")
        os.exec("cargo build --release")

        os.copy(
            "target/release/libfoo.so",
            "build/native/${platform}-${arch}/libfoo.so"
        )
    }

    io.println("Build complete!")
}
```

### システム依存関係チェック

インストール前に `[build.requirements]` をすべて自動的にチェックし、満たされない場合はエラーを返す：

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### Cargo Workspace との統合

パッケージに FFI コードが含まれる場合、Cargo workspace を同時に定義できる：

```
my-package/
├── yaoxiang.toml          # YaoXiang パッケージ構成
├── Cargo.toml             # Cargo workspace（FFI 部分）
├── src/
│   └── lib.yx             # YaoXiang コード
└── native/
    ├── Cargo.toml          # Rust FFI コード
    └── src/
        └── lib.rs
```

`yaoxiang build` は自動的に検出し、`cargo build` を呼び出して native 部分をコンパイルする。

## 詳細設計

### プラットフォーム識別子

`{os}-{arch}` 形式を使用する：

| プラットフォーム識別子 | OS | Arch |
|----------|----|------|
| `linux-x86_64` | Linux | x86_64 |
| `linux-aarch64` | Linux | ARM64 |
| `windows-x86_64` | Windows | x86_64 |
| `aarch64-apple-darwin` | macOS | ARM64 (Apple Silicon) |
| `x86_64-apple-darwin` | macOS | x86_64 |

### ビルド成果物ディレクトリ構造

```
build/
└── native/
    ├── linux-x86_64/
    │   └── libfoo.so
    ├── windows-x86_64/
    │   └── foo.dll
    └── aarch64-apple-darwin/
        └── libfoo.dylib
```

### プリコンパイル済みパッケージの完全なライフサイクル

```
開発者：
  1. .yx コードと FFI バインディングを書く
  2. yaoxiang.toml に [build] と [binaries] を宣言する
  3. yaoxiang publish
     → CI 上で自動的にマルチプラットフォームバイナリをビルド
     → ソースコードとプリコンパイル済み成果物をアップロード

ユーザー：
  yaoxiang add native-foo
    → プリコンパイル済み成果物を検出 → 直接ダウンロード（秒単位）
    → プリコンパイル済み成果物がない → ソースコードダウンロード + ビルド実行（分単位）
```

## トレードオフ

### 利点

- 宣言的構成により、ユーザーがビルドの詳細を理解する必要がない
- プリコンパイル済み優先により、インストール速度が極めて高速
- マルチプラットフォーム対応、自動選択
- Cargo エコシステムとシームレスに統合

### 欠点

- プリコンパイル済み成果物に CI サポートが必要
- マルチプラットフォームビルドがリリースの複雑性を増す
- build.yx スクリプトにサンドボックスセキュリティ機構が必要

## 代替案

| 代替案 | 採用しなかった理由 |
|------|-----------|
| 純粋なソースコード配布のみ | ユーザーがビルドツールチェーンをインストールする必要があり、敷居が高い |
| Python wheel 風のバイナリ形式 | 複雑すぎ、YaoXiang エコシステムの初期段階では不要 |
| FFI ビルドをサポートしない | 言語の拡張能力を制限する |

## 実装戦略

### フェーズ分割

| フェーズ | 内容 |
|------|------|
| Phase 5a | `[build]` 構成解析 + `BuildStrategy` enum |
| Phase 5b | システム依存関係チェック |
| Phase 5c | Cargo ビルド統合 |
| Phase 5d | プリコンパイル済みバイナリのダウンロード + 検証 |
| Phase 5e | build.yx スクリプト実行 |

### 依存関係

- RFC-014a（Registry プロトコル、プリコンパイル済み成果物のダウンロード用）に依存
- `sha2` crate（整合性検証用）に依存

## 未解決の問題

- [ ] build.yx スクリプトにサンドボックス分離は必要か？
- [ ] ビルド成果物の最大サイズ制限は？
- [ ] クロスコンパイル（Linux 上で Windows 用成果物をビルド）をサポートするか？
- [ ] Cargo バージョン非互換時の処理方法は？

---

## 参考文献

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)
```
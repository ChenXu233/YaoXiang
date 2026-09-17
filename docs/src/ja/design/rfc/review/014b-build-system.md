---
title: 'RFC-014b: ビルドシステムとバイナリ配布'
status: 'レビュー中'
author: '晨煦'
created: '2026-06-11'
updated: '2026-09-15'
group: 'rfc-014'
issue: '#91'
impl: '0%'
impl_status: 'not-started'
---

# RFC-014b: ビルドシステムとバイナリ配布

> 本 RFC は [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
> のサブ RFC である。

## 2026-09-15 レビュー決議

以下の決定は所有者により 2026-09-15 に下された：

1. **build.yx トラストゲート（「サンドボックス」未解決問題への落とし込み）**：`custom`
   戦略が任意のコードを実行することは、パッケージマネージャにとって最大のサプライチェーン攻撃面である——`yaoxiang add`
   で悪意あるパッケージを追加することは `std.os` の全権限を渡すのと同義である。最低限の強制ライン：
   - あるパッケージの `build.yx` を初めて実行する前にインタラクティブ確認が必須；
   - `yaoxiang add --trust <pkg>` / `install --trust` で信頼記録を `~/.yaoxiang/config.toml`
     に永続化する（`[trust] build-scripts = ["name@version"]`）；
   - 非インタラクティブ環境（CI）はデフォルトで `custom` ビルドを拒否する（明示的な `--trust`
     を除く）；
   - 完全なサンドボックス機構は未解決問題として研究を継続するが、トラストゲートは省略不可の下限である。
2. **フェーズ再編**：`5a → 5b → 5c（cargo）→ 5d（[binaries]）→ 5f（bindgen、RFC-026b に依存）→ 5e（custom は最後、トラストゲート付き）`。宣言的な経路（cargo）を先行し、任意のコード実行は最後とする。
3. **バイナリ配布の統一**：`[binaries]`
   は唯一のバイナリ配布機構である（RFC-014a 決定 3 に対応——`.yxpkg`
   にはソースコードのみが含まれる）。
4. **クロスコンパイル**：初期はサポートせず、マルチプラットフォーム成果物は CI の複数ホストで生成する（RFC-037 の cargo-dist の考え方と整合）。
5. **ビルド成果物のサイズ**：上限は設けない。配布チャネルが自主管理する。
6. **Cargo バージョン非互換**：`[build.requirements]`
   プレチェックエラー + インストールガイドで対応する（本文で定義済み、追加機構なし）。

## 概要

YaoXiang パッケージ管理システムのビルド機構を定義する：宣言的ビルド設定、ビルド戦略（cargo/cmake/custom/none）、事前コンパイル済みバイナリ配布、システム依存チェック。

## 動機

パッケージには純粋な `.yx` コードでビルド不要なものもあれば、FFI バインディング（`std.os`
経由の Cargo や CMake などの呼び出し）のコンパイルが必要なものもある。パッケージ作者がビルド要件を宣言し、パッケージマネージャが自動的に処理する統一的な機構が必要である。

### 現在の問題

- ビルド設定の宣言がない（`yaoxiang.toml` に `[build]` セクションがない）
- 事前コンパイル済みバイナリ配布機構がない
- FFI パッケージのビルドは完全にユーザーの手動操作に依存している
- システム依存チェックがない

## 提案

### 中核設計：宣言的ビルド + 事前コンパイル優先

パッケージ作者は `yaoxiang.toml`
にビルド要件を宣言し、パッケージマネージャは宣言に基づいて自動的に判断する。

### ビルド戦略

```rust
enum BuildStrategy {
    None,          // 純 .yx パッケージ、ビルド不要
    Cargo,         // cargo build を呼び出し、[build.cargo] 設定を読む
    Cmake,         // cmake を呼び出す
    Custom,        // build.yx スクリプトを実行
}
```

注意：`Precompiled` バリアントは削除された。`[binaries]`
の存在により事前コンパイル優先動作が自動的にトリガーされ、strategy の明示的な宣言は不要。

### yaoxiang.toml でのビルド宣言

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # ビルド戦略
headers = ["include/sqlite3.h"] # 任意：yx-bindgen が自動処理する C ヘッダ

[build.cargo]
features = ["ffi"]             # cargo build --features ffi
target = "release"             # cargo build --release

[build.requirements]
cargo = ">= 1.70"              # ビルド時に必要なツール
cmake = ">= 3.20"

[build.platforms]              # プラットフォーム固有のオーバーライド
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### インストール決定木

```
yaoxiang install foo
    │
    ├─ 1. [binaries] に現在のプラットフォームのエントリがあるか？
    │     → ある場合：ダウンロード、SHA-256 検証、直接インストール（ビルドをスキップ）
    │     → ない場合：続行
    │
    ├─ 2. ソースパッケージをダウンロード
    │
    ├─ 3. [build].headers に値があるか？
    │     → ある場合：自動的に yx-bindgen を実行してバインディングファイルを生成
    │
    ├─ 4. [build].strategy を読む
    │     → "none"：直接インストール
    │     → "cargo"：[build.cargo] 設定を読み、cargo build コマンドを組み立てる
    │     → "cmake"：cmake を呼び出す
    │     → "custom"：build.yx スクリプトを実行
    │
    └─ 5. vendor/ にインストール
```

**事前コンパイル優先、ソースコードでフォールバック。** `[binaries]`
の存在により事前コンパイルチェックが自動的にトリガーされ、明示的な strategy は不要。

### cargo 戦略の詳細

`strategy = "cargo"` のとき、`[build.cargo]` 設定を読んでコマンドを組み立てる：

```toml
[build]
strategy = "cargo"

[build.cargo]
features = ["ffi"]             # → cargo build --features ffi
target = "release"             # → cargo build --release

[build.platforms]              # プラットフォームオーバーライド
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

実際に実行されるコマンド：

```bash
# 基本
cargo build --release --features ffi

# プラットフォームオーバーライドがある場合（linux の例）
cargo build --release --features ffi,linux-ffi
```

### 事前コンパイル済みバイナリ宣言

```toml
# yaoxiang.toml
[binaries]
"x86_64-unknown-linux-gnu" = { url = "releases/download/v1.0.0/foo-linux-x86_64.tar.gz", sha256 = "abc123" }
"x86_64-pc-windows-msvc" = { url = "https://example.com/foo-win-x86_64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-macos-aarch64.tar.gz", sha256 = "ghi789" }
```

**URL 形式：**
絶対 URL と相対パスの両方をサポート。相対パスはパッケージのリポジトリアドレス（GitHub リポジトリ URL または Registry ルート URL）からの相対。

**ビルドをスキップする条件：**

1. `[binaries]` に現在のプラットフォームのエントリがある
2. SHA-256 検証が通る
3. ダウンロードが成功する

3 つの条件すべてが満たされる → ビルドをスキップ。それ以外 → ソースコードビルドにフォールバック。

### build.yx ビルドスクリプト

`strategy = "custom"` のとき `build.yx` を実行する。

**実行モデル（最小仕様）：**

- スクリプトは通常の `.yx` コードであり、完全な `std` アクセス権限を持つ
- **トラストゲート（2026-09-15 決定、強制）**：初めて実行する前にインタラクティブ確認が必須であり、非インタラクティブ環境ではデフォルトで拒否する（上記決定 1 参照）
- 作業ディレクトリ：パッケージルートディレクトリ（`vendor/<pkg>-<ver>/`）
- 成功：終了コード 0
- 失敗：非 0 終了コード、インストール中止
- パッケージマネージャはスクリプトの挙動を制約せず、終了コードのみをチェックする

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

インストール前にすべての `[build.requirements]`
を自動的にチェックし、満たされない場合はエラーを報告する：

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen 統合（headers フィールド）

`[build].headers`
は yx-bindgen が処理する必要のある C ヘッダファイルを宣言する。ビルドシステムは自動的に yx-bindgen を実行して
`.yx` バインディングファイルを生成する。

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

ビルドフロー：

```
1. [binaries] に事前コンパイル済み成果物があるか？ → ビルド全体をスキップ
2. [build].headers に値があるか？ → yx-bindgen を自動的に実行してバインディングを生成
3. [build].strategy（cargo/cmake/custom）を実行
4. インストール
```

yx-bindgen は C ヘッダファイル（`.h`）から関数シグネチャと型定義を解析し、`.yx`
バインディング宣言を自動的に生成する。ユーザーが手動で実行する必要はない。ビルドシステムが `headers`
設定を検出した時点で自動的に処理する。

**RFC-026 との関係：** RFC-026 は yx-bindgen の言語レベルのセマンティクス（`native("symbol")`
構文、unsafe 型）を定義する。RFC-014b はビルドフローにおけるその統合方式（`headers`
設定）を定義する。両者は補完的である。

### Cargo ワークスペースとの統合

パッケージに FFI コードが含まれる場合、Cargo ワークスペースを同時に定義できる：

```
my-package/
├── yaoxiang.toml          # YaoXiang パッケージ設定
├── Cargo.toml             # Cargo ワークスペース（FFI 部分）
├── src/
│   └── lib.yx             # YaoXiang コード
└── native/
    ├── Cargo.toml          # Rust FFI コード
    └── src/
        └── lib.rs
```

`yaoxiang build` は自動的に検出し、`cargo build` を呼び出してネイティブ部分をコンパイルする。

## 詳細設計

### プラットフォーム識別子

Rust target triple 形式（`arch-vendor-os-env`）を使用する：

| プラットフォーム       | 識別子                      |
| ---------------------- | --------------------------- |
| Linux x86_64 (glibc)   | `x86_64-unknown-linux-gnu`  |
| Linux x86_64 (musl)    | `x86_64-unknown-linux-musl` |
| Linux ARM64            | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC)  | `x86_64-pc-windows-msvc`    |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu`     |
| macOS ARM64            | `aarch64-apple-darwin`      |
| macOS x86_64           | `x86_64-apple-darwin`       |

Rust target triple を簡略化した形式ではなく使用する理由は以下の通り：

1. 同一 OS 上の異なる ABI を区別する（gnu vs musl、msvc vs gnu）
2. Rust/Cargo エコシステムと整合し、マッピングエラーを削減する
3. 将来の拡張で形式変更が不要

### ビルド成果物ディレクトリ構造

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

### 事前コンパイル済みパッケージの完全なライフサイクル

```
開発者：
  1. .yx コード + FFI バインディングを作成
  2. yaoxiang.toml に [build] + [binaries] を宣言
  3. yaoxiang publish
     → CI 上で自動的にマルチプラットフォームバイナリをビルド
     → ソースコード + 事前コンパイル済み成果物をアップロード

ユーザー：
  yaoxiang add native-foo
    → 事前コンパイル済み成果物を検出 → 直接ダウンロード（秒単位）
    → 事前コンパイル済み成果物なし → ソースコードダウンロード + ビルド実行（分単位）
```

## トレードオフ

### 利点

- 宣言的設定により、ユーザーはビルド詳細を理解する必要がない
- 事前コンパイル優先でインストール速度が極めて高速
- マルチプラットフォーム対応、自動的に選択
- Cargo エコシステムとシームレスに統合

### 欠点

- 事前コンパイル済み成果物には CI サポートが必要
- マルチプラットフォームビルドがリリースの複雑性を増す
- build.yx スクリプトにはサンドボックスセキュリティ機構が必要

## 代替案

| 案                                | 採用しなかった理由                                               |
| --------------------------------- | ---------------------------------------------------------------- |
| 純粋なソースコード配布            | ユーザーはビルドツールチェインのインストールが必要で、敷居が高い |
| Python wheel のようなバイナリ形式 | 複雑すぎ、YaoXiang エコシステム初期には不要                      |
| FFI ビルド非対応                  | 言語の拡張能力を制限する                                         |

## 実装戦略

### フェーズ区分

| フェーズ | 内容                                                             |
| -------- | ---------------------------------------------------------------- |
| Phase 5a | `[build]` 設定解析 + `BuildStrategy` enum                        |
| Phase 5b | システム依存チェック                                             |
| Phase 5c | Cargo ビルド統合（`[build.cargo]` を読んでコマンドを組み立てる） |
| Phase 5d | 事前コンパイル済みバイナリのダウンロード + 検証                  |
| Phase 5f | yx-bindgen 統合（`headers` フィールド、RFC-026b に依存）         |
| Phase 5e | build.yx スクリプト実行（**最後**に実装、トラストゲート付き）    |

実行順序（2026-09-15 決定 2）：`5a → 5b → 5c → 5d → 5f → 5e`。宣言的ビルドを先行し、任意のコード実行は最後。

### 依存関係

- RFC-014a（事前コンパイル済み成果物のダウンロード用 Registry プロトコル）に依存
- `sha2` クレート（整合性検証）に依存

## 未解決の問題

- [x] build.yx スクリプトにはサンドボックス隔離が必要か？→ トラストゲートを強制下限とする（2026-09-15 決定 1）；完全なサンドボックスは研究を継続
- [x] ビルド成果物の最大サイズ制限は？→ 上限なし。チャネルが自主管理（2026-09-15 決定 5）
- [x] クロスコンパイル（Linux 上で Windows 成果物をビルド）をサポートするか？→ 初期はサポートせず、CI の複数ホストで生成（2026-09-15 決定 4）
- [x] Cargo バージョン非互換時の処理は？→ `[build.requirements]`
      プレチェックエラー + インストールガイド（2026-09-15 決定 6）

---

## 参考文献

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)

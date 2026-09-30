---
title: 'RFC-014b: ビルドシステムとバイナリ配布'
status: '承認済み'
author: '晨煦'
created: '2026-06-11'
updated: '2026-09-30'
group: 'rfc-014'
issue: '#91'
impl: '90%'
impl_status: '進行中'
---

# RFC-014b: ビルドシステムとバイナリ配布

> 本RFCは [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
> のサブRFCである。

## 2026-09-15 審査決議

以下は所有者により2026-09-15に決定された：

1. **build.yx 信頼ゲート（「サンドボックス」未解決問題への着地）**：`custom`
   戦略が任意のコードを実行することは、パッケージマネージャにとって最大のサプライチェーン攻撃面である——悪意あるパッケージを
   `yaoxiang add` することは `std.os` の全権限を渡すことと同義である。最低限の強制ライン：
   - あるパッケージの `build.yx` を初めて実行する前に必ず対話で確認；
   - `yaoxiang add --trust <pkg>` / `install --trust` で信頼記録を
     `~/.yaoxiang/config.toml`（`[trust] build-scripts = ["name@version"]`）に永続化；
   - 非対話環境（CI）は明示的な `--trust` が無ければデフォルトで `custom` ビルドを拒否；
   - 完全なサンドボックス機構は引き続き未解決問題として研究するが、信頼ゲートは省略不可の最低ラインである。
2. **フェーズ並べ替え**：`5a → 5b → 5c（cargo）→ 5d（[binaries]）→ 5f（bindgen、RFC-026bに依存）→ 5e（customは最後、信頼ゲート付き）`。宣言的パス（cargo）を先行し、任意コード実行は最後とする。
3. **バイナリ配布の統一**：`[binaries]` が唯一のバイナリ配布機構（RFC-014a 決議3に対応——`.yxpkg`
   はソースのみ）。
4. **クロスコンパイル**：初期は非対応、複数プラットフォーム成果物はCIで複数ホストから出力する（RFC-037
   cargo-dist と考え方を合わせる）。
5. **ビルド成果物サイズ**：上限なし、配布チャネル側で管理する。
6. **Cargoバージョン非互換**：`[build.requirements]`
   プレチェックでエラー報告 + インストールガイド（本文で定義済み、追加機構なし）。

## 概要

YaoXiang パッケージ管理システムのビルド機構を定義する：宣言的ビルド構成、ビルド戦略（cargo/cmake/custom/none）、コンパイル済みバイナリ配布、システム依存チェック。

## 動機

一部のパッケージは純粋な `.yx`
コードであり、ビルド不要である。一部は FFI バインディングのコンパイルが必要（cargo、cmake 等の呼び出し）。パッケージ作者がビルド要件を宣言し、パッケージマネージャが自動処理する統一機構が必要である。

### 現状の問題

- ビルド構成宣言がない（`yaoxiang.toml` に `[build]` セクションがない）
- コンパイル済みバイナリ配布機構がない
- FFI パッケージのビルドが完全にユーザー手動操作に依存している
- システム依存チェックがない

## 提案

### コア設計：宣言的ビルド + コンパイル済み優先

パッケージ作者は `yaoxiang.toml`
でビルド要件を宣言し、パッケージマネージャは宣言に基づいて自動決定する。

### ビルド戦略

```rust
enum BuildStrategy {
    None,          // 純粋な .yx パッケージ、ビルド不要
    Cargo,         // cargo build を呼び出し、[build.cargo] 設定を読む
    Cmake,         // cmake を呼び出す
    Custom,        // build.yx スクリプトを実行
}
```

注意：`Precompiled` バリアントは削除済みである。`[binaries]`
の存在が自動的にコンパイル済み優先動作をトリガーし、明示的な strategy 宣言は不要。

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

### インストール決定木

```
yaoxiang install foo
    │
    ├─ 1. [binaries] に現在のプラットフォームのエントリあり？
    │     → あり：ダウンロード、SHA-256 検証、直接インストール（ビルドをスキップ）
    │     → なし：続行
    │
    ├─ 2. ソースパッケージをダウンロード
    │
    ├─ 3. [build].headers に値あり？
    │     → あり：yx-bindgen を自動実行してバインディングファイルを生成
    │
    ├─ 4. [build].strategy を読む
    │     → "none"：直接インストール
    │     → "cargo"：[build.cargo] 設定を読み、cargo build コマンドを組み立て
    │     → "cmake"：cmake を呼び出す
    │     → "custom"：build.yx スクリプトを実行
    │
    └─ 5. vendor/ にインストール
```

**コンパイル済み優先、ソースはフォールバック。** `[binaries]`
の存在が自動的にコンパイル済みチェックをトリガーし、明示的な strategy は不要。

### cargo 戦略詳細

`strategy = "cargo"` の場合、`[build.cargo]` 設定からコマンドを組み立てる：

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

# プラットフォーム上書きありの場合（linux の例）
cargo build --release --features ffi,linux-ffi
```

### コンパイル済みバイナリ宣言

```toml
# yaoxiang.toml
[binaries]
"x86_64-unknown-linux-gnu" = { url = "releases/download/v1.0.0/foo-linux-x86_64.tar.gz", sha256 = "abc123" }
"x86_64-pc-windows-msvc" = { url = "https://example.com/foo-win-x86_64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-macos-aarch64.tar.gz", sha256 = "ghi789" }
```

**URL 形式：**
絶対 URL と相対パスの両方をサポートする。相対パスはパッケージのリポジトリアドレス（GitHub repo
URL または Registry ルート URL）からの相対。

**ビルドをスキップする条件：**

1. `[binaries]` に現在のプラットフォームのエントリがある
2. SHA-256 検証が成功
3. ダウンロード成功

3 条件すべて満たす → ビルドをスキップ。それ以外 → ソースビルドにフォールバック。

### build.yx ビルドスクリプト

`strategy = "custom"` の場合、`build.yx` を実行する。

**実行モデル（最小仕様）：**

- スクリプトは通常の `.yx` コードで、完全な `std` アクセス権限を持つ
- **信頼ゲート（2026-09-15 決議、強制）**：初回実行前に必ず対話で確認、非対話環境はデフォルトで拒否（上記決議 1 を参照）
- 作業ディレクトリ：パッケージルートディレクトリ（`vendor/<pkg>-<ver>/`）
- 成功：終了コード 0
- 失敗：非 0 終了コード、インストール中止
- パッケージマネージャはスクリプトの挙動を制約せず、終了コードのみ確認する

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

インストール前にすべての `[build.requirements]` を自動チェックし、満たさなければエラー：

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen 統合（headers フィールド）

`[build].headers`
は yx-bindgen が処理する C ヘッダファイルを宣言する。ビルドシステムが自動的に yx-bindgen を実行して
`.yx` バインディングファイルを生成する。

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

ビルドフロー：

```
1. [binaries] にコンパイル済みあり？→ ビルド全体をスキップ
2. [build].headers に値あり？→ yx-bindgen が自動的にバインディングを生成
3. [build].strategy（cargo/cmake/custom）を実行
4. インストール
```

yx-bindgen は C ヘッダファイル（`.h`）から関数シグネチャと型定義を解析し、自動的に `.yx`
バインディング宣言を生成する。ユーザーが手動で実行する必要はない——ビルドシステムが `headers`
設定を検出した時点で自動処理する。

**RFC-026 との関係：** RFC-026 は `yx-bindgen` の言語レベルセマンティクス（`native("symbol")`
構文、unsafe 型）を定義する。RFC-014b はビルドフローにおける統合方法（`headers`
設定）を定義する。両者は補完的である。

### Cargo Workspace との統合

パッケージ内に FFI コードがある場合、Cargo workspace も同時に定義可能：

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

`yaoxiang build` は自動的に検出して `cargo build` を呼び出し、native 部分をコンパイルする。

## 詳細設計

### プラットフォーム識別子

Rust target triple 形式（`arch-vendor-os-env`）を使用：

| プラットフォーム       | 識別子                      |
| ---------------------- | --------------------------- |
| Linux x86_64 (glibc)   | `x86_64-unknown-linux-gnu`  |
| Linux x86_64 (musl)    | `x86_64-unknown-linux-musl` |
| Linux ARM64            | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC)  | `x86_64-pc-windows-msvc`    |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu`     |
| macOS ARM64            | `aarch64-apple-darwin`      |
| macOS x86_64           | `x86_64-apple-darwin`       |

Rust target triple 形式を使う理由（簡略形式ではなく）：

1. 同一 OS 上の異なる ABI を区別する（gnu vs musl、msvc vs gnu）
2. Rust/Cargo エコシステムと整合し、マッピングエラーを削減
3. 将来拡張時にフォーマット変更不要

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

### コンパイル済みパッケージの完全なライフサイクル

```
開発者：
  1. .yx コード + FFI バインディングを書く
  2. yaoxiang.toml で [build] + [binaries] を宣言
  3. yaoxiang publish
     → CI で自動的にマルチプラットフォームバイナリをビルド
     → ソース + コンパイル済み成果物をアップロード

ユーザー：
  yaoxiang add native-foo
    → コンパイル済み成果物を検出 → 直接ダウンロード（秒単位）
    → コンパイル済み成果物なし → ソースをダウンロード + ビルド実行（分単位）
```

## トレードオフ

### 利点

- 宣言的構成で、ユーザーがビルド詳細を理解する必要がない
- コンパイル済み優先により、インストール速度が極めて高速
- マルチプラットフォーム対応で自動選択
- Cargo エコシステムとシームレスに統合

### 欠点

- コンパイル済み成果物に CI サポートが必要
- マルチプラットフォームビルドがリリースの複雑性を増大させる
- build.yx スクリプトにサンドボックスセキュリティ機構が必要

## 代替案

| 案                            | 採用しなかった理由                                                     |
| ----------------------------- | ---------------------------------------------------------------------- |
| 純粋なソース配布              | ユーザーがビルドツールチェーンをインストールする必要があり、敷居が高い |
| Python wheel 風のバイナリ形式 | 複雑すぎる、YaoXiang エコシステム初期には不要                          |
| FFI ビルド非対応              | 言語の拡張能力を制限する                                               |

## 実装戦略

### フェーズ分割

| フェーズ | 内容                                                                                                            | 状態                                                  |
| -------- | --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Phase 5a | `[build]` 設定解析 + `BuildStrategy` enum（`[binaries]` 宣言、プラットフォームトリプルを含む）                  | ✅ 完了                                               |
| Phase 5b | システム依存チェック（`<tool> --version` 検出 + コンパレータオペランド補完 + インストールガイド）               | ✅ 完了                                               |
| Phase 5c | Cargo ビルド統合（`[build.cargo]` からコマンド組み立て + プラットフォーム上書きマージ + scratch 分離）          | ✅ 完了                                               |
| Phase 5d | コンパイル済みバイナリダウンロード + 検証（パッケージ全体 SHA-256 + 安全な展開 + フォールバックセマンティクス） | ✅ 完了                                               |
| Phase 5f | yx-bindgen 統合（`headers` フィールド設定パイプライン；RFC-026b まだドラフト、実行時に明確なエラー）            | ✅ 設定パイプライン完了（ジェネレータは 026b と共に） |
| Phase 5e | build.yx スクリプト実行（**最後に実装**、信頼ゲート付き）                                                       | ✅ 完了                                               |

実行順序（2026-09-15 決議 2）：`5a → 5b → 5c → 5d → 5f → 5e`。宣言的ビルドを先行し、任意コード実行を最後とする。

**実装メモ（2026-09-30、コミットは feat/rfc014 を参照）**：

- **成果物 2 層分離**：cargo scratch（target/）は `CARGO_TARGET_DIR` でプロジェクト
  `.yaoxiang/build/cargo/<pkg>/`
  に指定する——vendor パッケージディレクトリに落とさない。さもないとディレクトリ完全性チェックサムが増分ビルド成果物で破裂する。FFI が消費可能なライブラリファイル（.so/.dll/.dylib/.a）は vendor パッケージディレクトリの
  `build/native/<triple>/` にコピー。vendor 完全性チェックサムセマンティクスは
  **ソースツリー完全性**（`build/` 派生の成果物はチェックサムに入らない）に明確化する。
- **`[binaries]`
  フォールバックセマンティクス**：現在のプラットフォームにエントリあるが「sha256 未宣言 / ダウンロード失敗 / 検証不一致」いずれか →
  stderr 通知 + ソースビルドへフォールバック（RFC「それ以外はフォールバック」）、フォールバック前に半完成品をクリーンアップ。パッケージ全体検証通過後、安全な展開を実行（パスエスケープ防御 + 展開総量ガード 512
  MiB、成果物サイズにハード上限なし——決議 5）。
- **信頼ゲート（決議 1）実装**：信頼記録はユーザー構成体系（`~/.config/yaoxiang/config.toml` の
  `[trust] build-scripts`；RFC ドラフト時の `~/.yaoxiang/config.toml` は存在せず、`[cache] dir`
  と同様に統合）に紐付ける。3 つのパス許可：記録済み / 今回
  `--trust`（**許可＝即永続化**、「add/install
  --trust で信頼記録を永続化」を実現）/ 対話確認（確認＝永続化）。非対話環境（stdin が非ターミナル）はデフォルトで拒否し、`--trust`
  のみ通過可能。
- **build.yx 実行モデル**：通常の .yx スクリプト、**トップレベル文がビルドロジック**（Script/eval セマンティクス；上記
  `fn main()`
  例はドラフト期の擬似コード）、インプロセス実行、作業ディレクトリは一時的にパッケージルートへ切り替え（グローバルロックでシリアライズ）。現在の std に exec/exit
  API なし——スクリプトは生成/ファイル系ロジックを記述可能で、`os.exec`
  形式の外部ツール呼び出しは std 進化と共に提供される。
- **cmake 戦略**：enum と設定解析は準備済み、実行は未実装（明確にエラー報告）——RFC フェーズ表に cmake フェーズを単独記載していないため、実需要パッケージ登場時にスケジュールする。
- **publish 公開前テスト**（014a 検証 3）本 Phase で配線済み：デフォルトで `[tool.test]`
  検出テストを実行（RFC-036 機構）、失敗で公開中止、`--no-test` でスキップ。

### 依存関係

- RFC-014a に依存（コンパイル済み成果物ダウンロード用 Registry プロトコル）
- `sha2` crate に依存（完全性検証）

## オープン問題

- [x] build.yx スクリプトにサンドボックス分離は必要か？→ 信頼ゲートが強制下限（2026-09-15 決議 1）；完全サンドボックスは引き続き研究
- [x] ビルド成果物の最大サイズ制限は？→ 上限なし、チャネル側で管理（2026-09-15 決議 5）
- [x] クロスコンパイル対応（Linux で Windows 成果物をビルド）？→ 初期非対応、CI マルチホスト出力（2026-09-15 決議 4）
- [x] Cargo バージョン非互換時の処理は？→ `[build.requirements]`
      プレチェックでエラー + インストールガイド（2026-09-15 決議 6）

---

## 参考文献

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)

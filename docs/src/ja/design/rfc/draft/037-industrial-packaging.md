---
title: 'RFC-037: 産業化配布方案 — cargo-dist ベースのコンパイラ/ツールチェーンパッケージング'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-07-27'
issue: '#230'
---

# RFC-037: 産業化配布方案 — cargo-dist ベースのコンパイラ/ツールチェーンパackageング

> 本 RFC は [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
> と互补的な関係にあります。RFC-014b では **YaoXiang パッケージマネージャー**
> が第三方パッケージをどのように構築・配布するかを定義していますが、本 RFC では
> **YaoXiang コンパイラ/ツールチェーン自体** をどのようにPackageング・配布するかを定義します。

## 概要

手書きの CI ビルド/Packageングロジックを
`cargo-dist`（Rust エコシステムのバイナリ配布標準ツール）に置き換え、クロスプラットフォームの自動化リリースを実現します。`libz3.dll`
の欠落、標準ライブラリインターフェースファイルの未Packageング、ディレクトリ構造の混乱、CI スクリプトの重複保守などの問題を解決します。

## 動機

### この機能が必要な理由

YaoXiang をダウンロードしたユーザーは、追加の手順なしで**すぐに使える**必要があります。

### 現在の問題

#### 問題 1：Windows ユーザーがダウンロード後に実行できない

現在の Release では `yaoxiang.exe` のみをアップロードしていますが、`libz3.dll`
はPackage内に含まれていません。Windowsユーザーがダブルクリックで実行すると、次のようなエラーが発生します：

```
The code execution cannot proceed because libz3.dll was not found.
```

これは **致命的なバグ** です—ユーザーは最初のステップすら踏めません。

#### 問題 2：Release 成果物が単一ファイルの exe のみ

```
yaoxiang-v0.7.10-x86_64-pc-windows-msvc.zip
└── yaoxiang.exe
```

標準ライブラリインターフェースファイル（`.yx`
ファイル、LSP に必要）が配布Packageに含まれていません。ユーザーは `yaoxiang package init`
を実行して初めて生成できます。産業化の做法では、配布Packageには標準ライブラリが付属しているべきです。

#### 問題 3：CI 手書きスクリプトの重複保守

現在、4 セットのビルドパイプラインを保守しています：

| ファイル                  | 責務                         | 行数        |
| ------------------------- | ---------------------------- | ----------- |
| `_build-platforms.yml`    | クロスプラットフォームビルド | ~255 行     |
| `release.yml`             | バージョンリリース           | ~176 行     |
| `nightly.yml`             | デイリービルド               | ~145 行     |
| `_build-wasm.yml`         | Wasm ビルド                  | ~75 行      |
| `scripts/build/setup.iss` | Inno Setup インストーラー    | ~250 行     |
| **合計**                  |                              | **~900 行** |

大部分は重複しています（Rust のインストール → キャッシュ → ビルド → リネーム → アップロード）。各プラットフォームごとに一度ずつ記述する必要があります。`cargo-dist`
なら 1 行のコマンドで同等のパイプラインを生成できます。

#### 問題 4：Inno Setup のバージョン番号がハードコードされている

`setup.iss` 内の `MyAppVersion` は `0.7.0` で固定されており、ビルド時に `sed`
で置き換えています。これは迟早破綻します。

#### 問題 5：RFC-014b との境界が曖昧

RFC-014b では"YaoXiang パッケージのビルドと配布メカニズム"（`yaoxiang.toml` の `[build]` と
`[binaries]`
設定）を定義していますが、**"YaoXiang コンパイラ自体をどのようにリリースするか"はカバーしていません**。本 RFC はこの空白を埋めます。

## 提案

### 中核設計

**cargo-dist**
をリリースパイプラインの骨格として採用し、カスタムの post-build スクリプトでPackage構造と追加ファイルを処理します。

```
cargo-dist の責務:
  ├── クロスプラットフォームコンパイル（6 つの target）
  ├── CI パイプラインの生成（手書き ~900 行 YAML を置換）
  ├── インストーラーの生成（MSI / shell / powershell / homebrew）
  ├── npm 公開（@yaoxiang/cli — バイナリダウンロードラッパー）
  ├── checksum + 署名
  └── GitHub Release へのアップロード

build.rs が引き続き担当:
  └── Z3 のダウンロード/リンク（既存のロジック、全プラットフォーム動的リンクに変更）

YaoXiang カスタムスクリプト（package-dist.sh）が担当:
  ├── ビルド後の zip 構造再構成（bin/ + lib/）
  ├── 共有ライブラリの添付（libz3.so / dylib / dll）
  └── 事前生成された標準ライブラリ .yx インターフェースファイル
```

### 配布ディレクトリ構造

各プラットフォームの release 圧縮Packageは、`package-dist.sh` が cargo-dist ビルド後に再構成します：

```
yaoxiang-{version}-{target}.tar.gz / .zip
├── bin/
│   ├── yaoxiang                      # または yaoxiang.exe
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # 事前生成された標準ライブラリインターフェースファイル
│           ├── io.yx
│           ├── math.yx
│           ├── string.yx
│           ├── ...
│           └── mod.yx
├── README.md
└── LICENSE
```

cargo-dist のデフォルトの zip は平坦です（バイナリ + 自動的に含まれる README/LICENSE はすべてルートディレクトリにあります）。これは問題ありません—明確な分担：cargo-dist がコンパイル+CI+インストーラーを担当し、YaoXiang は 50 行の
`package-dist.sh` で zip 構造を担当します。

### プラットフォームサポート

| プラットフォーム | target triple               | 説明                    |
| ---------------- | --------------------------- | ----------------------- |
| Linux x86_64     | `x86_64-unknown-linux-gnu`  | メインプラットフォーム  |
| Linux ARM64      | `aarch64-unknown-linux-gnu` | CI でのクロスコンパイル |
| macOS x86_64     | `x86_64-apple-darwin`       | Intel Mac               |
| macOS ARM64      | `aarch64-apple-darwin`      | Apple Silicon           |
| Windows x86_64   | `x86_64-pc-windows-msvc`    | メインプラットフォーム  |

Windows ARM64 は一時的にサポート外とします（Z3 公式には ARM64 プレビルドPackageがありません）。

### Z3 配布Strategy

**全プラットフォームで動的リンクに統一します。**

| プラットフォーム | 改动                   | 产物          |
| ---------------- | ---------------------- | ------------- |
| Linux            | **静的→動的に変更**    | `libz3.so`    |
| macOS            | **静的→動的に変更**    | `libz3.dylib` |
| Windows          | 変更なし               | `libz3.dll`   |
| wasm32           | 変更なし（静的リンク） | 内包 `.a`     |

理由：

- **一貫性** — 3 つのプラットフォームの挙動を統一、もう特例不要再
- **これは外部ライブラリであり、共有ライブラリで配布すべきです**。Python（`python3.dll`+`DLLs/lib*.dll`）、Node（`node`+`lib/`）也是如此
- **ユーザーは Z3 の升级を待つ必要はありません** — `.so`/`.dylib`/`.dll` を交换するだけ
- **バイナリサイズが小さくなります** — Z3 は小さくなく、静的リンクすると exe が数 MB 肥大化する

対応する `build.rs` の修正：

```rust
// 統一動的リンク
fn link_z3(z3_dir: &Path) {
    println!("cargo:rustc-link-lib=z3");     // Windows/非 Windows の区別不要再
    // C++ 標準ライブラリリンクは維持
    let cxx = if target_os == "macos" { "c++" } else { "stdc++" };
    println!("cargo:rustc-link-lib={}", cxx);
}
```

**"全プラットフォーム静的リンク"はもう目标ではありません。**
これは特殊ケースの消除ではなく、误った方法で合理的なケースを消除することです。共有ライブラリは外部ライブラリの正常な配布方式です。

### インストーラーサポート

| インストーラー        | 状態                   | 説明                                      |
| --------------------- | ---------------------- | ----------------------------------------- |
| zip / tar.gz          | ✅ デフォルト          | 全プラットフォーム、手動ダウンロード      |
| shell スクリプト      | ✅ cargo-dist          | Unix: `curl ... \| sh`                    |
| powershell スクリプト | ✅ cargo-dist          | Windows: `irm ... \| iex`                 |
| Homebrew formula      | ✅ cargo-dist          | macOS: `brew install yaoxiang`            |
| Windows MSI           | ✅ cargo-dist          | WiX ベース、メイン Windows インストーラー |
| **Inno Setup**        | **✅ 補助として 유지** | 国内ユーザー向け替代策、削除しない        |

**Inno Setup 維持の理由：**

- 国内 Windows ユーザーは exe インストールウィザード（次へ → 次へ → 完了）を更喜欢する
- MSI は certain 企業/学校のネットワーク環境でブロックされる場合がある
- `setup.iss` を追加で保守するコストは、一部のユーザーを失うコストよりはるかに低い

### 標準ライブラリインターフェースファイル生成

サブコマンド名：**`yaoxiang package gen-std`**（既存の `package init`/`add`/`install` と同じ体系中）

現在の `src/std/gen_interfaces.rs`
には完全な実装があります（`generate_all_interfaces()`、`write_interfaces_to_dir()`）。`main.rs`
に新しいサブコマンドエントリを追加し、`package-dist.sh` で呼び出すだけです：

```bash
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"
```

### Wasm ビルド

**独立性を維持し、cargo-dist に移行しません。**

cargo-dist が管理するのは"コンパイラをユーザーに届ける"ことで、wasm は"オンラインプレイグラウンド埋め込みドキュメントWebsite"—这两つの交付物は完全に異なります。

| 方面               | 做法                                    |
| ------------------ | --------------------------------------- |
| ビルドツール       | `wasm-pack build` を維持                |
| CI workflow        | `_build-wasm.yml` 独立 job を維持       |
| トリガータイミング | release と同一次の push、並列の独立 job |
| 配布対象           | `docs/public/wasm/` → GitHub Pages      |

### npm 公開

2 つの異なる npm Packageが、それぞれ独立しています：

| Package                | 内容                                   | ツール                    | 状態                             |
| ---------------------- | -------------------------------------- | ------------------------- | -------------------------------- |
| `@yaoxiang/cli`        | CLI バイナリをダウンロード（ラッパー） | cargo-dist 原生生成       | cargo-dist 設定のまま使用可能    |
| `@yaoxiang/playground` | wasm ライブラリ（JS + .wasm）          | wasm-pack + `npm publish` | オプション、現在は docs のみ公開 |

这两者は冲突せず、名前も冲突しません。

### Nightly 公開

cargo-dist には原生の nightly サポートがありません（[#1143](https://github.com/axodotdev/cargo-dist/issues/1143)、仍然是 open
feature request）。

**既存の cron + tag 方案を維持**し、ビルド部分を cargo-dist に置き換えます：

```yaml
# nightly.yml（移行後、約 50 行）
on: schedule: "17 22 * * *"
jobs:
  build:
    # cargo-dist のビルド機能は再利用し、release フローは通らない
    uses: ./.github/workflows/release.yml  # cargo-dist が生成したビルド job
  publish:
    # 既存のまま継続：nightly tag を打つ → GitHub Pre-release を上書き
```

### cargo-dist 設定（草案）

`cargo dist init` を実行後に生成される初期設定 expected core 部分：

```toml
[workspace]
members = ["cargo:."]

[dist]
targets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
]
installers = [
  "shell",
  "powershell",
  "homebrew",
  "msi",
]
```

具体的な設定項目は `cargo dist init` の実際の生成物に準拠します。

### package-dist.sh（草案）

```bash
#!/bin/bash
# cargo-dist ビルド後に実行され、配布Package構造を再構成
# cargo-dist の extra-artifacts または独立 CI step から呼び出される
set -euo pipefail

VERSION="$1"
TARGET="$2"
DIST_DIR="target/distrib"
PKG_ROOT="$DIST_DIR/yaoxiang-$VERSION-$TARGET"

mkdir -p "$PKG_ROOT/bin" "$PKG_ROOT/lib/yaoxiang/std"

# バイナリ
mv "$DIST_DIR/yaoxiang" "$PKG_ROOT/bin/"

# 共有ライブラリ
Z3_DIR=".z3/z3-4.16.0-..."
case "$TARGET" in
  *windows*)   cp "$Z3_DIR/bin/libz3.dll"   "$PKG_ROOT/bin/" ;;
  *linux*)     cp "$Z3_DIR/lib/libz3.so"    "$PKG_ROOT/bin/" ;;
  *apple*)     cp "$Z3_DIR/lib/libz3.dylib" "$PKG_ROOT/bin/" ;;
esac

# 標準ライブラリインターフェースファイル
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"

# README + LICENSE
cp README.md LICENSE "$PKG_ROOT/"

# 再Packageング
cd "$DIST_DIR"
tar czf "yaoxiang-$VERSION-$TARGET.tar.gz" "yaoxiang-$VERSION-$TARGET"
```

### 標準ライブラリインターフェースファイル生成

現在の `src/std/gen_interfaces.rs` には `.yx`
インターフェースファイルを生成する機能がすでに実装されています（`write_interfaces_to_dir`）。`package init`
コマンド에서도呼び出しています。

`main.rs` に新しいサブコマンドエントリを追加し、Packageングスクリプトで呼び出すだけです。

### 廃止する手書き CI

移行完了後に以下のファイルを削除します：

| ファイル                                 | 行数        | 代替                                 |
| ---------------------------------------- | ----------- | ------------------------------------ |
| `.github/workflows/_build-platforms.yml` | 255         | cargo-dist が自動生成                |
| `.github/workflows/release.yml`          | 176         | cargo-dist が自動生成                |
| `.github/workflows/nightly.yml`          | 145         | cargo-dist ビルド + 发布ロジック保持 |
| `scripts/build/setup.iss`                | ~250        | **保持**（国内使用）                 |
| **合計削除**                             | **~600 行** |                                      |

保持するもの：

- `ci.yml`（日常の fmt + clippy + test + MSRV、公開フローに属さない）
- `nightly.yml`（发布ロジック部分を保持）
- `_build-wasm.yml`（独立ビルドフロー）
- `_build-z3-wasm.yml`（wasm 专用 Z3）
- `setup.iss`（国内補助インストーラー）
- `docs-deploy.yml`（ドキュメントDeployment）

## 权衡

### 优点

- **即使用可能** — ユーザーはダウンロード・解凍後すぐに実行でき、DLL 欠落の問題がない
- **保守コスト削減** — 手書き CI YAML 約 600 行を削除、cargo-dist が自動保守
- **標準化** — 業界標準ツール、何百ものプロジェクトで検証済み
- **クロスプラットフォーム一貫性** — 全プラットフォーム動的リンク、挙動を統一
- **インストーラー覆盖** — shell/powershell/homebrew/msi/inno setup 完全サポート

### 缺点

- **cargo-dist 設定の学習** — チームは新ツールを学ぶ必要がある
- **カスタムPackageングスクリプト仍有保守コスト** —
  Package構造と標準ライブラリインターフェースファイルのスクリプトは保守が必要
- **cargo-dist のバージョン迭代** — upstream の变更，关注が必要
- **cargo-dist には原生の nightly がない** — nightly 公开部分は仍用手書き

### RFC-014b との関係

|            | RFC-014b                              | RFC-037                                      |
| ---------- | ------------------------------------- | -------------------------------------------- |
| **範囲**   | 第三方パッケージのビルドと配布        | コンパイラ自体のPackageングと配布            |
| **ツール** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist`                                 |
| **产物**   | 第三方パッケージの FFI ライブラリ     | コンパイラ + 標準ライブラリ + ツールチェーン |
| **互斥**   | 否、互补                              | 否、互补                                     |

## 替代方案

| 方案                                | 为什么不选                                                                     |
| ----------------------------------- | ------------------------------------------------------------------------------ |
| **继续手書き CI**                   | すでに ~900 行の手書きがあり、重复労働、DLL 漏れしやすい                       |
| **自分でPackageングツールを書く**   | 車輪の再発明禁欲、cargo-dist はすでに成熟している                              |
| **tar.gz のみ、インストーラーなし** | ユーザーはよりユーザーフレンドリーなインストール方式必要がある（Homebrew/MSI） |
| **Docker 配布**                     | コンパイラと言語ツールチェーンはネイティブバイナリ必要，不是容器场景           |
| **全静的リンク Z3**                 | 外部ライブラリは正常に共有ライブラリで配布すべき、静的追求不要                 |
| **Inno Setup 廃止**                 | 国内ユーザーの习惯不同、維持コストは非常に低い                                 |

## 実装Strategy

### フェーズ一：build.rs の修正 + gen-std サブコマンド（P0）

1. `build.rs` を修正：全プラットフォーム統一動的リンク、`copy_dll()` を `copy_shared_lib()` に扩展
2. `main.rs` に新しい `yaoxiang package gen-std` サブコマンドを追加（`gen_interfaces.rs` を再利用）

### フェーズ二：cargo-dist 接入（P0）

1. `cargo dist init` を実行して初期設定を生成
2. `package-dist.sh` Packageングスクリプトを作成
3. `release.yml` に統合：cargo-dist ビルド → `package-dist.sh` 再構成 → アップロード
4. 生成された圧縮Packageの構造と内容が正しいことを確認

### フェーズ三：旧 CI の下线（P1）

1. 新旧 CI を並行実行し、产物を比較
2. 问题なければ `_build-platforms.yml` を削除
3. `nightly.yml` を精简（ビルド部分を cargo-dist に交換）
4. `setup.iss` が仍然使用可能であることを確認

### フェーズ四：インストーラー有効化（P2）

1. Homebrew tap 自動公开を設定
2. MSI インストーラー生成を設定
3. npm 公開を設定（`@yaoxiang/cli`）

## 開放問題（已解决）

以下の問題は設計讨论で 이미 解決済みです：

- ~~Windows での Z3 静的リンクの可行性？~~ → **静的リンク不做、全プラットフォーム動的**
- ~~gen-std-interfaces サブコマンドの名前？~~ → **`yaoxiang package gen-std`**
- ~~Inno Setup を保持するか？~~ → **保持**
- ~~cargo-dist extra-artifacts 条件実行？~~ → **`package-dist.sh` スクリプトで処理、shell
  case 分岐を使用**
- ~~標準ライブラリインターフェースのバージョン互換性？~~ →
  **コンパイラバージョンと一緒に公開、同じ圧縮Package内**

## 参考文献

- [cargo-dist 公式ドキュメント](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 ビルド設定 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

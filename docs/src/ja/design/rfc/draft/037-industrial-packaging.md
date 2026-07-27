---
title: "RFC-037: 工業的配布ソリューション — cargo-dist ベースのコンパイラ/ツールチェーンパッケージング"
author: "ChenXu233"
created: "2026-07-26"
updated: "2026-07-27"
issue: "#230"
---

# RFC-037: 工業的配布ソリューション — cargo-dist ベースのコンパイラ/ツールチェーンパッケージング

> 本 RFC は [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md) と補完関係にある。
> RFC-014b は **YaoXiang パッケージマネージャ**がサードパーティパッケージをビルド・配布する方法を定義しており、
> 本 RFC は **YaoXiang コンパイラ/ツールチェーン自体**をパッケージング・配布する方法を定義する。

## 概要

既存の hand-written CI ビルド/パッケージングロジックを `cargo-dist`（Rust エコシステムのバイナリ配布標準ツール）に置き換え、クロスプラットフォーム自動リリースを実現する。`libz3.dll` の欠落、標準ライブラリインターフェースファイルがパッケージに含まれない問題、ディレクトリ構造の混乱、CI スクリプトの重複メンテナンスなどの問題を解決する。

## 動機

### なぜこの機能が必要なのか？

YaoXiang をダウンロードしたユーザーは、追加の手順なしで**すぐに使える**べきである。

### 現状の問題

#### 問題 1：Windows ユーザーがダウンロード後に実行できない

現在の Release では `yaoxiang.exe` のみアップロードされており、`libz3.dll` がパッケージに含まれていない。Windows でダブルクリックして実行すると以下のエラーが出る：

```
The code execution cannot proceed because libz3.dll was not found.
```

これは**ブロッキングバグ**であり、ユーザーは最初の段階で先に進めない。

#### 問題 2：Release アーティファクトが単一 exe のみ

```
yaoxiang-v0.7.10-x86_64-pc-windows-msvc.zip
└── yaoxiang.exe
```

標準ライブラリインターフェースファイル（`.yx` ファイル、LSP で必要）が配布パッケージに含まれていない。ユーザーは `yaoxiang package init` を実行して生成する必要がある。工業的なアプローチとしては、配布パッケージに標準ライブラリを同梱すべきである。

#### 問題 3：CI の hand-written スクリプトの重複メンテナンス

現在、4 セットのビルドパイプラインをメンテナンスしている：

| ファイル | 役割 | 行数 |
|------|------|------|
| `_build-platforms.yml` | クロスプラットフォームビルド | ~255 行 |
| `release.yml` | バージョンリリース | ~176 行 |
| `nightly.yml` | デイリービルド | ~145 行 |
| `_build-wasm.yml` | Wasm ビルド | ~75 行 |
| `scripts/build/setup.iss` | Inno Setup インストーラ | ~250 行 |
| **合計** | | **~900 行** |

大部分は重複しており（Rust のインストール → キャッシュ → ビルド → リネーム → アップロード）、各プラットフォームごとに記述が必要。`cargo-dist` なら 1 コマンドで同等のパイプラインを生成できる。

#### 問題 4：Inno Setup のバージョン番号ハードコード

`setup.iss` の `MyAppVersion` が `0.7.0` とハードコードされており、ビルド時に `sed` で置換している。迟早壊れる。

#### 問題 5：RFC-014b との境界が曖昧

RFC-014b は「YaoXiang パッケージのビルドと配布メカニズム」（すなわち `yaoxiang.toml` の `[build]` と `[binaries]` 設定）を定義しているが、**「YaoXiang コンパイラ自体をどうリリースするか」をカバーしていない**。本 RFC はこの空白を埋める。

## 提案

### コアデザイン

**cargo-dist** をリリースパイプラインの骨格として採用し、カスタム post-build スクリプトでパッケージ構造と追加ファイルを処理する。

```
cargo-dist の役割:
  ├── クロスプラットフォームコンパイル（6 つの target）
  ├── CI パイプラインの生成（手書きの ~900 行 YAML を置換）
  ├── インストーラの生成（MSI / shell / powershell / homebrew）
  ├── npm 公開（@yaoxiang/cli — バイナリダウンロード wrapper）
  ├── checksum + 署名
  └── GitHub Release へのアップロード

build.rs は引き続き担当:
  └── Z3 のダウンロード/リンク（既存ロジック、全プラットフォーム動的リンクに変更）

YaoXiang カスタムスクリプト（package-dist.sh）の役割:
  ├── ビルド後に zip 構造を再編成（bin/ + lib/）
  ├── 共有ライブラリを同梱（libz3.so / dylib / dll）
  └── 標準ライブラリ .yx インターフェースファイルを事前生成
```

### 配布ディレクトリ構造

各プラットフォームの release 圧縮パッケージは、`package-dist.sh` が cargo-dist ビルド後に再編成する：

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

cargo-dist デフォルトの zip はフラット構造（バイナリ + 自動同梱の README/LICENSE がすべてルートディレクトリ）になっている。
これは問題ではない — 役割を明確に分ける：cargo-dist はコンパイル + CI + インストーラを担当し、YaoXiang は 50 行の `package-dist.sh` で zip 構造を管理する。

### プラットフォームサポート

| プラットフォーム | target triple | 説明 |
|------|-------------|------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | メインプラットフォーム |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | CI 上でクロスコンパイル |
| macOS x86_64 | `x86_64-apple-darwin` | Intel Mac |
| macOS ARM64 | `aarch64-apple-darwin` | Apple Silicon |
| Windows x86_64 | `x86_64-pc-windows-msvc` | メインプラットフォーム |

Windows ARM64 は当面サポートしない（Z3 公式に ARM64 のプレコンパイルパッケージがない）。

### Z3 配布戦略

**全プラットフォームで統一して動的リンクに変更する。**

| プラットフォーム | 変更点 | 成果物 |
|------|------|------|
| Linux | **静的→動的に変更** | `libz3.so` |
| macOS | **静的→動的に変更** | `libz3.dylib` |
| Windows | 変更なし | `libz3.dll` |
| wasm32 | 変更なし（静的リンク） | 埋め込み `.a` |

理由：
- **一貫性** — 3 つのプラットフォームの動作が統一され、特別なケースがなくなる
- **外部ライブラリは共有ライブラリで配布すべき**。Python（`python3.dll`+`DLLs/lib*.dll`）、Node（`node`+`lib/`）も同じ方式
- **ユーザーが Z3 をアップグレードする際にコンパイラのバージョンを待つ必要がない** — `.so`/`.dylib`/`.dll` を置き換えるだけ
- **バイナリサイズが小さくなる** — Z3 は小さくないため、静的リンクは exe を数 MB 膨張させる

対応する `build.rs` の変更：

```rust
// 統一して動的リンク
fn link_z3(z3_dir: &Path) {
    println!("cargo:rustc-link-lib=z3");     // Windows/非 Windows を区別しない
    // C++ 標準ライブラリのリンクは変更しない
    let cxx = if target_os == "macos" { "c++" } else { "stdc++" };
    println!("cargo:rustc-link-lib={}", cxx);
}
```

**「全プラットフォームでの静的リンク」は目標としない。** これは特殊ケースの排除ではなく、合理的なケースを誤った方法で排除するものである。共有ライブラリは外部ライブラリの通常の配布方式である。

### インストーラサポート

| インストーラ | ステータス | 説明 |
|--------|------|------|
| zip / tar.gz | ✅ デフォルト | 全プラットフォーム、手動ダウンロード |
| shell スクリプト | ✅ cargo-dist | Unix: `curl ... \| sh` |
| powershell スクリプト | ✅ cargo-dist | Windows: `irm ... \| iex` |
| Homebrew formula | ✅ cargo-dist | macOS: `brew install yaoxiang` |
| Windows MSI | ✅ cargo-dist | WiX ベース、メイン Windows インストーラ |
| **Inno Setup** | **✅ 補助として保持** | 国内ユーザ向けオプション、削除しない |

**Inno Setup を保持する理由：**
- 国内の Windows ユーザは exe インストールウィザート（次へ → 次へ → 完了）に慣れている
- MSI は一部の企業/学校のネットワーク環境でブロックされる
- `setup.iss` を 1 つ追加でメンテナンスするコストは、一部のユーザを失うコストよりはるかに低い

### 標準ライブラリインターフェースファイルの生成

サブコマンド名：**`yaoxiang package gen-std`**（既存の `package init`/`add`/`install` と同じ体系）

現在 `src/std/gen_interfaces.rs` に完全な実装が既にある（`generate_all_interfaces()`、`write_interfaces_to_dir()`）。`main.rs` にサブコマンドエントリを追加し、`package-dist.sh` から呼び出すだけ：

```bash
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"
```

### Wasm ビルド

**独立を維持し、cargo-dist には移行しない。**

cargo-dist は「コンパイラをユーザに配布する」役割であり、wasm は「ドキュメントサイトにオンライン playground を埋め込む」ためのもので、2 つは全く異なる成果物である。

| 側面 | 方針 |
|------|------|
| ビルドツール | `wasm-pack build` を維持 |
| CI workflow | `_build-wasm.yml` を独立 job として保持 |
| トリガタイミング | release と同じ push で並行して実行される独立 job |
| 公開先 | `docs/public/wasm/` → GitHub Pages |

### npm 公開

2 つの異なる npm パッケージ、それぞれ独立：

| パッケージ | 内容 | ツール | ステータス |
|----|------|------|------|
| `@yaoxiang/cli` | CLI バイナリのダウンロード（wrapper） | cargo-dist ネイティブ生成 | cargo-dist 設定で動作 |
| `@yaoxiang/playground` | wasm ライブラリ（JS + .wasm） | wasm-pack + `npm publish` | オプション、現在 docs のみ公開 |

両者は競合せず、名前も競合しない。

### Nightly 公開

cargo-dist にはネイティブの nightly サポートがない（[#1143](https://github.com/axodotdev/cargo-dist/issues/1143)、依然として open feature request）。

**既存の cron + tag 方式を維持**し、ビルド部分を cargo-dist に置き換える：

```yaml
# nightly.yml（移行後、約 50 行）
on: schedule: "17 22 * * *"
jobs:
  build:
    # cargo-dist のビルド能力を再利用、ただし release フローは通さない
    uses: ./.github/workflows/release.yml  # cargo-dist 生成のビルド job
  publish:
    # 既存を維持：nightly tag を打つ → GitHub Pre-release を上書き
```

### cargo-dist 設定（ドラフト）

`cargo dist init` 実行後に生成される初期設定、想定されるコア部分：

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

具体的な設定項目は `cargo dist init` の実際の生成結果に従う。

### package-dist.sh（ドラフト）

```bash
#!/bin/bash
# cargo-dist ビルド後に実行し、配布パッケージ構造を再編成する
# cargo-dist の extra-artifacts または独立した CI step から呼ばれる
set -euo pipefail

VERSION="$1"
TARGET="$2"
DIST_DIR="target/distrib"
PKG_ROOT="$DIST_DIR/yaoxiang-$VERSION-$TARGET"

mkdir -p "$PKG_ROOT/bin" "$PKG_ROOT/lib/yaoxiang/std"

# binary
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

# 再パッケージング
cd "$DIST_DIR"
tar czf "yaoxiang-$VERSION-$TARGET.tar.gz" "yaoxiang-$VERSION-$TARGET"
```

### 標準ライブラリインターフェースファイルの生成

現在 `src/std/gen_interfaces.rs` に `.yx` インターフェースファイル生成機能が既に実装されており（`write_interfaces_to_dir`）、`package init` コマンドもこれを呼び出している。

`main.rs` にサブコマンドエントリを追加し、パッケージングスクリプトから呼び出すだけでよい。

### 廃止する hand-written CI

移行完了後に以下のファイルを削除する：

| ファイル | 行数 | 代替 |
|------|------|------|
| `.github/workflows/_build-platforms.yml` | 255 | cargo-dist 自動生成 |
| `.github/workflows/release.yml` | 176 | cargo-dist 自動生成 |
| `.github/workflows/nightly.yml` | 145 | cargo-dist ビルド + 公開ロジック保持 |
| `scripts/build/setup.iss` | ~250 | **保持**（国内向け） |
| **削除合計** | **~600 行** | |

保持する：
- `ci.yml`（日常 fmt + clippy + test + MSRV、リリースフローに属さない）
- `nightly.yml`（公開ロジック部分のみ保持）
- `_build-wasm.yml`（独立したビルドフロー）
- `_build-z3-wasm.yml`（wasm 専用 Z3）
- `setup.iss`（国内向け補助インストーラ）
- `docs-deploy.yml`（ドキュメントデプロイ）

## トレードオフ

### メリット

- **すぐ使える** — ダウンロード・解凍後すぐに実行でき、DLL 欠落の問題がない
- **メンテナンスコストの削減** — 手書きの CI YAML 約 600 行を削除、cargo-dist が自動メンテナンス
- **標準化** — 業界標準ツール、何百ものプロジェクトで検証済み
- **クロスプラットフォームの一貫性** — 全プラットフォーム動的リンクで動作統一
- **インストーラの網羅** — shell/powershell/homebrew/msi/inno setup をすべてサポート

### デメリット

- **cargo-dist 設定の学習** — チームが新しいツールを学習する必要がある
- **カスタムパッケージングスクリプトのメンテナンスコストが残る** — パッケージ構造と標準ライブラリインターフェースファイルのスクリプトのメンテナンスが必要
- **cargo-dist のバージョンアップ** — upstream の変更をフォローする必要がある
- **cargo-dist にネイティブの nightly がない** — nightly 公開部分は引き続き手書きが必要

### RFC-014b との関係

| | RFC-014b | RFC-037 |
|--|----------|---------|
| **範囲** | サードパーティパッケージのビルドと配布 | コンパイラ自体のパッケージングと配布 |
| **ツール** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist` |
| **成果物** | サードパーティパッケージの FFI ライブラリ | コンパイラ + 標準ライブラリ + ツールチェーン |
| **排他** | いいえ、補完関係 | いいえ、補完関係 |

## 代替案

| 案 | 採用しない理由 |
|------|-----------|
| **手書き CI を継続** | 既に約 900 行手書き済みで、重複作業となり DLL 漏れが発生しやすい |
| **独自のパッケージングツールを作成** | 車輪の再発明はしない、cargo-dist は既に成熟している |
| **tar.gz のみでインストーラを使わない** | ユーザによりフレンドリーなインストール方式（Homebrew/MSI）が必要 |
| **Docker 配布** | コンパイラと言語ツールチェーンにはネイティブバイナリが必要で、コンテナ向けではない |
| **Z3 を全静的リンク** | 外部ライブラリは共有ライブラリで配布するのが普通であり、静的リンクを追求しない |
| **Inno Setup を廃止** | 国内ユーザの習慣が異なるため、保持コストは非常に低い |

## 実装戦略

### フェーズ 1：build.rs の変更 + gen-std サブコマンド（P0）

1. `build.rs` の修正：全プラットフォーム統一動的リンク、`copy_dll()` を `copy_shared_lib()` に拡張
2. `main.rs` に `yaoxiang package gen-std` サブコマンドを追加（`gen_interfaces.rs` を再利用）

### フェーズ 2：cargo-dist 統合（P0）

1. `cargo dist init` を実行して初期設定を生成
2. `package-dist.sh` パッケージングスクリプトを記述
3. `release.yml` に統合：cargo-dist ビルド → `package-dist.sh` 再編成 → アップロード
4. 生成された圧縮パッケージの構造と内容が正しいことを検証

### フェーズ 3：旧 CI の停止（P1）

1. 新旧 CI を並行実行し、成果物を比較
2. 問題がないことを確認後、`_build-platforms.yml` を削除
3. `nightly.yml` を簡素化（ビルド部分を cargo-dist に置換）
4. `setup.iss` が引き続き使用できることを確認

### フェーズ 4：インストーラの有効化（P2）

1. Homebrew tap の自動公開を設定
2. MSI インストーラ生成を設定
3. npm 公開を設定（`@yaoxiang/cli`）

## オープンイシュー（解決済み）

以下の問題はデザイン議論で解決済み：

- ~~Windows での Z3 静的リンクの実現性？~~ → **静的リンクは行わず、全プラットフォーム動的に**
- ~~gen-std-interfaces サブコマンドの命名？~~ → **`yaoxiang package gen-std`**
- ~~Inno Setup を保持するか？~~ → **保持**
- ~~cargo-dist extra-artifacts の条件付き実行？~~ → **`package-dist.sh` スクリプトで処理、shell case 分岐を使用**
- ~~標準ライブラリインターフェースのバージョン互換性？~~ → **コンパイラのバージョンと一緒にリリース、同じ圧縮パッケージ内**

## 参考資料

- [cargo-dist 公式ドキュメント](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 ビルド設定 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)
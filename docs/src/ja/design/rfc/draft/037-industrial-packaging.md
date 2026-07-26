---
title: "RFC-037: 産業的配布ソリューション — cargo-dist に基づくコンパイラ/ツールチェーンパッケージング"
author: "ChenXu233"
created: "2026-07-26"
updated: "2026-07-26"
issue: "#230"
---

# RFC-037: 産業的配布ソリューション — cargo-dist に基づくコンパイラ/ツールチェーンパッケージング

> 本 RFC は [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md) を補完するものである。
> RFC-014b は **YaoXiang パッケージマネージャ** がサードパーティ製パッケージをビルドし配布する方法を定義している。
> 本 RFC は **YaoXiang コンパイラ/ツールチェーン自体** をパッケージングし配布する方法を定義する。

## 概要

既存の CI ビルド/パッケージング手書きロジックを `cargo-dist`（Rust エコシステムのバイナリ配布標準ツール）に置き換え、クロスプラットフォームの自動リリースを実現する。`libz3.dll` の欠落、標準ライブラリインターフェースファイルがパッケージに含まれない問題、ディレクトリ構造の混乱、CI スクリプトの重複メンテナンスなどの問題を解決する。

## 動機

### なぜこの機能が必要なのか?

YaoXiang をダウンロードしたユーザーは、**箱から出してすぐ使える** べきであり、追加のステップは一切必要ない。

### 現状の問題

#### 問題 1: Windows ユーザーがダウンロード後に実行できない

現在の Release では `yaoxiang.exe` のみがアップロードされているが、`libz3.dll` はパッケージに含まれていない。Windows ユーザーがダブルクリックで実行すると以下のエラーが発生する:

```
The code execution cannot proceed because libz3.dll was not found.
```

これは **ブロッキングバグ** であり、ユーザーは最初の段階で先に進むことすらできない。

#### 問題 2: Release 成果物が単一 exe ファイルのみ

```
yaoxiang-v0.7.10-x86_64-pc-windows-msvc.zip
└── yaoxiang.exe
```

標準ライブラリのインターフェースファイル（`.yx` ファイル、LSP で必要）はリリースパッケージに含まれておらず、ユーザーは `yaoxiang package init` を実行して生成する必要がある。産業的なアプローチは次のとおりであるべきである:

```
yaoxiang-0.7.10-x86_64-pc-windows-msvc.zip
├── bin/
│   ├── yaoxiang.exe
│   └── libz3.dll
└── lib/
    └── std/
        ├── io.yx
        ├── math.yx
        ├── ...
        └── mod.yx
```

#### 問題 3: CI 手書きスクリプトの重複メンテナンス

現在 3 種類のビルドパイプラインをメンテナンスしている:

| ファイル | 役割 | 行数 |
|------|------|------|
| `_build-platforms.yml` | クロスプラットフォームビルド（Linux/macOS/Windows） | 約 250 行 |
| `release.yml` | バージョンリリースフロー | 約 170 行 |
| `nightly.yml` | 日次ビルド | 約 170 行 |

**合計約 600 行の手書き YAML。** これらのスクリプトの大部分は重複しており（Rust インストール → キャッシュ → ビルド → リネーム → アップロード）、各プラットフォームごとに記述する必要がある。`cargo-dist` なら 1 コマンドで同等のパイプラインを生成できる。

#### 問題 4: Inno Setup のバージョン番号ハードコード

`setup.iss` の `MyAppVersion` に `0.7.0` がハードコードされており、ビルド時に `sed` で置換している。早晩問題が発生する。

#### 問題 5: RFC-014b との境界が曖昧

RFC-014b は「YaoXiang パッケージのビルドおよび配布機構」（すなわち `yaoxiang.toml` の `[build]` と `[binaries]` 設定）を定義しているが、**「YaoXiang コンパイラ自体をどうリリースするか」まではカバーしていない**。本 RFC はこのギャップを埋める。

## 提案

### コア設計

リリースパイプラインとして **cargo-dist** を採用し、Z3 DLL と標準ライブラリインターフェースファイルを処理するためにカスタム post-build スクリプトを組み合わせる。

```
cargo-dist の担当:
  ├── クロスプラットフォームビルド（6 プラットフォーム）
  ├── tar.gz/zip アーカイブの生成
  ├── インストールスクリプト（shell/powershell）の生成
  ├── Windows MSI インストーラの生成
  ├── GitHub Release の自動発行
  └── changelog の自動生成

build.rs の担当（継続）:
  └── Z3 ダウンロード/リンク（既存ロジック、微調整のみ必要）

カスタムスクリプトの担当:
  ├── ビルド後に libz3.dll をパッケージディレクトリにコピー
  └── 標準ライブラリ .yx インターフェースファイルの事前生成
```

### リリースディレクトリ構造

各プラットフォームの release アーカイブ:

```
yaoxiang-{version}-{target}.tar.gz / .zip
├── bin/
│   ├── yaoxiang                      # または yaoxiang.exe
│   └── libz3.dll                     # Windows のみ、他のプラットフォームは静的リンク
├── lib/
│   └── std/                          # 事前生成された標準ライブラリインターフェースファイル
│       ├── io.yx
│       ├── math.yx
│       ├── string.yx
│       ├── ...
│       └── mod.yx
└── README.md                         # 簡潔なインストール手順
```

### プラットフォームサポート

| プラットフォーム | target triple | 説明 |
|------|-------------|------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | メインプラットフォーム |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | CI でクロスコンパイル |
| macOS x86_64 | `x86_64-apple-darwin` | Intel Mac |
| macOS ARM64 | `aarch64-apple-darwin` | Apple Silicon |
| Windows x86_64 | `x86_64-pc-windows-msvc` | メインプラットフォーム |
| Windows ARM64 | `aarch64-pc-windows-msvc` | オプション、後続対応 |

### Z3 配布戦略

| プラットフォーム | 戦略 | 理由 |
|------|------|------|
| Linux | 静的リンク `libz3.a` | 既存、維持 |
| macOS | 静的リンク `libz3.a` | 既存、維持 |
| Windows | `libz3.dll` をパッケージング | Z3 公式の Windows プレビルドは DLL のみ |
| wasm32 | 静的リンク `libz3.a` | 既存、維持 |

**Windows の `libz3.dll` は build.rs がビルド時に `.z3/` ディレクトリへダウンロードし、cargo-dist の `extra-artifacts` 機構によってアーカイブへパッケージングされる。**

長期的目標: Z3 の Windows 用静的ライブラリを自前でビルドし（`-DZ3_BUILD_LIBZ3_SHARED=OFF`）、全プラットフォームで静的リンクによる単一ファイル配布を実現する。

### インストーラサポート

| インストーラ | サポート | 説明 |
|--------|------|------|
| tar.gz / zip | ✅ デフォルト | 全プラットフォーム |
| shell インストールスクリプト | ✅ cargo-dist 内蔵 | Unix プラットフォーム |
| powershell インストールスクリプト | ✅ cargo-dist 内蔵 | Windows プラットフォーム |
| Homebrew formula | ✅ cargo-dist 内蔵 | macOS |
| Windows MSI | ✅ cargo-dist 内蔵 | Inno Setup の代替 |
| Inno Setup | ❌ 廃止 | cargo-dist の MSI へ移行 |

## 詳細設計

### cargo-dist 設定

プロジェクトルートに `dist-workspace.toml` を作成する:

```toml
[workspace]
# Cargo ワークスペースを指す、すべての binary パッケージが自動検出される
members = ["cargo:."]

[dist]
# ビルド成果物
package-libraries = ["cdylib"]

# インストーラ
installers = [
    "shell",           # Unix shell インストールスクリプト
    "powershell",      # Windows powershell インストールスクリプト
    "homebrew",        # macOS Homebrew
    "msi",             # Windows MSI インストーラ
]

# ビルド後の追加処理スクリプト
extra-artifacts = [
    "scripts/build/package-z3.sh",   # libz3.dll と標準ライブラリインターフェースファイルのコピー
]

# CI 設定
ci = "github"
ci.github.create-release = true
ci.github.pr-run-mode = "plan"
```

### ビルド後処理スクリプト

`scripts/build/package-z3.sh`（クロスプラットフォーム、cargo-dist ビルド後に実行）:

```bash
#!/bin/bash
# cargo-dist のビルド後に libz3.dll と標準ライブラリインターフェースファイルをパッケージディレクトリへコピーする

set -euo pipefail

# 1. Z3 DLL のコピー（Windows のみ）
if [ "$CARGO_DIST_TARGET" = "x86_64-pc-windows-msvc" ]; then
    Z3_DIR=".z3/z3-4.16.0-x64-win"
    cp "$Z3_DIR/bin/libz3.dll" "$DIST_DIR/bin/"
fi

# 2. 標準ライブラリインターフェースファイルの生成
yaoxiang package gen-std-interfaces --out-dir "$DIST_DIR/lib/std/"
```

### CI パイプラインの変化

#### 移行前（現状）:

```
release.yml (170 行) ──→ _build-platforms.yml (250 行) ──→ ビルド → アップロード
                  └──→ wasm ビルド
                  └──→ セキュリティ監査
                  └──→ テスト
                  └──→ リリース

nightly.yml (170 行) ──→ 同上（重複）
```

#### 移行後:

```
cargo-dist 生成の release.yml（約 100 行、自動メンテナンス）
  └──→ 6 プラットフォーム並列ビルド
  └──→ アーカイブとインストールスクリプトの生成
  └──→ GitHub Release の作成
  └──→ Homebrew / npm へのアップロード

cargo-dist 生成の pr.yml（約 50 行、自動メンテナンス）
  └──→ PR 時に dist plan チェックを実行
```

### 標準ライブラリインターフェースファイルの生成

現在の `src/std/gen_interfaces.rs` は既に `.yx` インターフェースファイルの生成機能（`write_interfaces_to_dir`）を実装しており、`package init` コマンドもこれを呼び出している。

必要な作業:

1. `main.rs` に新しいサブコマンド `yaoxiang package gen-std-interfaces` を追加（または独立スクリプト）
2. パッケージングスクリプトでこのコマンドを呼び出し、`lib/std/` へ生成

### 廃止する手書き CI

移行完了後に以下のファイルを削除する:

| ファイル | 代替 |
|------|------|
| `.github/workflows/_build-platforms.yml` | cargo-dist が自動生成 |
| `.github/workflows/release.yml` | cargo-dist が自動生成 |
| `.github/workflows/nightly.yml` | cargo-dist の schedule トリガー |
| `scripts/build/setup.iss` | cargo-dist の MSI インストーラ |
| `scripts/build/ChineseSimplified.isl` | 同上 |

## トレードオフ

### 利点

- **箱から出してすぐ使える** — ユーザーがアーカイブを展開後すぐ実行でき、DLL 欠落の問題は発生しない
- **メンテナンスコストの削減** — 手書き CI YAML を約 600 行削除し、cargo-dist が自動メンテナンス
- **標準化** — 業界標準ツールであり、数百のプロジェクトで検証済み
- **クロスプラットフォームの一貫性** — 6 プラットフォームで同一のパイプラインを使用
- **自動 changelog** — changelog 生成とリリースノート作成を内蔵
- **インストーラの網羅** — shell/powershell/homebrew/msi をすべてサポート

### 欠点

- **cargo-dist 設定の学習** — チームは新しいツールを学ぶ必要がある
- **カスタム処理にもメンテナンスコストが残る** — Z3 DLL と標準ライブラリインターフェースファイルのスクリプトはメンテナンスが必要
- **cargo-dist のバージョン更新への追従** — upstream の更新に追従する必要がある
- **Windows ARM64 サポート** — cargo-dist はデフォルトで対応するが、Z3 に ARM64 用プレビルド DLL がない可能性がある

### RFC-014b との関係

| | RFC-014b | RFC-037 |
|--|----------|---------|
| **スコープ** | サードパーティ製パッケージのビルドと配布 | コンパイラ自体のパッケージングと配布 |
| **ツール** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist` |
| **成果物** | サードパーティ製パッケージの FFI ライブラリ | コンパイラ + 標準ライブラリ + ツールチェーン |
| **排他性** | いいえ、補完関係 | いいえ、補完関係 |

## 代替案

| 代替案 | 採用しない理由 |
|------|-----------|
| **手書き CI を継続** | 既に約 600 行を手書きしており、重複作業であり、DLL 漏れが発生しやすい |
| **パッケージングツールを自前開発** | 車輪の再発明は避けるべき、cargo-dist は既に成熟している |
| **tar.gz のみでインストーラは使用しない** | ユーザーにはより 친화的なインストール方法（Homebrew/MSI）が必要 |
| **Docker 配布** | コンパイラと言語ツールチェーンにはネイティブバイナリが必要であり、コンテナは適さない |
| **Z3 を完全静的リンク** | 理想案だが、Windows での Z3 静的コンパイルには追加の CI ステップが必要であり、後続の最適化で対応可能 |

## 実装戦略

### フェーズ 1: 基盤移行（高優先度）

1. cargo-dist の最新バージョンと設定形式を調査・確認
2. cargo-dist をインストールし、`dist init` を実行して初期設定を生成
3. `dist-workspace.toml` を設定し、ターゲットプラットフォームを指定
4. `cc` crate を使用して build.rs の Z3 外部ダウンロードロジックを代替（オプション）

### フェーズ 2: カスタムパッケージング（中優先度）

1. `package-z3.sh` ビルド後処理スクリプトを作成
2. `main.rs` に `gen-std-interfaces` サブコマンドを追加
3. パッケージングスクリプトで標準ライブラリインターフェースファイルの生成を呼び出し
4. 生成されたアーカイブの構造が正しいことを検証

### フェーズ 3: 旧 CI の廃止（高優先度）

1. `release.yml` に cargo-dist パイプラインを統合
2. 新旧 CI を並列実行し、成果物の一貫性を比較
3. 問題がないことを確認後、旧 CI ファイルを削除
4. `setup.iss` と関連スクリプトを削除

### フェーズ 4: 最適化（低優先度）

1. Windows での Z3 静的コンパイルの実現可能性を調査
2. Homebrew formula の自動リリースを追加
3. MSI インストーラを追加
4. ARM64 Windows サポートを検討

### 依存関係

- 外部ツールチェーンチェーン依存なし（cargo-dist は `cargo install` で導入）
- GitHub Actions で CI を実行する必要あり
- Homebrew メンテナアカウントが必要（オプション）

### リスク

- **cargo-dist のバージョンアップ**: 設定形式が変更される可能性があり、changelog の確認が必要
- **Z3 公式リリースの変更**: Z3 プレビルドパッケージの場所や形式が変更される可能性
- **Windows 静的リンク**: Z3 の Windows 用静的ライブラリは追加処理（C++ ランタイム依存など）が必要になる可能性

## 未解決の問題

- [ ] Windows での Z3 静的リンクの実現可能性？MSVC 环境下で `-DZ3_BUILD_LIBZ3_SHARED=OFF` の動作を実測する必要あり
- [ ] `gen-std-interfaces` サブコマンドの具体的な命名とインターフェース設計は？
- [ ] Inno Setup インストーラを MSI の補完として保持するか？国内ユーザーは exe 形式のインストールウィザードに慣れている可能性がある
- [ ] cargo-dist の `extra-artifacts` はクロスプラットフォーム条件付き実行（Windows のみで DLL をコピーするなど）をサポートするか？
- [ ] 標準ライブラリインターフェースファイルにバージョン互換性の保証はあるか？コンパイラのバージョンと同時にリリースすべきか？

## 参考文献

- [cargo-dist 公式ドキュメント](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
- [Rust コンパイラ配布フロー — bootstrap dist](https://doc.rust-lang.org/stable/nightly-rustc/bootstrap/core/build_steps/dist/index.html)
- [Go ツールチェーン配布 — Go Toolchains](https://go.dev/doc/toolchain)
- [Z3 ビルド設定 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)
- [Z3 Windows 配布スクリプト](https://github.com/Z3Prover/z3/blob/master/scripts/mk_win_dist_cmake.py)
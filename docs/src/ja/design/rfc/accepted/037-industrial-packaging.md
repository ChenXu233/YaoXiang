---
title:
  'RFC-037: 産業化ディストリビューション方案 — cargo-dist
  に基づくコンパイラ/ツールチェーンパッケージング'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-09-10'
accepted: '2026-09-09'
issue: '#230'
status: '承認済み'
---

# RFC-037: 産業化ディストリビューション方案 — cargo-dist に基づくコンパイラ/ツールチェーンパッケージング

> 本 RFC は [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
> と補完関係にある。RFC-014b は
> **YaoXiang パッケージマネージャ**がサードパーティパッケージをどのようにビルド・配布するかを定義しており、本 RFC は
> **YaoXiang コンパイラ/ツールチェーン自体**をどのようにパッケージング・配布するかを定義する。

## 概要

`cargo-dist`（Rust エコシステムのバイナリ配布ツール）にクロスプラットフォームのビルドオーケストレーションを担わせ、自前のスクリプトがリリースパッケージの構造を担当する。中心となる約束は2つ：リリースパッケージは**物理的に標準ライブラリソースコードディレクトリを同梱**し（ユーザが Python の
`Lib/`
を読むように直接読める）、全プラットフォームで動的リンクされる Z3 共有ライブラリをパッケージと一緒に配布する。コマンドモデルは**フロントドア/エンジン分離**：常用コマンド
`yx`（小さなフロントドア、組み込みのバージョン管理 — rustup/Go GOTOOLCHAIN 相当）、エンジン
`yaoxiang-rs`（現在の `yaoxiang`
モノリスから改名）。インストール方式は二層：標準チャネルは Go/Zig に揃えて**リリースパッケージがそのまま製品**、展開 +
PATH；簡単チャネルは一行コマンドでインストール（Linux `apt` / `curl | sh`、Windows `irm | iex` /
Inno exe ウィザード）。`libz3.dll`
の欠落、標準ライブラリがユーザから見えない、CI スクリプトの重複保守といった問題を解決する。

## 動機

### なぜこの機能が必要なのか？

YaoXiang をダウンロードしたユーザは**すぐに使える**べきで、追加のステップは一切不要；標準ライブラリは**直接読める**ようにし、バイナリの中のブラックボックスにしない。

### 現状の問題

#### 問題 1: Windows ユーザがダウンロード後に実行できない

現状の Release は `yaoxiang.exe` のみをアップロードしているが、`libz3.dll`
がパッケージに含まれていない。Windows でユーザがダブルクリックするとエラーになる：

```
The code execution cannot proceed because libz3.dll was not found.
```

これは **致命的なバグ** — ユーザは最初の一歩すら進められない。

#### 問題 2: Release アーティファクトが単一 exe のみで、標準ライブラリがユーザに見えない

現状は三重の断絶：

- Release アーティファクトはベアバイナリのみで、標準ライブラリはリリースに同梱されない
- LSP のインターフェースファイル検索チェーンはほぼ機能していない：`find_std_interface_file`
  を呼び出す際にプロジェクトディレクトリを渡さず（グローバルの `~/.yaoxiang/std/`
  のみ検索し、それを埋めるフローが存在しない）；`package init` は `.yaoxiang/std`
  に書き込むが、検索チェーン上にない
- 標準ライブラリソースコード（`.yx`
  層）とインターフェースビュー（native 層）がユーザにとって完全にブラックボックス

産業化のアプローチ：ユーザが Python の `Lib/`
を直接開くように標準ライブラリディレクトリを直接読めるようにする —
**リリースパッケージが物理的に std ディレクトリを同梱することが本方案の必須要件**（裁決済み）。

#### 問題 3: CI の手書きスクリプトの重複保守

現在、複数のビルドパイプラインを保守している：

| ファイル                  | 役割                       | 行数        |
| ------------------------- | -------------------------- | ----------- |
| `_build-platforms.yml`    | クロスプラットフォーム構築 | ~255 行     |
| `release.yml`             | バージョンリリース         | ~189 行     |
| `nightly.yml`             | デイリービルド             | ~173 行     |
| `scripts/build/setup.iss` | Inno Setup インストーラ    | ~250 行     |
| **合計**                  |                            | **~870 行** |

大部分は重複（Rust インストール → キャッシュ → ビルド → リネーム → アップロード）で、プラットフォームごとに一度ずつ書く必要がある。

#### 問題 4: Inno Setup のバージョン番号のハードコード

`setup.iss` の `MyAppVersion` は `0.7.0` とハードコードされており、ビルド時に `sed`
で置換している。これは迟早必ず失敗する。

#### 問題 5: RFC-014b との境界が曖昧

RFC-014b は「YaoXiang パッケージのビルドと配布メカニズム」（つまり `yaoxiang.toml` の `[build]` と
`[binaries]`
設定）を定義しているが、**「YaoXiang コンパイラ自体をどうリリースするか」をカバーしていない**。本 RFC はこのギャップを埋める。

## 提案

### コア設計

cargo-dist は**ビルドオーケストレーション層のみ**を担当；パッケージ構造とインストーラは全て自前。役割分担：

```
cargo-dist の役割（ビルドオーケストレーション層）:
  ├── クロスプラットフォームコンパイル（5 ターゲット）
  └── 圧縮パッケージとチェックサムの生成
  （ネイティブインストーラと npm ラッパーは廃止 — フラットバイナリ仮定が bin/+lib/ 構造と衝突）

build.rs は引き続き担当:
  └── Z3 ダウンロード/リンク（全プラットフォーム動的 + rpath）

YaoXiang 自前スクリプト:
  ├── package-dist.sh — パッケージ構造の再構成（bin/ + lib/）、共有ライブラリ同梱、
  │   std ディレクトリの充填（リポジトリで事前生成されたインターフェースビュー + .yx 層ソース）、チェックサム再計算
  └── Inno Setup — Windows インストールウィザード（既存資産；完全なディレクトリ構造を構築）

コマンドモデル（フロントドア/エンジン分離）:
  ├── yx — フロントドア（新しい小さな crate）：バージョン解決 + ディスパッチ；toolchain/self の動詞のみ保持、他は透過
  └── yaoxiang-rs — エンジン（現在の yaoxiang モノリスから改名）：コンパイル/実行/パッケージ管理/fmt/lsp サブコマンド

インストール方式（二層）:
  ├── 標準チャネル（Go/Zig モード）: リリースパッケージがそのまま製品、展開 + PATH
  └── 簡単チャネル（Rust モード）: 一行コマンドでインストール + バージョン管理（フロントドア yx に組み込み）
      ├── Linux: apt（自前 deb リポジトリ、システムレベルフラットインストール）/ curl … | sh（パッケージ全体を一発インストール）
      ├── Windows: irm … | iex（パッケージ全体を一発インストール）/ Inno Setup ウィザード（既存資産、システムレベルフラットインストール）
      └── macOS: curl … | sh（brew は homebrew-core コミュニティに任せる）
```

### リリースディレクトリ構造（裁決済み：標準ライブラリソースの物理的同梱）

ユーザは Python の `Lib/`
を読むように直接標準ライブラリを読めるようにする必要がある — リリースパッケージに std ディレクトリを同梱することは必須要件であり、パッケージングの詳細ではない。Z3 についても同様：外部システムとして、ディレクトリ型の共有ライブラリ配布が自然な形態である —
`.so`
を exe に詰め込むことと外に出して動的リンクすることは「どちらもパッケージに同梱する必要がある」点で等価であり、後者はなお交換可能性を保つ。

各プラットフォームのリリースパッケージは、`package-dist.sh` が cargo-dist ビルド後に再構成する：

```
yaoxiang-{version}-{target}.tar.gz / .zip     （ポータブル即使用：展開後 bin/ 内で直接実行）
├── bin/
│   ├── yx                            # フロントドア（または yx.exe）
│   ├── yaoxiang-rs                   # エンジン（または yaoxiang-rs.exe）
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # ユーザが直接読める（Python Lib/ モード）
│           ├── io.yx                 # native モジュール：リポジトリで事前生成されたインターフェースビュー
│           ├── math.yx
│           ├── test.yx               # .yx 層：リポジトリ内の実際のソースをそのままコピー
│           └── ...
├── README.md
└── LICENSE
```

インストール = 任意のディレクトリに展開 + `bin/` を PATH に追加（Go の `/usr/local/go/bin`
モード；`~/.yaoxiang/` への展開が一般的選択）。Windows では Inno
Setup ウィザードが同じことを行う（デフォルト Program Files）。ポータブル展開時、`yx` フロントドアは
`~/.yaoxiang` 状態を持たず、隣接の `yaoxiang-rs`
にフォールバック — マネージドインストールと動作が一致；エンジンの rpath と exe 相対 std 検索はフロントドアの存在によって変わらない。

### プラットフォームサポート

| プラットフォーム | target triple               | 説明                   |
| ---------------- | --------------------------- | ---------------------- |
| Linux x86_64     | `x86_64-unknown-linux-gnu`  | メインプラットフォーム |
| Linux ARM64      | `aarch64-unknown-linux-gnu` | CI でクロスコンパイル  |
| macOS x86_64     | `x86_64-apple-darwin`       | Intel Mac              |
| macOS ARM64      | `aarch64-apple-darwin`      | Apple Silicon          |
| Windows x86_64   | `x86_64-pc-windows-msvc`    | メインプラットフォーム |

合計 5 ターゲット。Windows
ARM64 は当面サポートしない（Z3 公式のプレコンパイル済み ARM64 パッケージがないため）。

### Z3 配布戦略

**全プラットフォーム動的リンク**（再確認の上で維持）：

| プラットフォーム | 変更                   | 成果物        |
| ---------------- | ---------------------- | ------------- |
| Linux            | **静的→動的に変更**    | `libz3.so`    |
| macOS            | **静的→動的に変更**    | `libz3.dylib` |
| Windows          | 変更なし               | `libz3.dll`   |
| wasm32           | 変更なし（静的リンク） | 埋め込み `.a` |

理由：

- **一貫性** — 3 つのプラットフォームで動作が統一され、特例がなくなる
- **これは外部ライブラリなので、共有ライブラリで配布すべき**。Python（`python3.dll`+`DLLs/lib*.dll`）、Node（`node`+`lib/`）もそうしている
- **ユーザが Z3 をアップグレードする際にコンパイラのバージョンを待つ必要がない** —
  `.so`/`.dylib`/`.dll` を置き換えるだけでよい
- **バイナリサイズが小さい** — Z3 は小さくないので、静的リンクは exe を数 MB 膨張させる

動的リンクには**必要な付帯事項**がある：Linux/macOS の動的リンカはデフォルトでバイナリの存在するディレクトリを検索しないので、rpath を注入しないと「展開即使用」が成立しない（Windows はデフォルトで exe ディレクトリを検索するので対処不要）。対応する
`build.rs` の変更：

```rust
// 動的リンクと rpath の統一
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 リリースパッケージのレイアウトは統一されておらず、lib/bin 両ディレクトリを探索
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037: 全プラットフォーム動的リンク。共有ライブラリはリリースパッケージ bin/ とともに配布、ユーザが Z3 をまとめて交換・アップグレード可能
    if target_os == "windows" {
        // MSVC import lib の名前は libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // 動的リンカはデフォルトでバイナリの存在するディレクトリを検索しないので、rpath を注入しなければ「展開即使用」が成立しない
        // （リリースパッケージ内では exe と libz3 は同じ bin/ 内；Windows はデフォルトで exe ディレクトリを検索するので対処不要）
        match target_os.as_str() {
            "linux" => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
            "macos" => println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path"),
            _ => {}
        }
        let cxx = if target_os == "macos" {
            "c++".to_string()
        } else {
            env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into())
        };
        println!("cargo:rustc-link-lib={}", cxx);
    }
}
```

**「全プラットフォーム静的リンク」は目標としない。**これは特殊なケースを取り除くことではなく、間違った方法で合理的なケースを取り除こうとしている。共有ライブラリは外部ライブラリの通常の配布方式である。

### インストーラサポート

主要言語ツールチェーンの配布方式との比較（2026-09 調査）：

| 言語     | 公式配布物                   | 公式インストール方法                                     | インストーラ保守者                     |
| -------- | ---------------------------- | -------------------------------------------------------- | -------------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | 公式ドキュメントがそのまま「ダウンロード → 展開 → PATH」 | なし（brew/apt はコミュニティ）        |
| Zig      | `zig/{bin,lib/std}` tarball  | 同上、公式インストールスクリプトなし                     | なし（homebrew-core コミュニティ）     |
| Node     | `{bin,lib,include}` tarball  | tar + 公式 pkg/msi                                       | チーム自作                             |
| Rust     | 複数コンポーネント tarball   | rustup                                                   | チーム自作                             |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                              | チーム + brew コミュニティ             |
| Deno/Bun | 単一バイナリ zip             | 公式 curl スクリプト                                     | チーム自作（スクリプトは非常に小さい） |
| Gleam    | cargo-dist 単一バイナリ      | cargo-dist が生成するスクリプト                          | cargo-dist                             |

3 つのルール：

- **複数ファイルのツールチェーンで、サードパーティのジェネレータを使ってインストーラを作っているところはない**
  —
  cargo-dist のインストーラは単一バイナリシナリオのみに対応する（Gleam が使えるのはまさに外部依存なしの単一 Rust バイナリだから）
- 最もシンプルなモデルは
  **Go/Zig の「リリースパッケージがそのまま製品」**：公式インストールガイドは展開 +
  PATH で、インストーラコードはゼロ；リリースパッケージが可読な std ソースコード（Go の
  `src/`、Zig の `lib/std/`、Crystal の `src/`）を同梱するのは普通のこと
- curl 一発インストールしたい（Deno/Bun/rustup）ところは全て**自作スクリプト**で、ほとんど進化しない；brew フォーミュラは全て homebrew-core でコミュニティ保守、言語チームは自前の tap を作らない（Crystal チームが明確に formula はコミュニティのものだと言っている）

YaoXiang は二層モデルを採用：

| チャネル                                                | 層   | 状態 | 説明                                                                                                                                      |
| ------------------------------------------------------- | ---- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                            | 標準 | ✅   | 展開即使用（rpath + 同ディレクトリ共有ライブラリ）、extract + PATH が公式ガイド                                                           |
| `yx`（フロントドア、組み込みバージョン管理）            | 簡単 | ✅   | rustup/Go GOTOOLCHAIN 相当：複数バージョンインストール/切替/更新 + プロジェクト pin                                                       |
| `curl ... \| sh`（install.sh）                          | 簡単 | ✅   | Linux / macOS：再構成パッケージをダウンロードして `versions/` に展開、`bin/yx` をインストールルートに配置しデフォルトバージョンを書き込む |
| `irm ... \| iex`（install.ps1）                         | 簡単 | ✅   | Windows：同ロジック                                                                                                                       |
| `apt install yaoxiang`                                  | 簡単 | ✅   | `.deb`（amd64/arm64）+ GitHub Pages 静的 apt リポジトリ；システムレベルフラットインストール、`apt upgrade` でバージョン追従               |
| Inno Setup exe                                          | 簡単 | ✅   | Windows ウィザード（既存資産）、システムレベルフラットインストール、完全な bin/+lib/ 構造を構築                                           |
| winget / `.rpm` / homebrew-core / npm                   | —    | ⏸    | 任意のフォローアップ：winget と brew-core はどちらもコミュニティ保守、rpm と deb は同型                                                   |
| MSI / cargo-dist ネイティブインストーラ / 自前 brew tap | —    | ❌   | 「代替案」を参照                                                                                                                          |

**簡単チャネルは Rust を参照、バージョン管理はフロントドアに組み込む。**Rust の分解は「ブートストラップスクリプト（sh.rustup.rs）→
rustup
→ ツールチェーンパッケージ」で、rustup は最初から複数バージョン、切替、更新を管理している；Python/Node の公式インストーラはこの層を作らず、エコシステムとして後付けで pyenv/nvm/pdm が乱立した。YaoXiang は**フロントドア/エンジン分離**を採用（Go の
`go` フロントドア + GOTOOLCHAIN、rustup のプロキシディスパッチ、両者の同型形態）：常用コマンドは
`yx`、エンジンは `yaoxiang-rs` ——

```
~/.yaoxiang/
├── bin/yx                  # フロントドア：小さなバイナリ、バージョン解決 + ディスパッチ（プロジェクト pin > デフォルト > 隣接エンジン）
├── settings.toml           # デフォルトバージョン、ミラーソース
└── versions/               # バージョンが第一級の概念；<ver>/ がそのバージョンのリリースパッケージの展開ルートディレクトリ
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # エンジン：コンパイル/実行/パッケージ管理/fmt/lsp サブコマンド
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- インストールルート `~/.yaoxiang/` は業界慣例に合わせる（pyenv `~/.pyenv`、nvm `~/.nvm`、deno
  `~/.deno`、bun `~/.bun`、volta `~/.volta` 全て単一ルート；rustup の `~/.cargo`+`~/.rustup`
  の二重ルートは cargo が rustup より先に存在していた歴史的経緯であり、模倣しない）、さらに
  `~/.yaoxiang`
  は既にコードベースの既存の名前空間（std グローバルフォールバックスロット）；`YAOXIANG_HOME`
  環境変数での上書きをサポート（先例 RUSTUP_HOME /
  DENO_INSTALL、CI とコンテナシナリオに対応）；Windows では `%USERPROFILE%\.yaoxiang`
- **バージョンディレクトリ = リリースパッケージ展開ルートディレクトリ**：`versions/<ver>/`
  はポータブル展開、deb インストールツリーと完全に同型で、バージョンをインストールすることはリリースパッケージを展開すること —
  3 つのチャネルで構造の分岐がゼロ
- コマンド面（rustup 相当）：`yx toolchain install / default / update / list / uninstall`、`yx self update`
  を含む；その他の動詞はそのままエンジンに透過
- **バージョンロックは構造的保証**：fmt などのツールは構文と同期して進化する（古い fmt は新しい構文を認識しない）、バージョン解決はフロントドアで一度完了し、ツール群全体を切り替える — 「新しいエンジンに古い fmt」の組み合わせ空間は存在しない；将来 fmt/LSP を独立したバイナリに分割する場合も、同バージョンの
  `bin/` 内に配置される
- プロジェクトレベル pin：`yx-toolchain.toml`（先例 rust-toolchain.toml、コマンド名に追随；`yaoxiang.toml`
  には入れない — パッケージマニフェストがツールチェーンバージョンをライブラリの利用者に強制すべきではない）
- ブートストラップエントリ（`curl | sh` /
  `irm | iex`）が一度に最新の stable パッケージ全体を入れる：`versions/` への展開、`bin/yx`
  をインストールルートに配置、`settings.toml`
  にデフォルトバージョンを書き込み（rustup の「ブートストラップでマネージャをインストール」分解の等価収束 — フロントドアとエンジンが同パッケージであり、二段階不要）
- ミラーソースを設定可能（settings.toml）、国内のユーザへの配慮を継続（Z3 ダウンロードと同じネットワーク問題）
- **バージョン管理は自己完結不変量を破壊しない**：各バージョンは完全なリリースツリーであり、rpath と exe 相対 std 検索はツリー内で自己完結し、フロントドアはディスパッチするだけで構造を変えない

`.deb` と Inno は**システムレベルフラットインストール**チャネル（root / Program
Files 単一バージョン、`apt upgrade`
/ コントロールパネルで更新）、サーバ、CI、完全初心者シナリオ向け；マネージャとの共存は PATH 順序で実現（先例：apt の rustc と rustup の共存）。全チャネルが同じプロダクトツリーを共有する。

`.deb`
レイアウトは同じディレクトリツリーを再利用：`/usr/lib/yaoxiang/`（リリースツリー全体：`bin/{yx,yaoxiang-rs,libz3.so}` +
`lib/yaoxiang/std/`）+ `/usr/bin/yx` シンボリックリンクが `/usr/lib/yaoxiang/bin/yx` を指す —
`$ORIGIN` は解決後の**実際のパス**で計算されるため、リンク後も同じディレクトリの `libz3.so`
にヒットし、展開パッケージ構造と同型。ブランドフルネームはパッケージ名と製品名に残し（`apt install yaoxiang`、Inno 製品名 YaoXiang）、コマンド面は統一
`yx` — Go と同じ：パッケージ名 `golang-go`、コマンド `go`。apt リポジトリは GitHub
Pages で静的ホスティング（Packages/Release/InRelease メタデータを GPG 署名、release
CI が公開）；将来的に Debian/Ubuntu 公式収録を申請可能（サイクルが長く、バージョン遅延、メイン経路ではない）。

### 標準ライブラリディレクトリ

`lib/yaoxiang/std/`
の内容は全てリポジトリの静的ファイルから来ており、パッケージングは**純粋なコピー**で、ランタイム生成のエントリはない：

| 層                                | ソース                                                                      | 性質                                           |
| --------------------------------- | --------------------------------------------------------------------------- | ---------------------------------------------- |
| native モジュール（io/math/…）    | リポジトリ `src/std/interfaces/*.yx` で事前生成されたインターフェースビュー | インターフェース署名ビュー（実装はバイナリ内） |
| .yx 層モジュール（test/… 増加中） | リポジトリ `src/std/*.yx` をそのままコピー                                  | 実際のソース                                   |

事前生成ビューは `StdModule::exports()` から派生（`src/std/gen_interfaces.rs` の
`generate_all_interfaces()`）、**ランタイム生成サブコマンドは設けない**（2026-09-10 裁決：パッケージングが固まった後、サブコマンドは余分なインターフェース面になる）。同期は「生成物入库 + テストゲート」パターンを採用（RFC-013 のコードテーブルと同じ）：`test_committed_interface_files_match_generation`
が事前生成ファイルと現在の生成結果をバイト単位で比較、ズレがあれば赤；治癒は bless エントリ
`cargo test update_committed_interface_files -- --ignored` で行う。生成ロジックは crate 内部の
`StdModule` 実装に依存し、build.rs に降ろせないため、ゲートはテスト段階で構築段階ではなく。

**ランタイム検索チェーン** — `find_std_interface_file` に exe 相対検索レベルを追加：

1. プロジェクト `.yaoxiang/vendor/std/<name>.yx`（プロジェクトオーバーライド、現状）
2. **exe の存在するディレクトリ
   `../lib/yaoxiang/std/<name>.yx`（新規）**：ポータブル展開、マネージドインストール（`versions/<ver>/`）、deb フラットインストールを統一ヒット
3. `~/.yaoxiang/std/<name>.yx`（グローバルフォールバック、手動オーバーライドとして保持）

現状このチェーンはほぼ機能していない（LSP 呼び出しがプロジェクトディレクトリを渡さない、`package init`
が書き込む `.yaoxiang/std` はチェーンにない）、今回は付随的にこれを受け持ち、`package init` の出力を
`.yaoxiang/vendor/std` に統一（パッケージマネージャの vendor ディレクトリと一致）。

**コンパイル権限は変更なし**：`.yx` 層は引き続き `include_str!`
で埋め込まれる（RFC-036 の「std バージョンとバイナリの厳格なバインド」不変量を保持）。リリースディレクトリの位置付けは**可読ビュー +
LSP 解析ソース**であり、コンパイル入力ではない — ユーザがリリースディレクトリ内の `.yx`
を手動で変更してもコンパイラには採用されない（Python 風の「`Lib/`
を変更すれば即有効」のセマンティクスを開放するかどうかは、開放問題を参照）。

### Wasm ビルド

**独立を維持、cargo-dist には移行しない。**

cargo-dist が管理するのは「コンパイラをユーザに配布する」ことで、wasm は「オンライン playground をドキュメントサイトに埋め込む」 —
2 つは完全に異なるデリバリ。

| 側面             | 方法                                     |
| ---------------- | ---------------------------------------- |
| ビルドツール     | `wasm-pack build` を維持                 |
| CI workflow      | `_build-wasm.yml` を独立 job として保持  |
| トリガタイミング | リリースと同じ tag push で並列の独立 job |
| 公開ターゲット   | `docs/public/wasm/` → GitHub Pages       |

### npm 公開

| パッケージ             | 内容                                         | 状態                                                                                                                                                                                                                 |
| ---------------------- | -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@yaoxiang/cli`        | リリースパッケージをダウンロードするラッパー | 延期：cargo-dist の npm ラッパーも同様にフラットアーティファクト仮定に基づき、インストーラとともに廃止；npm チャネルが必要なら、自作ラッパー（再構成パッケージをダウンロードして展開、展開インストールと同ロジック） |
| `@yaoxiang/playground` | wasm ライブラリ（JS + .wasm）                | 任意、現在は docs のみ公開                                                                                                                                                                                           |

両者は競合せず、名前も競合しない。

### 既存リリースフローとの統合

現状の `release.yml`：push main → check-version（`v{version}` tag が存在しない場合のみ通過）→ build
/ build-wasm / security / test の 4 経路 → release job（tag を作成して push +
`generate-commit-list.ts` が @mentions を含む body を生成 + アーティファクトをアップロード）。

cargo-dist が生成するパイプラインは tag 駆動で、announce/publish を含み、fmt/clippy/test/audit ゲートを含まず、release
notes フォーマットも merge commit
changelog を運べない。**全体を入れ替えると既存リリースセレモニーを壊す**（PR → CI 全て緑 → bump →
merge commit が changelog）。

統合原則：**トリガとゲートは現状維持、ビルドは `cargo dist build` に委ねる、公開は現状維持。**

1. check-version / security / test の 3 つの job は現状維持（push
   main でトリガ、tag 作成前のゲート）
2. 全て通過後、release job が `v{version}` tag を作成して push（現状変更なし）
3. tag push が新しい `dist-release.yml` をトリガ：plan
   job が dist で runner/システム依存関係マトリクスを計算 → `cargo dist build`（5 ターゲット）→
   `package-dist.sh` がターゲットごとに再構成 → Inno Setup
   job（Windows の再構成パッケージをウィザードビルドに消費、`/DMyAppVersion=`
   でバージョンを注入、二度目のコンパイルなし）→ `_build-wasm.yml`（並列 job）
4. publish job：`generate-commit-list.ts` が body を生成（既存スクリプト再利用）→ 再構成パッケージ +
   `.sha256` + `.deb` + wasm + Setup exe のアップロードを追加；独立した `publish-apt` job が GitHub
   Pages apt リポジトリメタデータを公開（`secrets.APT_GPG_KEY`
   未設定時は自動スキップ、他チャネルに影響なし）

### Nightly リリース

cargo-dist にはネイティブの nightly サポートがない（[axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143)、依然として open
feature request）。

既存の cron + tag 上書きソリューションを維持し、ビルド部分を `_build-platforms.yml` から
`cargo dist build`
に変更 — それは本質的に cargo コマンドなので、nightly.yml で直接呼び出せる。workflow 再利用はしない（想定していた
`uses: ./release.yml` は不可：再利用される側に `workflow_call` トリガーが必要、かつ cargo-dist
workflow は tag 駆動で、ビルドと公開が結合）：

```yaml
# nightly.yml（移行後）
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh（正式版と同じセット）
  publish: # 現状維持：nightly tag を作成/移動 → GitHub Pre-release を上書き
```

### cargo-dist 設定（既に dist-workspace.toml に実装済み）

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# dist バージョンをロック（Cargo.toml SemVer 構文）
cargo-dist-version = "0.32.0"
ci = "github"
# インストーラは全て自前、cargo-dist はビルド + 圧縮パッケージ + チェックサムのみ
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# 生成された workflow は意図的に改変されている（package-dist.sh による再構成 + 自前公開セグメント、RFC-037）、
# generate テンプレートからのドリフトによりビルドを拒否しない
allow-dirty = ["ci"]
```

以上はリポジトリ vendor の実際の設定（`cargo dist init`
で生成後、必要に応じて修正）；`cargo-dist-version`
を 0.32.0 にロック、生成された workflow をリポジトリに vendor して review を受け、ランタイム取得はしない。ビルドプロファイルは init がルート Cargo.toml の
`[profile.dist]` に注入（inherits release、lto=thin）、2 つのバイナリを `target/<triple>/dist/`
に配置し再構成スクリプトが取り出す。

### package-dist.sh（既に実装済み）

リポジトリの `scripts/release/package-dist.sh` を基準とする、要点：

- 2 つのバイナリは `cargo dist build` のビルド出力ディレクトリ
  `target/<triple>/dist/`（profile=dist）から直接取得 —
  cargo-dist 自体のフラット単一バイナリアーカイブはデリバリではなく、同名の再構成パッケージを
  `target/distrib/` で直接上書き
- Z3 共有ライブラリも同様にビルド出力ディレクトリから取得 —
  build.rs のリンク時に対応するプラットフォームの共有ライブラリを選択して配置（`copy_shared_lib`）、**単一ソース**：パッケージングスクリプトは Z3 のバージョンとプラットフォームディレクトリ名を知る必要がないし、知る必要もない；Z3 ライセンステキストがライブラリとともに配置され、パッケージングで同梱される（MIT 配布義務）、macOS 側ではパッケージング時に dylib の install_name を
  `@rpath` に統一し、バイナリに ad-hoc 再署名
- std ディレクトリは純粋コピー：`src/std/interfaces/*.yx`（事前生成されたインターフェースビュー）+
  `src/std/*.yx`（.yx 層の実際のソース）
- README/LICENSE を同梱；再パッケージング（Windows zip / その他 tar.gz；Git
  Bash に zip がない場合 zip → System32 bsdtar → PowerShell の 3 段階フォールバック）し `.sha256`
  を再計算
- Linux かつ `dpkg-deb` が利用可能な場合、ついでに `build-deb.sh` を呼び出して `.deb`
  を生成（`/usr/lib/yaoxiang` フラットインストールツリー + `/usr/bin/yx` シンボリックリンク）

### 廃止された手書き CI

移行完了後に調整されるファイル：

| ファイル                                 | 行数        | 処分                                                             |
| ---------------------------------------- | ----------- | ---------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254         | 削除（cargo-dist ビルドマトリクスで代替）                        |
| `.github/workflows/release.yml`          | 189         | ゲート + tag 作成に縮小（ビルド/公開は dist-release.yml に移動） |
| `.github/workflows/nightly.yml`          | 173         | ビルドセグメントを `cargo dist build` に変更、公開ロジックは保持 |
| `scripts/build/setup.iss`                | ~250        | **保持して正式化**（Windows ウィザード）                         |
| **合計削減**                             | **~600 行** |                                                                  |

保持されるもの：

- `ci.yml`（日常の fmt + clippy + test + MSRV、公開フローに属さない）
- `_build-wasm.yml`（独立したビルドフロー、dist-release.yml の並列 job に組み込み）
- `_build-z3-wasm.yml`（wasm 専用 Z3）
- `docs-deploy.yml`（ドキュメントデプロイ）

### 受け入れ基準

「すぐに使える」はテスト可能であり、移行完了の判定は「新旧アーティファクトが一致」ではなく、以下が全て通ることである：

- クリーンなマシン（Rust なし / Z3 なし / `~/.yaoxiang`
  なし）でいずれかのプラットフォームの圧縮パッケージを展開し、直接 `bin/yaoxiang-rs --version`
  を実行して成功 — `LD_LIBRARY_PATH` を設定しない（rpath が有効）
- 展開ディレクトリ内の `lib/yaoxiang/std/*.yx`
  が全て可読：native モジュールは署名インターフェースビュー、`.yx` 層は実際のソース
- 展開ディレクトリ下のサンプルプロジェクトで LSP を起動し、std メンバーの補完 / ジャンプ定義が使える（exe 相対検索が有効）
- 公式ガイドに従って `/usr/local`（または
  `~/.yaoxiang`）に展開して PATH に追加した後、任意のディレクトリで `yx --version` が成功
- `apt install yaoxiang`（自前リポジトリ）後に実行可能、`apt upgrade`
  でバージョン追従；`/usr/bin/yx` シンボリックリンク下でエンジンの `$ORIGIN`（実際のパスで解決）が
  `bin/libz3.so` にヒット
- `curl ... | sh` と `irm ... | iex` をクリーンな環境で実行した後 `yx` が実行可能、PATH が設定される
- `yx toolchain install <ver>` / `default` / `update`
  が有効：複数バージョンの共存、`yx-toolchain.toml`
  のプロジェクト pin がデフォルトバージョンより優先、フロントドアが正しいバージョンにディスパッチ（バージョンツリー内で rpath と std 検索が自己完結、「新しいエンジンに古い fmt」の組み合わせなし）
- ポータブル展開後 `yx` が隣接の `yaoxiang-rs` にフォールバック、動作がマネージドインストールと一致
- Inno Setup インストール後にディレクトリ構造が完成、PATH が有効、アンインストール可能
- Release アセット完備：5 プラットフォームの再構成パッケージ + `.sha256` が実際の内容と一致
- Release body は `generate-commit-list.ts` の出力（merge commit changelog 完備）
- nightly アーティファクトは Pre-release、最新の正式 tag に影響しない

## トレードオフ

### 利点

- **すぐ使える**
  — ポータブル展開即使用（rpath + 同ディレクトリ共有ライブラリ）、インストーラが完全なディレクトリを構築
- **標準ライブラリが読める** — ユーザが Python `Lib/` を読むように直接 std を読める（必須要件達成）
- **保守コストの削減** — 手書きビルド YAML 約 600 行を cargo-dist + 自前スクリプト約 80 行に
- **クロスプラットフォームの一貫性**
  — 全プラットフォーム動的リンク + 同ディレクトリ共有ライブラリ、特例なし
- **インストールの二層**
  — 標準チャネルはコード追加ゼロ（Go/Zig モード）；簡単チャネルは Rust を参照、apt / curl / iex /
  exe の 4 エントリが同じアーティファクト構造を共有
- **バージョン管理組み込み** — フロントドア `yx` が rustup/Go
  GOTOOLCHAIN 相当、Python/Node が pyenv/nvm/pdm で事後補講する生態分裂を回避；ツールのバージョンロックは構造保証であり約束ではない

### 欠点とリスク

- **簡単チャネルの保守面** — install.sh / install.ps1（一発スクリプト、ほとんど進化しない）+ `.deb`
  と apt リポジトリメタデータ公開（release CI で自動化）+ `yx` フロントドア crate
- **エンジン改名の影響範囲** — `yaoxiang` → `yaoxiang-rs`
  は CI アーティファクト名、Inno、テスト、ドキュメントの一回限りの移行が必要（フェーズ 5 内で完了）
- **学習コスト** — チームは cargo-dist 設定を学ぶ必要がある
- **cargo-dist の上流リスク** —
  2025年半ばに Axo の活動停止に伴い停滞、同年 9 月に原作者が復活し継続的にリリース（0.29 →
  0.32+）；`dist-version` ロック + 生成物の vendor 化によるリポジトリ review で緩和
- **cargo-dist にネイティブの nightly がない** — nightly 公開部分はいまだ手書き

### RFC-014b との関係

|              | RFC-014b                                  | RFC-037                                      |
| ------------ | ----------------------------------------- | -------------------------------------------- |
| **範囲**     | サードパーティパッケージのビルドと配布    | コンパイラ自体のパッケージングと配布         |
| **ツール**   | `yaoxiang build` / `yaoxiang publish`     | `cargo-dist` + 自前スクリプト                |
| **成果物**   | サードパーティパッケージの FFI ライブラリ | コンパイラ + 標準ライブラリ + ツールチェーン |
| **相互排他** | いいえ、補完                              | いいえ、補完                                 |

## 代替案

| 案                                           | なぜ選ばないか                                                                                                                                                                                         |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **手書き CI を続ける**                       | 既に約 870 行手書き、重複労働、DLL 漏れが発生しやすい                                                                                                                                                  |
| **自前でパッケージングツールを書く**         | 車輪の再発明をしない、cargo-dist は既に成熟                                                                                                                                                            |
| **tar.gz のみでインストーラなし**            | 唯一の公式チャネルは展開 + PATH；Inno は Windows ウィザード慣習のためだけにサービス（国内ユーザは保持を裁決）                                                                                          |
| **Docker 配布**                              | コンパイラと言語ツールチェーンはコンテナシナリオではなくネイティブバイナリが必要                                                                                                                       |
| **自前 Homebrew tap**                        | tap は全てコミュニティ保守（homebrew-core）、自前構築は時期尚早なニーズ；簡単チャネルの macOS エントリは curl スクリプト                                                                               |
| **独立した `yaoxiangup` マネージャバイナリ** | rustup と同じ先例として可能；しかし 2 つ目のユーザ動詞を作り出し、「パッケージマネージャ/fmt も独立すべきか」という対称性の問いを生む — フロントドア/エンジン分離で一挙に解消（2026-09-09 議論で否決） |
| **自己更新のみ、複数バージョンなし**         | 単一バージョン自己更新は複数プロジェクトが異なるバージョンを pin するニーズを解決しない；Python/Node は公式バージョン管理を欠き、生態系が pyenv/nvm/pdm の出現を強制 — 組み込みを裁決（2026-09-09）    |
| **Z3 全静的リンク**                          | 裁決済みで否決 — 外部システムのディレクトリ型共有ライブラリ配布が自然な形態；exe に詰め込むことと外に出すことが「どちらもパッケージに同梱する」点で等価であり、交換可能性を失う                        |
| **Inno Setup を廃止**                        | 裁決済みで否決 — Windows ウィザードとして保持（追加チャネル）                                                                                                                                          |
| **cargo-dist ネイティブインストーラ**        | フラットバイナリ仮定が bin/+lib/ 構造と衝突、インストール直後にライブラリ不足                                                                                                                          |
| **自前 WiX/MSI**                             | cargo-dist ジェネレータを失った後、コストが Inno が既にカバーする価値を上回る                                                                                                                          |

## 実装戦略

### フェーズ 1: 言語側の変更（P0）

1. `build.rs`：全プラットフォーム統一動的リンク + rpath link-arg；`copy_dll()` を
   `copy_shared_lib()` に拡張（so/dylib/dll）
2. リポジトリに native インターフェースビューを事前生成（`src/std/interfaces/`）、テストゲートで
   `StdModule::exports()` との同期を強制（gen-std サブコマンドは裁決済みでキャンセル）
3. `find_std_interface_file` に exe 相対検索ブランチを追加；`package init` の出力パスを
   `.yaoxiang/vendor/std` に統一

### フェーズ 2: cargo-dist 導入（P0）

1. `cargo dist init` を実行して初期設定を生成（`installers = []`、dist-version をロック）
2. `package-dist.sh` を記述（再構成 + .yx ソースコピー + チェックサム再計算）
3. 新しい `dist-release.yml` を作成（tag 駆動：dist build → 再構成 →
   wasm 並列 → 自前 publish）；`release.yml` をゲート + tag 作成に縮小
4. 新旧パイプラインを並行実行し、受け入れ基準に従って一つずつ検証

### フェーズ 3: 旧 CI の停止（P1）

1. 確認後 `_build-platforms.yml` を削除
2. `nightly.yml` のビルドセグメントを `cargo dist build` に変更
3. `setup.iss`
   を新アーティファクト構造に接続（Inno 正式化；バージョン番号を Cargo.toml から注入、sed 置換を排除）

### フェーズ 4: 簡単チャネル（P2）

1. `install.sh` / `install.ps1`（プラットフォーム検出 → 最新再構成パッケージをダウンロード →
   `versions/` に展開 → `bin/yx` をインストールルートに配置 → `settings.toml`
   にデフォルトバージョンを書き込み →
   PATH のヒント/書き込み；フェーズ 5 と同じラウンドで実装、最終形まで一気通貫）
2. `.deb` パッケージング（`package-dist.sh` と同じディレクトリツリー + `/usr/bin`
   シンボリックリンクを再利用）+ GitHub Pages 静的 apt リポジトリ（メタデータを GPG 署名、release
   CI が公開）

### フェーズ 5: フロントドア yx とエンジン改名（P2）

1. 現モノリスを `yaoxiang-rs`
   に改名（CI アーティファクト名、Inno、テスト、ドキュメントを全面的に見直し、一回限りの移行）
2. 新しいフロントドア小 crate
   `yx`（workspace メンバー）を追加：バージョン解決、リリースアーティファクトのダウンロード、tar/zip 展開、settings.toml、ディスパッチ（toolchain/self の動詞のみ保持、他は透過、手書きディスパッチで clap を使わず引数の逐語転送を保証）；バージョンインデックスは GitHub
   Releases latest API から取得（ミラーソースは ghproxy スタイルプレフィックス連結）；`yx`
   コマンド名は衝突チェック済み（主要ディストリビューション/Homebrew に同名の常用コマンドなし、ある小ツールのみが yx をエイリアスとして使用）
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml`
   プロジェクト pin + ミラーソース + `yx self update`
4. ブートストラップスクリプト：`curl | sh` / `irm | iex`
   → 再構成パッケージをダウンロードしてフロントドアとデフォルト stable を一度にインストール（フェーズ 4 と同じラウンドで最終形として実装）

### フェーズ 6: 任意のフォローアップ（いずれもブロックしない）

1. winget 提出（Inno exe を指す、コミュニティ保守、homebrew-core モードと対称）
2. `.rpm`（dnf ユーザ、`.deb` と同型）
3. Homebrew：homebrew-core 参入基準に達したらコミュニティが提出
4. npm `@yaoxiang/cli` の自作ラッパー（名称は現在未登録）

## 開放問題

### 未決

- **.yx 層に「リリースディレクトリを手動で変更すればコンパイルで採用される」Python 式のセマンティクスを提供するか？**
  デフォルトは否 — コンパイル権限は RFC-036 の埋め込みを維持（std バージョンとバイナリの厳格なバインド）、リリースディレクトリは可読ビュー +
  LSP 解析ソースとして位置付け。将来開放する場合、バージョンバインド不変量を再検討する必要あり。

### 既に解決済み

以下の問題は設計議論で解決済み：

- ~~Windows での Z3 静的リンクの実現可能性？~~ →
  **静的リンクはしない、全プラットフォーム動的**（2026-09-09 再確認の上で維持）
- ~~gen-std-interfaces サブコマンドの命名？~~ →
  **サブコマンドは設けない**（2026-09-10 裁決：通常のパッケージングが固まった後、サブコマンド面は余分；native インターフェースビューはリポジトリ事前生成
  `src/std/interfaces/` + テストゲート同期に変更、パッケージングは純粋コピー）
- ~~Inno Setup を保持するか？~~ → **Windows ウィザードとして保持（追加チャネル）**
- ~~リリースパッケージ構造は標準ライブラリソースを物理的に同梱するか？~~ →
  **必須**（ユーザ可読性が Python `Lib/` に揃う、2026-09-09 裁決）
- ~~cargo-dist ネイティブインストーラ（shell/powershell/homebrew/msi/npm）？~~ →
  **全て廃止**、インストーラは自前（フラット仮定が bin/+lib/ と衝突）
- ~~インストーラ戦略？~~ →
  **二層**（2026-09-09 終裁）：標準チャネル = リリースパッケージがそのまま製品（Go/Zig モード、展開 +
  PATH）；簡単チャネルは Rust を参照して一行コマンドインストール — Linux
  `apt`（自前 deb リポジトリ）/ `curl | sh`、Windows `irm | iex` / Inno
  exe。やらない：MSI、cargo-dist ネイティブインストーラ、自前 brew tap
- ~~バージョン管理マネージャを含めるか？~~ →
  **必須、インストーラ体系に属する**（2026-09-09 裁決、同日の早朝「将来独立 RFC」の境界を覆す）：動機は Python/Node が公式バージョン管理を欠き、pyenv/nvm/pdm で補講する生態分裂
- ~~バージョン管理マネージャの形態：独立バイナリかサブコマンドか？~~ →
  **フロントドア/エンジン分離**（2026-09-09 終裁、3 回の収束 A→C→命名の反転）：常用コマンド `yx`
  = フロントドア（小さなバイナリ、組み込み toolchain/self 動詞）、エンジン `yaoxiang-rs` = 現
  `yaoxiang` モノリスの改名；ディレクトリ構造 `versions/<ver>/`
  — バージョンが第一級の概念、バージョンディレクトリがリリースパッケージの展開ルートディレクトリ、ツールはバージョンごとに一括ロック（古い fmt は新しい構文を認識しない；かつて想定していた内層
  `toolchains/` は冗長としてキャンセル）。先例：Go `go` フロントドア +
  GOTOOLCHAIN、rustup プロキシディスパッチ
- ~~cargo-dist extra-artifacts の条件付き実行？~~ → **`package-dist.sh` スクリプトで処理、shell
  case 分岐で**
- ~~標準ライブラリインターフェースバージョンの互換性？~~ →
  **コンパイラのバージョンと一緒にリリース、同じ圧縮パッケージ内**

## 参考文献

- [cargo-dist 公式ドキュメント](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 ビルド設定 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

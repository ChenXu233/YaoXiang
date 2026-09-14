---
title:
  'RFC-037: 産業化ディストリビューション方案 — cargo-dist
  ベースのコンパイラ/ツールチェーンパッケージング'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-09-10'
accepted: '2026-09-09'
issue: '#230'
status: '承認済み'
---

# RFC-037: 産業化ディストリビューション方案 — cargo-dist ベースのコンパイラ/ツールチェーンパッケージング

> 本 RFC は [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
> と補完関係にある。RFC-014b は
> **YaoXiang パッケージマネージャ**がサードパーティパッケージをビルド・配布する方法を定義する。本 RFC は
> **YaoXiang コンパイラ/ツールチェーン自体**のパッケージングと配布方法を定義する。

## 概要

`cargo-dist`（Rust エコシステムのバイナリ配布ツール）でクロスプラットフォームビルドのオーケストレーションを担い、独自スクリプトで配布パッケージの構造を担当する。中心となるコミットメントは 2 つ：配布パッケージは**物理的に標準ライブラリのソースコードディレクトリを同梱**し（ユーザは Python の
`Lib/`
を読み込むように直接読める）、全プラットフォームで動的リンクされる Z3 共有ライブラリをパッケージに同梱する。コマンドモデルは**フロントドア/エンジン分離**：常用コマンドは
`yx`（小さなフロントドア、rustup/Go GOTOOLCHAIN 相当のバージョン管理を内蔵）、エンジンは
`yaoxiang-rs`（現 `yaoxiang`
モノリスのリネーム）。インストール方法は二層：標準経路は Go/Zig 揃えで**配布パッケージ即プロダクト**（展開 +
PATH）、簡易経路はワンライナーインストール（Linux `apt` / `curl | sh`、Windows `irm | iex` / Inno
exe ウィザード）。`libz3.dll`
欠落、標準ライブラリがユーザに見えない、CI スクリプトの重複保守などの問題を解決する。

## 動機

### なぜこの機能が必要か？

YaoXiang をダウンロードしたユーザは**箱から出してすぐ使える**べきで、いかなる追加ステップも必要としない；標準ライブラリは**ユーザが直接読める**べきで、バイナリの中のブラックボックスに隠されているべきではない。

### 現状の問題

#### 問題 1：Windows ユーザがダウンロード後に実行できない

現状の Release には `yaoxiang.exe` しかアップロードされておらず、`libz3.dll`
がパッケージに含まれていない。Windows ユーザがダブルクリックすると以下のエラーが出る：

```
The code execution cannot proceed because libz3.dll was not found.
```

これは**中断性バグ**であり、ユーザは最初の一歩で先に進めない。

#### 問題 2：Release アーティファクトが単一 exe のみで、標準ライブラリがユーザに見えない

現状は三重の断絶：

- Release アーティファクトは生のバイナリのみで、標準ライブラリが配布物に同梱されていない
- LSP のインターフェースファイル検索チェーンはほぼ失联状態：`find_std_interface_file`
  呼び出し時にプロジェクトディレクトリを渡さず（グローバルの `~/.yaoxiang/std/`
  のみ検索し、それを埋めるフローが存在しない）；`package init` が書き込むのは `.yaoxiang/std`
  で、検索チェーン上にない
- 標準ライブラリソース（`.yx`
  レイヤ）とインターフェースビュー（native レイヤ）がユーザには完全にブラックボックス

産業的なアプローチ：ユーザは Python の `Lib/`
を開くように標準ライブラリディレクトリを直接開いてソースを読めるべきで、**配布パッケージが物理的に std ディレクトリを同梱することは本方案の必須要件**（裁定済み）。

#### 問題 3：CI の手書きスクリプトの重複保守

現在、複数のビルドパイプラインを保守している：

| ファイル                  | 責務                          | 行数        |
| ------------------------- | ----------------------------- | ----------- |
| `_build-platforms.yml`    | クロスプラットフォーム ビルド | ~255 行     |
| `release.yml`             | バージョンリリース            | ~189 行     |
| `nightly.yml`             | デイリービルド                | ~173 行     |
| `scripts/build/setup.iss` | Inno Setup インストーラ       | ~250 行     |
| **合計**                  |                               | **~870 行** |

大部分は重複（Rust インストール → キャッシュ → ビルド → リネーム → アップロード）で、プラットフォームごとに書き直す必要がある。

#### 問題 4：Inno Setup のバージョン番号ハードコード

`setup.iss` 内の `MyAppVersion` が `0.7.0` とハードコードされており、ビルド時に `sed`
で置換している。迟早事故になる。

#### 問題 5：RFC-014b との境界が曖昧

RFC-014b は「YaoXiang パッケージのビルド・配布機構」（すなわち `yaoxiang.toml` の `[build]` と
`[binaries]`
設定）を定義しているが、**「YaoXiang コンパイラ自体をどうリリースするか」はカバーしていない**。本 RFC はこのギャップを埋める。

## 提案

### 中核設計

cargo-dist は**ビルドオーケストレーション層**のみを担う；パッケージ構造とインストーラは独自実装。責務分担：

```
cargo-dist の責務（ビルドオーケストレーション層）:
  ├── クロスプラットフォームコンパイル（5 ターゲット）
  └── 圧縮パッケージとチェックサムの生成
  （ネイティブインストーラと npm wrapper は廃止 — それらのフラットバイナリ前提が bin/+lib/ 構造と衝突するため）

build.rs は引き続き担当:
  └── Z3 ダウンロード/リンク（全プラットフォーム動的 + rpath）

YaoXiang 独自スクリプト:
  ├── package-dist.sh — パッケージ構造の再編成（bin/ + lib/）、共有ライブラリの同梱、
  │   std ディレクトリの充填（リポジトリで事前生成したインターフェースビュー + .yx レイヤソース）、チェックサム再計算
  └── Inno Setup — Windows インストールウィザード（既存資産；完全なディレクトリ構造を敷設）

コマンドモデル（フロントドア/エンジン分離）:
  ├── yx — フロントドア（新規小 crate）：バージョン解決 + ディスパッチ；toolchain/self 動詞のみ保持、他は透過
  └── yaoxiang-rs — エンジン（現 yaoxiang モノリスのリネーム）：コンパイル/実行/パッケージ管理/fmt/lsp サブコマンド

インストール方法（二層）:
  ├── 標準経路（Go/Zig 方式）: 配布パッケージ即プロダクト、展開 + PATH
  └── 簡易経路（Rust 方式）: ワンライナーインストール + バージョン管理（yx フロントドアに内蔵）
      ├── Linux: apt（自前 deb リポジトリ、システムレベル展開）/ curl … | sh（ワンクリックで完全パッケージ）
      ├── Windows: irm … | iex（ワンクリックで完全パッケージ）/ Inno Setup ウィザード（既存資産、システムレベル展開）
      └── macOS: curl … | sh（brew は homebrew-core コミュニティに委ねる）
```

### 配布ディレクトリ構造（裁定済み：標準ライブラリソースを物理同梱）

ユーザは Python の `Lib/`
を読むように直接標準ライブラリを読めるべきで、配布パッケージが std ディレクトリを同梱することは必須要件であり、パッケージング詳細ではない。Z3 も同様：外部システムとして、ディレクトリ型の共有ライブラリ配布が自然な形態である—.so` を exe に埋め込むことと外に出して動的リンクすることは「どちらもパッケージ同梱が必要」において等価で、後者には交換可能性が保持される。

各プラットフォームの配布パッケージは、`package-dist.sh` が cargo-dist ビルド後に再構成する：

```
yaoxiang-{version}-{target}.tar.gz / .zip     （ポータブル即使用：展開後 bin/ から直接実行）
├── bin/
│   ├── yx                            # フロントドア（または yx.exe）
│   ├── yaoxiang-rs                   # エンジン（または yaoxiang-rs.exe）
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # ユーザが直接読める（Python Lib/ 方式）
│           ├── io.yx                 # native モジュール：リポジトリで事前生成したインターフェースビュー
│           ├── math.yx
│           ├── test.yx               # .yx レイヤ：リポジトリ内の実ソースをそのままコピー
│           └── ...
├── README.md
└── LICENSE
```

インストール = 任意のディレクトリに展開 + `bin/` を PATH に追加（Go の `/usr/local/go/bin`
方式；`~/.yaoxiang/` への展開が一般的）。Windows では Inno
Setup ウィザードが同じことを行う（デフォルト Program Files）。ポータブル展開時、`yx` フロントドアは
`~/.yaoxiang` 状態を持たず、隣接の `yaoxiang-rs`
にフォールバックする—マネージドインストールと動作は一致；エンジンの rpath と exe 相対 std 検索はフロントドアの存在によって変わらない。

### プラットフォームサポート

| プラットフォーム | target triple               | 説明                  |
| ---------------- | --------------------------- | --------------------- |
| Linux x86_64     | `x86_64-unknown-linux-gnu`  | メイン                |
| Linux ARM64      | `aarch64-unknown-linux-gnu` | CI でクロスコンパイル |
| macOS x86_64     | `x86_64-apple-darwin`       | Intel Mac             |
| macOS ARM64      | `aarch64-apple-darwin`      | Apple Silicon         |
| Windows x86_64   | `x86_64-pc-windows-msvc`    | メイン                |

合計 5 ターゲット。Windows
ARM64 は当面未対応（Z3 公式のプリコンパイル ARM64 パッケージが存在しないため）。

### Z3 配布戦略

**全プラットフォーム動的リンク**（再確認の上維持）：

| プラットフォーム | 変更点                 | 成果物        |
| ---------------- | ---------------------- | ------------- |
| Linux            | **静的→動的に変更**    | `libz3.so`    |
| macOS            | **静的→動的に変更**    | `libz3.dylib` |
| Windows          | 変更なし               | `libz3.dll`   |
| wasm32           | 変更なし（静的リンク） | 埋め込み `.a` |

理由：

- **一貫性** — 3 プラットフォームの挙動が統一され、もはや特例がない
- **これは外部ライブラリなので、共有ライブラリで配布すべき**。Python（`python3.dll`+`DLLs/lib*.dll`）、Node（`node`+`lib/`）も同じ方式
- **ユーザの Z3 アップグレードはコンパイラのバージョンを待たない** — `.so`/`.dylib`/`.dll`
  を差し替えるだけ
- **バイナリサイズが小さい** — Z3 は小さくなく、静的リンクは exe を数 MB 膨張させる

動的リンクには**必要な付帯措置**がある：Linux/macOS の動的リンカはデフォルトでバイナリのあるディレクトリを検索しないため、rpath を注入しなければ「展開即使用」が成立しない（Windows はデフォルトで exe ディレクトリを検索するため、処理不要）。対応する
`build.rs` の修正：

```rust
// 動的リンク + rpath 統一
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 配布パッケージのレイアウトは統一されておらず、lib/bin 両ディレクトリを探索
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037：全プラットフォーム動的リンク。共有ライブラリは bin/ に同梱して配布し、ユーザは Z3 をまとめて置換アップグレード可能
    if target_os == "windows" {
        // MSVC import lib の名前は libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // 動的リンカはデフォルトでバイナリのあるディレクトリを検索しないため、rpath を注入しなければ「展開即使用」が成立しない
        // （配布パッケージ内で exe と libz3 は同じ bin/ 内；Windows はデフォルトで exe ディレクトリを検索するため、処理不要）
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

**「全プラットフォーム静的リンク」は目標としない。**これは特例の除去ではなく、合理的な状況を間違ったやり方で除去することになる。共有ライブラリは外部ライブラリの正常な配布形態である。

### インストーラサポート

主要言語ツールチェーンの配布方式との対比（2026-09 調査）：

| 言語     | 公式配布物                   | 公式インストール方式                           | インストーラ保守者                 |
| -------- | ---------------------------- | ---------------------------------------------- | ---------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | 公式ドキュメントがそのまま「DL → 展開 → PATH」 | なし（brew/apt はコミュニティ）    |
| Zig      | `zig/{bin,lib/std}` tarball  | 同上、公式インストールスクリプトなし           | なし（homebrew-core コミュニティ） |
| Node     | `{bin,lib,include}` tarball  | tar + 公式 pkg/msi                             | チーム自作                         |
| Rust     | 複数コンポーネント tarball   | rustup                                         | チーム自作                         |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                    | チーム + brew コミュニティ         |
| Deno/Bun | 単一バイナリ zip             | 公式 curl スクリプト                           | チーム自作（スクリプト極小）       |
| Gleam    | cargo-dist 単一バイナリ      | cargo-dist 生成スクリプト                      | cargo-dist                         |

3 つの法則：

- **複数ファイルツールチェーンでサードパーティジェネレータを使ってインストーラを作る例は皆無**—cargo-dist のインストーラは単一バイナリシナリオにのみ対応（Gleam で使えるのは、外部依存なしの単一 Rust バイナリだからこそ）
- 最もシンプルなモデルは **Go/Zig の「配布パッケージ即プロダクト」**：公式インストールガイドは展開 +
  PATH で、インストーラコードはゼロ；配布パッケージに可読な std ソースが同梱されている（Go の
  `src/`、Zig の `lib/std/`、Crystal の `src/`）のが通常
- curl ワンライナーリストール方式（Deno/Bun/rustup）は**全て自作スクリプト**でほぼ進化しない；brew
  formula は一律コミュニティが homebrew-core で保守し、言語チームは自前 tap を作らない（Crystal チームが明確に「formula はコミュニティのもの」と発言）

YaoXiang は二層モデルを採用：

| 経路                                                    | 層   | 状態 | 説明                                                                                                                                      |
| ------------------------------------------------------- | ---- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                            | 標準 | ✅   | 展開即使用（rpath + 同ディレクトリ共有ライブラリ）、展開 + PATH が公式ガイド                                                              |
| `yx`（フロントドア、バージョン管理内蔵）                | 簡易 | ✅   | rustup/Go GOTOOLCHAIN 相当：複数バージョンインストール/切替/更新 + プロジェクト pin                                                       |
| `curl ... \| sh`（install.sh）                          | 簡易 | ✅   | Linux / macOS：再構成パッケージをダウンロードして `versions/` に展開、`bin/yx` をインストールルートに配置しデフォルトバージョンを書き込む |
| `irm ... \| iex`（install.ps1）                         | 簡易 | ✅   | Windows：同ロジック                                                                                                                       |
| `apt install yaoxiang`                                  | 簡易 | ✅   | `.deb`（amd64/arm64）+ GitHub Pages 静的 apt リポジトリ；システムレベル展開、`apt upgrade` でバージョン追従                               |
| Inno Setup exe                                          | 簡易 | ✅   | Windows ウィザード（既存資産）、システムレベル展開、完全な bin/+lib/ 構造を敷設                                                           |
| winget / `.rpm` / homebrew-core / npm                   | —    | ⏸    | オプション追従：winget と brew-core は共にコミュニティ保守、rpm は deb と同型                                                             |
| MSI / cargo-dist ネイティブインストーラ / 自作 brew tap | —    | ❌   | 「代替案」セクション参照                                                                                                                  |

**簡易経路は Rust を参照し、バージョン管理はフロントドアに内蔵。**Rust の分解は「ブートストラップスクリプト（sh.rustup.rs）→
rustup
→ ツールチェーンパッケージ」で、rustup は最初から複数バージョン、切替、更新を管理する；Python/Node の公式インストーラはこの層を作らず、エコシステム，事後的に pyenv/nvm/pdm を分裂的に生んだ。YaoXiang は**フロントドア/エンジン分離**を採用（Go の
`go` フロントドア + GOTOOLCHAIN、rustup のプロキシディスパッチ、両者の同型構造）：常用コマンドは
`yx`、エンジンは `yaoxiang-rs`—

```
~/.yaoxiang/
├── bin/yx                  # フロントドア：小バイナリ、バージョン解決 + ディスパッチ（プロジェクト pin > デフォルト > 隣接エンジン）
├── settings.toml           # デフォルトバージョン、ミラーソース
└── versions/               # バージョンが第一級の概念；<ver>/ が当該バージョン配布パッケージの展開ルートディレクトリ
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # エンジン：コンパイル/実行/パッケージ管理/fmt/lsp サブコマンド
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- インストールルート `~/.yaoxiang/` は業界慣例に揃える（pyenv `~/.pyenv`、nvm `~/.nvm`、deno
  `~/.deno`、bun `~/.bun`、volta `~/.volta` 全て単一ルート；rustup の `~/.cargo`+`~/.rustup`
  二重ルートは cargo が rustup より先に存在した歴史的経緯で、踏襲しない）、かつ `~/.yaoxiang`
  は元々コードベースの既存ネームスペース（std グローバルフォールバックスロット）；環境変数
  `YAOXIANG_HOME` でのオーバーライドをサポート（先例 RUSTUP_HOME /
  DENO_INSTALL、CI とコンテナシナリオに対応）；Windows は `%USERPROFILE%\.yaoxiang`
- **バージョンディレクトリ = 配布パッケージの展開ルートディレクトリ**：`versions/<ver>/`
  はポータブル展開、deb インストールツリーと完全に同型で、バージョンのインストールは配布パッケージを展開するのと同じ —
  3 経路間で構造的分岐なし
- コマンド面（rustup 相当）：`yx toolchain install / default / update / list / uninstall`、加えて
  `yx self update`；その他の動詞はそのままエンジンに透過
- **バージョンロックは構造的保証**：fmt などのツールは構文と同期して進化する（旧 fmt は新構文を認識しない）、バージョン解決はフロントドアで一度完了し、グループ全体として切替—「新エンジンに旧 fmt」の組み合わせ空間は存在しない；将来 fmt/LSP を独立バイナリに分割する場合も同バージョン
  `bin/` 内に配置
- プロジェクトレベル pin：`yx-toolchain.toml`（先例 rust-toolchain.toml、コマンド名に追随；`yaoxiang.toml`
  には入れない—パッケージマニフェストがツールチェーンバージョンをライブラリの利用者に強制すべきではない）
- ブートストラップエントリポイント（`curl | sh` /
  `irm | iex`）で最新の stable パッケージを一度にインストール：`versions/` に展開、`bin/yx`
  をインストールルートに配置、`settings.toml`
  にデフォルトバージョンを書き込み（rustup「ブートストラップでマネージャをインストール」分解の等価収束—フロントドアとエンジンが同パッケージで、二段階不要）
- ミラーソース設定可能（settings.toml）、国内のユーザ事情を継続考慮（Z3 ダウンロードと同じネットワーク問題）
- **バージョン管理は自己完結不変量を壊さない**：各バージョンは完全な配布ツリーで、rpath と exe 相対 std 検索はツリー内で自己完結し、フロントドアは構造を変更せずディスパッチのみ

`.deb` と Inno は**システムレベル展開**経路（root / Program Files の単一バージョン、`apt upgrade`
/ コントロールパネルで更新）、サーバ、CI、完全初心者シナリオに対応；マネージャとの共存は PATH 順序で実現（先例：apt の rustc と rustup は並存可能）。全経路で同一の成果物ツリーを共有。

`.deb`
レイアウトは同じディレクトリツリーを再利用：`/usr/lib/yaoxiang/`（配布ツリー全体：`bin/{yx,yaoxiang-rs,libz3.so}` +
`lib/yaoxiang/std/`）+ `/usr/bin/yx` から `/usr/lib/yaoxiang/bin/yx`
へのシンボリックリンク—`$ORIGIN`
は**解決後の実際のパス**で計算されるため、リンク後も同じディレクトリの `libz3.so`
をヒットし、展開パッケージ構造と同型。ブランドフルネームはパッケージ名と製品名に残し（`apt install yaoxiang`、Inno 製品名 YaoXiang）、コマンド面は
`yx` で統一—Go と同じ：パッケージ名 `golang-go`、コマンド `go`。apt リポジトリは GitHub
Pages で静的ホスト（Packages/Release/InRelease メタデータを GPG 署名、release
CI が発行）；将来的に Debian/Ubuntu 公式収録を申請可能（サイクルが長く、版が滞后するため主経路ではない）。

### 標準ライブラリディレクトリ

`lib/yaoxiang/std/`
の内容は全てリポジトリの静的ファイルから取得し、パッケージングは**純粋なコピー**で、ランタイム生成エントリは設けない：

| レイヤ                                  | 出所                                                                      | 性質                                           |
| --------------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------- |
| native モジュール（io/math/...）        | リポジトリ `src/std/interfaces/*.yx` で事前生成したインターフェースビュー | インターフェース署名ビュー（実装はバイナリ内） |
| .yx レイヤモジュール（test/... 増加中） | リポジトリ `src/std/*.yx` をそのままコピー                                | 実ソース                                       |

事前生成ビューは `StdModule::exports()` から派生（`src/std/gen_interfaces.rs` の
`generate_all_interfaces()`）、**ランタイム生成サブコマンドは設けない**（2026-09-10 裁定：パッケージングが固まった後、サブコマンドは余分なインターフェース面）。同期は「生成物をリポジトリにコミット + テストゲート」方式を採用（RFC-013 コード表と同じ）：`test_committed_interface_files_match_generation`
で事前生成ファイルと現在生成結果をバイト単位で比較、ドリフトで赤；治癒は bless エントリ
`cargo test update_committed_interface_files -- --ignored` で。生成ロジックは crate 内部の
`StdModule` 実装に依存し、build.rs に下ろせないため、ゲートはビルド期ではなくテスト期。

**ランタイム検索チェーン**—`find_std_interface_file` に exe 相対検索を追加：

1. プロジェクト `.yaoxiang/vendor/std/<name>.yx`（プロジェクトオーバーライド、現状）
2. **exe のあるディレクトリ
   `../lib/yaoxiang/std/<name>.yx`（新規）**：ポータブル展開、マネージドインストール（`versions/<ver>/`）、deb 展開を統一ヒット
3. `~/.yaoxiang/std/<name>.yx`（グローバルフォールバック、手動オーバーライド用に保持）

現状このチェーンはほぼ失联状態（LSP 呼び出しがプロジェクトディレクトリを渡さない、`package init`
が書き込む `.yaoxiang/std` がチェーン上にない）、今回はついでに修正し、`package init` の出力を
`.yaoxiang/vendor/std` に統一（パッケージマネージャの vendor ディレクトリと一致）。

**コンパイル権限は変更なし**：`.yx` レイヤは引き続き `include_str!`
で埋め込み（RFC-036 の「std バージョンとバイナリの厳密バインド」不変量を維持）。配布ディレクトリの位置付けは**可読ビュー +
LSP 解析ソース**であり、コンパイル入力ではない—ユーザが配布ディレクトリ内の `.yx`
を手作業で変更してもコンパイラは採用しない（Python 流の「`Lib/`
を変更すれば即座に反映」セマンティクスを開放するかは、開放問題を参照）。

### Wasm ビルド

**独立を維持、cargo-dist には移行しない。**

cargo-dist が管理するのは「コンパイラをユーザに配布する」こと、wasm は「オンライン playground をドキュメントサイトに埋め込む」こと—全く異なる 2 セットのデリバラブル。

| 観点             | 方法                                         |
| ---------------- | -------------------------------------------- |
| ビルドツール     | `wasm-pack build` を維持                     |
| CI workflow      | `_build-wasm.yml` 独立ジョブを維持           |
| トリガタイミング | release と同じ tag push 時、並列の独立ジョブ |
| 公開先           | `docs/public/wasm/` → GitHub Pages           |

### npm 公開

| パッケージ             | 内容                                     | 状態                                                                                                                                                                                                       |
| ---------------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@yaoxiang/cli`        | 配布パッケージをダウンロードする wrapper | 延期：cargo-dist の npm wrapper も同じくフラットアーティファクト前提のため、インストーラと共に廃止；npm 経路が必要なら自作 wrapper（再構成パッケージをダウンロードして展開、展開インストールと同ロジック） |
| `@yaoxiang/playground` | wasm ライブラリ（JS + .wasm）            | オプション、現状は docs のみ                                                                                                                                                                               |

両者は衝突せず、名前も衝突しない。

### 既存リリースフローとの統合

現状の `release.yml`：push main → check-version（`v{version}` tag が存在しない場合のみ通過）→ build
/ build-wasm / security / test の 4 経路 → release ジョブ（tag 打ち & プッシュ +
`generate-commit-list.mjs` で @mentions を含む body 生成 + アーティファクトアップロード）。

cargo-dist 生成のパイプラインは tag 駆動で announce/publish を内蔵し、fmt/clippy/test/audit ゲートを含まず、release
notes 形式も merge commit changelog を担えない。**全体一括置換は既存リリース儀式を破壊する**（PR →
CI 全緑 → bump → merge commit が changelog）。

統合原則：**トリガとゲートは現状維持、ビルドは `cargo dist build` に委ねる、公開は現状維持。**

1. check-version / security / test の 3 ジョブは現状維持（push main トリガ、tag 打ち前のゲート）
2. 全通過後、release ジョブが `v{version}` tag を作成・プッシュ（現状維持）
3. tag push で新規 `dist-release.yml`
   をトリガ：plan ジョブが dist で runner/システム依存マトリクスを計算 →
   `cargo dist build`（5 ターゲット）→ `package-dist.sh` でターゲットごとに再構成 → Inno
   Setup ジョブ（Windows 再構成パッケージを食べ、ウィザードを構築、`/DMyAppVersion=`
   でバージョン注入、二次コンパイル不要）→ `_build-wasm.yml`（並列ジョブ）
4. publish ジョブ：`generate-commit-list.mjs`
   で body 生成（既存スクリプト再利用）→ 再構成パッケージ + `.sha256` + `.deb` + wasm + Setup
   exe の追加アップロード；独立した `publish-apt` ジョブで GitHub Pages
   apt リポジトリメタデータ公開（`secrets.APT_GPG_KEY` 未設定時は自動スキップ、他経路に影響なし）

### Nightly 公開

cargo-dist にはネイティブ nightly サポートがない（[axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143)、open
feature request のまま）。

既存の cron + tag オーバーライド方式を維持し、ビルド部分を `_build-platforms.yml` から
`cargo dist build`
に変更—これは本質的に 1 本の cargo コマンドなので、nightly.yml で直接呼び出せる。workflow 再利用は行わない（想定の
`uses: ./release.yml` は不可：再利用側に `workflow_call`
トリガが必要、かつ cargo-dist ワークフローは tag 駆動でビルドと公開が結合）：

```yaml
# nightly.yml（移行後）
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh（正式版と同じセット）
  publish: # 現状維持：nightly tag を打つ/移す → GitHub Pre-release を上書き
```

### cargo-dist 設定（dist-workspace.toml として既に実装）

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# dist バージョンを固定（Cargo.toml SemVer 構文）
cargo-dist-version = "0.32.0"
ci = "github"
# インストーラは全て独自、cargo-dist はビルド + 圧縮パッケージ + チェックサムのみ
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# 生成されたワークフローは意図的に改造されている（package-dist.sh 再構成 + 自作公開セグメント、RFC-037）、
# generate テンプレートからのドリフトでビルド拒否しない
allow-dirty = ["ci"]
```

以上はリポジトリ vendor の実際の設定（`cargo dist init`
生成後に必要に応じて修正）；`cargo-dist-version`
は 0.32.0 に固定、生成されたワークフローは vendor してリポジトリにコミットし review を受け、ランタイムに pull しない。ビルドプロファイルは init がルートの Cargo.toml の
`[profile.dist]` に注入（inherits release、lto=thin）、両バイナリは `target/<triple>/dist/`
に配置され再構成スクリプトがそこから取得。

### package-dist.sh（既に実装）

リポジトリの `scripts/release/package-dist.sh` を基準とし、要点：

- 両バイナリは `cargo dist build` のビルド出力ディレクトリ `target/<triple>/dist/`
  から直接取得（profile=dist）—cargo-dist が自生するフラット単一バイナリアーカイブはデリバラブルではなく、同名の再構成パッケージが
  `target/distrib/` で直接上書き
- Z3 共有ライブラリもビルド出力ディレクトリから取得—build.rs リンク時に対応するプラットフォームの共有ライブラリを選択し配置（`copy_shared_lib`）、**単一情報源**：パッケージングスクリプトは Z3 のバージョンとプラットフォームディレクトリ名を知る必要がない；Z3 ライセンス文書はライブラリと共に配置されパッケージングに含まれる（MIT 配布義務）、macOS 側ではパッケージング時に dylib の install_name を
  `@rpath` に統一し、バイナリに ad-hoc 再署名
- std ディレクトリは純粋コピー：`src/std/interfaces/*.yx`（事前生成インターフェースビュー）+
  `src/std/*.yx`（.yx レイヤ実ソース）
- README/LICENSE を添付；再パッケージング（Windows zip / その他 tar.gz；Git
  Bash に zip がない場合は zip → System32 bsdtar → PowerShell の三段フォールバック）し `.sha256`
  を再計算
- Linux かつ `dpkg-deb` 利用可能時、合わせて `build-deb.sh` を呼び出し `.deb`
  を生成（`/usr/lib/yaoxiang` 展開ツリー + `/usr/bin/yx` シンボリックリンク）

### 廃止する手書き CI

移行完了後に調整するファイル：

| ファイル                                 | 行数        | 処分                                                             |
| ---------------------------------------- | ----------- | ---------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254         | 削除（cargo-dist ビルドマトリクスで置換）                        |
| `.github/workflows/release.yml`          | 189         | ゲート + tag 打ちに縮小（ビルド/公開は dist-release.yml に移動） |
| `.github/workflows/nightly.yml`          | 173         | ビルドセグメントを `cargo dist build` に変更、公開ロジックは維持 |
| `scripts/build/setup.iss`                | ~250        | **維持して正式採用**（Windows ウィザード）                       |
| **合計削減**                             | **~600 行** |                                                                  |

維持：

- `ci.yml`（日常 fmt + clippy + test + MSRV、リリースフローには属さない）
- `_build-wasm.yml`（独立ビルドフロー、dist-release.yml に並列ジョブとして組み込み）
- `_build-z3-wasm.yml`（wasm 専用 Z3）
- `docs-deploy.yml`（ドキュメントデプロイ）

### 受け入れ基準

「箱から出してすぐ使える」はテスト可能で、移行完了の判定は「新旧アーティファクトが一致」ではなく、以下すべてが通ること：

- クリーンなマシン（Rust なし / Z3 なし / `~/.yaoxiang`
  なし）で任意のプラットフォームの圧縮パッケージを展開し、直接 `bin/yaoxiang-rs --version`
  を実行して成功—`LD_LIBRARY_PATH` を設定しない（rpath 有効）
- 展開ディレクトリ内の `lib/yaoxiang/std/*.yx`
  が全て可読：native モジュールは署名インターフェースビュー、`.yx` レイヤは実ソース
- 展開ディレクトリ下のサンプルプロジェクトで LSP を起動し、std メンバー補完 / ジャンプ定義が利用可能（exe 相対検索有効）
- 公式ガイド通り `/usr/local`（または `~/.yaoxiang`）に展開して PATH 追加後、任意のディレクトリで
  `yx --version` が成功
- `apt install yaoxiang`（自前リポジトリ）後、実行可能で `apt upgrade`
  でバージョン追従；`/usr/bin/yx` シンボリックリンク下のエンジンの `$ORIGIN`（実際のパスで解決）が
  `bin/libz3.so` をヒット
- `curl ... | sh` と `irm ... | iex` をクリーン環境で実行後、`yx` が実行可能で PATH 就位
- `yx toolchain install <ver>` / `default` / `update`
  が有効：複数バージョン共存、`yx-toolchain.toml`
  プロジェクト pin がデフォルトバージョンより優先、フロントドアが正しいバージョンにディスパッチ（バージョンツリー内の rpath と std 検索が自己完結、「新エンジンに旧 fmt」の組み合わせなし）
- ポータブル展開後、`yx` が隣接の `yaoxiang-rs` にフォールバックし、マネージドインストールと動作一致
- Inno Setup インストール後、ディレクトリ構造が完全で PATH 有効、アンインストール可能
- Release アセット完備：5 プラットフォーム再構成パッケージ + `.sha256` が実内容と一致
- Release body は `generate-commit-list.mjs` の出力（merge commit changelog 完全）
- nightly アーティファクトは Pre-release で、最新の正式 tag に影響しない

## トレードオフ

### メリット

- **箱から出してすぐ使える**
  — ポータブル展開即使用（rpath + 同ディレクトリ共有ライブラリ）、インストーラが完全なディレクトリを敷設
- **標準ライブラリが読める** — ユーザは Python `Lib/` のように直接 std を読める（必須要件達成）
- **保守コスト削減** — ~600 行の手書きビルド YAML を cargo-dist + ~80 行の独自スクリプトに交換
- **クロスプラットフォーム一貫性**
  — 全プラットフォーム動的リンク + 同ディレクトリ共有ライブラリ、特例なし
- **インストールの二層化** — 標準経路はコード追加なし（Go/Zig 方式）；簡易経路は Rust を参照、apt /
  curl / iex / exe 4 エントリで同じアーティファクト構造を共有
- **バージョン管理内蔵** — フロントドア `yx` が rustup/Go
  GOTOOLCHAIN 相当、Python/Node が事後的に pyenv/nvm/pdm で補講する生態分裂を回避；ツールのバージョンロックは規約ではなく構造保証

### デメリットとリスク

- **簡易経路の保守面** — install.sh / install.ps1（ワンクリックスクリプト、ほぼ進化しない）+ `.deb`
  と apt リポジトリメタデータ公開（release CI 自動化）+ `yx` フロントドア crate
- **エンジンリネームの影響範囲** — `yaoxiang` → `yaoxiang-rs`
  の CI アーティファクト名、Inno、テスト、ドキュメントの一括移行が必要（フェーズ 5 内で完了）
- **学習コスト** — チームは cargo-dist の設定を学習する必要
- **cargo-dist 上流リスク** —
  2025 年中頃に Axo の停止に伴い停滞、同年 9 月に原作者が復活し継続リリース（0.29 →
  0.32+）；`dist-version` 固定 + 生成物 vendor してリポジトリにコミット review で緩和
- **cargo-dist にネイティブ nightly がない** — nightly 公開部分は引き続き手書き

### RFC-014b との関係

|              | RFC-014b                                  | RFC-037                                      |
| ------------ | ----------------------------------------- | -------------------------------------------- |
| **範囲**     | サードパーティパッケージのビルドと配布    | コンパイラ自体のパッケージングと配布         |
| **ツール**   | `yaoxiang build` / `yaoxiang publish`     | `cargo-dist` + 独自スクリプト                |
| **成果物**   | サードパーティパッケージの FFI ライブラリ | コンパイラ + 標準ライブラリ + ツールチェーン |
| **相互排他** | いいえ、補完                              | いいえ、補完                                 |

## 代替案

| 方案                                         | 選択しない理由                                                                                                                                                                                             |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **手書き CI を継続**                         | 既に ~870 行手書き、重複作業、DLL 漏れが容易                                                                                                                                                               |
| **独自パッケージングツール**                 | 車輪の再発明をしない、cargo-dist は既に成熟                                                                                                                                                                |
| **tar.gz のみでインストーラなし**            | 唯一の公式経路は展開 + PATH；Inno は Windows ウィザード慣行のため（国内ユーザは維持を裁定）                                                                                                                |
| **Docker 配布**                              | コンパイラと言語ツールチェーンはネイティブバイナリが必要、コンテナシナリオではない                                                                                                                         |
| **自作 Homebrew tap**                        | tap は一律コミュニティ保守（homebrew-core）、自作は時期尚早；macOS の簡易経路は curl スクリプト                                                                                                            |
| **独立した `yaoxiangup` マネージャバイナリ** | rustup そのままの先例として可行；ただし 2 つ目のユーザ動詞を生み出し、「パッケージマネージャ/fmt も独立させるべきか」という対称性の問いを招く—フロントドア/エンジン分離で一括解消（2026-09-09 議論で否決） |
| **自己更新のみ、複数バージョンなし**         | 単一バージョンの自己更新は、複数プロジェクトが異なるバージョンを pin する要求を満たせない；Python/Node は公式バージョン管理を欠き、エコシステムが pyenv/nvm/pdm を強制的に生んだ—内蔵を裁定（2026-09-09）  |
| **Z3 全静的リンク**                          | 裁定で否決—外部システムはディレクトリ型共有ライブラリ配布が自然な形態；exe に埋め込むことと外に出すことは「どちらもパッケージ同梱」において等価で、後者の方が交換可能性を保つ                              |
| **Inno Setup 廃止**                          | 裁定で否決—Windows ウィザードとして維持（追加経路）                                                                                                                                                        |
| **cargo-dist ネイティブインストーラ**        | フラットバイナリ前提が bin/+lib/ 構造と衝突、インストール直後にライブラリ欠落                                                                                                                              |
| **自前 WiX/MSI 保守**                        | cargo-dist ジェネレータを失った後、コストが Inno のカバー価値を上回る                                                                                                                                      |

## 実装戦略

### フェーズ 1：言語側の変更（P0）

1. `build.rs`：全プラットフォーム統一動的リンク + rpath link-arg；`copy_dll()` を
   `copy_shared_lib()` に拡張（so/dylib/dll）
2. リポジトリで native インターフェースビューを事前生成（`src/std/interfaces/`）、テストゲートで
   `StdModule::exports()` との同期を強制（gen-std サブコマンドは取消裁定）
3. `find_std_interface_file` に exe 相対検索分岐を追加；`package init` の出力パスを
   `.yaoxiang/vendor/std` に統一

### フェーズ 2：cargo-dist 導入（P0）

1. `cargo dist init` を実行して初期設定を生成（`installers = []`、dist-version 固定）
2. `package-dist.sh` を記述（再構成 + .yx ソースコピー + チェックサム再計算）
3. 新規 `dist-release.yml`（tag 駆動：dist build → 再構成 →
   wasm 並列 → 独自 publish）；`release.yml` を ゲート + tag 打ちに縮小
4. 新旧パイプラインを並行実行し、受け入れ基準で逐次検証

### フェーズ 3：旧 CI 停止（P1）

1. 問題なければ `_build-platforms.yml` を削除
2. `nightly.yml` のビルドセグメントを `cargo dist build` に変更
3. `setup.iss`
   を新アーティファクト構造に統合（Inno 正式採用；バージョン番号は Cargo.toml から注入、sed 置換を廃止）

### フェーズ 4：簡易経路（P2）

1. `install.sh` / `install.ps1`（プラットフォーム検出 → 最新の再構成パッケージをダウンロード →
   `versions/` に展開 → `bin/yx` をインストールルートに配置 → `settings.toml`
   にデフォルトバージョンを書き込み →
   PATH ヒント/書き込み；フェーズ 5 と同ラウンドで実装、最終形を一気に作る）
2. `.deb` パッケージング（`package-dist.sh` と同じディレクトリツリーを再利用 + `/usr/bin`
   シンボリックリンク）+ GitHub Pages 静的 apt リポジトリ（メタデータを GPG 署名、release
   CI で公開）

### フェーズ 5：フロントドア yx とエンジンリネーム（P2）

1. 現モノリスを `yaoxiang-rs`
   にリネーム（CI アーティファクト名、Inno、テスト、ドキュメントを一括スキャン、一回限りの移行）
2. 新規フロントドア小 crate
   `yx`（workspace メンバー）：バージョン解決、release アーティファクトのダウンロード、tar/zip 展開、settings.toml、ディスパッチ（toolchain/self 動詞のみ保持、他は透過、手書きディスパッチで clap を使わず引数逐語転送を保証）；バージョンインデックスは GitHub
   Releases latest API（ミラーソースは ghproxy 風プレフィックス連結）から取得；`yx`
   コマンド名は衝突チェック済み（主要ディストリビューション/Homebrew に同名の常用コマンドなし、唯一の小ツールが yx をエイリアスとして使用）
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml`
   プロジェクト pin + ミラーソース + `yx self update`
4. ブートストラップスクリプト：`curl | sh` / `irm | iex`
   → 再構成パッケージをダウンロードしフロントドアとデフォルト stable を一度にインストール（フェーズ 4 と同ラウンドで最終形として実装）

### フェーズ 6：オプション追従（いずれもブロックしない）

1. winget 申請（Inno exe を指す、コミュニティ保守、homebrew-core モデルと対称）
2. `.rpm`（dnf ユーザ、`.deb` と同型）
3. Homebrew：homebrew-core 参入基準に達した後、コミュニティが提出
4. npm `@yaoxiang/cli` 自作 wrapper（名称は現在未登録）

## 開放問題

### 未決

- **.yx レイヤに「配布ディレクトリを手で変更すればコンパイルで採用される」Python 式セマンティクスを提供するか？**
  デフォルトはいいえ—コンパイル権限は RFC-036 埋め込みを維持（std バージョンとバイナリの厳密バインド）、配布ディレクトリは可読ビュー +
  LSP 解析ソースとして位置付け。将来開放する場合、バージョンバインド不変量を再検討する必要あり。

### 解決済み

設計議論で解決された以下の問題：

- ~~Windows での Z3 静的リンクの可否？~~ →
  **静的リンクは行わず、全プラットフォーム動的**（2026-09-09 再確認維持）
- ~~gen-std-interfaces サブコマンド名？~~ →
  **サブコマンドを設けない**（2026-09-10 裁定：通常のパッケージングが固まった後、サブコマンド面は余分；native インターフェースビューをリポジトリ事前生成
  `src/std/interfaces/` + テストゲート同期に変更、パッケージングは純粋コピー）
- ~~Inno Setup を維持するか？~~ → **Windows ウィザードとして維持（追加経路）**
- ~~配布パッケージ構造が標準ライブラリソースを物理同梱するか？~~ → **必須**（ユーザ可読性が Python
  `Lib/` に揃う、2026-09-09 裁定）
- ~~cargo-dist ネイティブインストーラ（shell/powershell/homebrew/msi/npm）？~~ →
  **全て廃止**、インストーラは独自（フラット前提が bin/+lib/ と衝突）
- ~~インストーラ戦略？~~ →
  **二層**（2026-09-09 終裁）：標準経路 = 配布パッケージ即プロダクト（Go/Zig 方式、展開 +
  PATH）；簡易経路は Rust ワンライナーインストールを参照—Linux `apt`（自前 deb リポジトリ）/
  `curl | sh`、Windows `irm | iex` / Inno
  exe。行わない：MSI、cargo-dist ネイティブインストーラ、自作 brew tap
- ~~バージョン管理を含めるか？~~ →
  **必須、インストーラ体系に属する**（2026-09-09 裁定、同日早朝の「遠期独立 RFC」境界を推翻）：動機は Python/Node が公式バージョン管理を欠き pyenv/nvm/pdm 補講の生態分裂を招いたこと
- ~~バージョン管理形態：独立バイナリかサブコマンドか？~~ →
  **フロントドア/エンジン分離**（2026-09-09 終裁、3 ラウンド収束 A→C→命名反転）：常用コマンド `yx`
  = フロントドア（小バイナリ、toolchain/self 動詞を内蔵）、エンジン `yaoxiang-rs` = 現 `yaoxiang`
  モノリスのリネーム；ディレクトリ構造
  `versions/<ver>/`—バージョンが第一級の概念、バージョンディレクトリが配布パッケージの展開ルートディレクトリで、ツールはバージョン全体でロック（旧 fmt は新構文を認識しない；かつて想定した内層
  `toolchains/` は冗長として取消）。先例：Go `go` フロントドア +
  GOTOOLCHAIN、rustup プロキシディスパッチ
- ~~cargo-dist extra-artifacts 条件実行？~~ → **`package-dist.sh` スクリプトで処理、shell
  case 分岐を使用**
- ~~標準ライブラリインターフェースバージョン互換性？~~ →
  **コンパイラバージョンと共にリリース、同じ圧縮パッケージ内**

## 参考文献

- [cargo-dist 公式ドキュメント](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 ビルド設定 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

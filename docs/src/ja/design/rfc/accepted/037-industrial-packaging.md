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
> を補完する。RFC-014b は
> **YaoXiang パッケージマネージャ**がサードパーティパッケージをビルド・配布する方法を定義する；本 RFC は
> **YaoXiang コンパイラ/ツールチェーン自体**のパッケージングと配布方法を定義する。

## 概要

`cargo-dist`（Rust エコシステムのバイナリ配布ツール）にクロスプラットフォームビルドのオーケストレーションを担わせ、自社スクリプトでディストリビューションの構造を担う。中心となる約束は 2 つ：ディストリビューションは**物理的に標準ライブラリのソースコードディレクトリを同梱**し（ユーザは Python の
`Lib/`
を読むように直接読める）、全プラットフォームで動的リンクする Z3 共有ライブラリをパッケージに同梱する。コマンドモデルは**フロントドア/エンジン分離**：常用コマンド
`yx`（小さなフロントドア、組込みバージョン管理 — rustup/Go GOTOOLCHAIN 相当）、エンジン
`yaoxiang-rs`（現 `yaoxiang`
モノリスの改名）で、Python/Node が事後的に nvm/pdm で補講する生態系の分裂を避ける。インストール方式は二重：標準チャネルは Go/Zig に合わせて**ディストリビューションがそのままプロダクト**、展開 +
PATH；お手軽チャネルはワンライナーインストール（Linux `apt` / `curl | sh`、Windows `irm | iex` /
Inno exe ウィザード）。`libz3.dll`
の欠落、標準ライブラリのユーザ非可視、CI スクリプトの重複保守といった問題を解決する。

## 動機

### なぜこの機能が必要か

YaoXiang をダウンロードしたユーザは**箱から出してすぐ使える**べきで、追加手順は一切不要；標準ライブラリは**ユーザが直接読める**べきで、バイナリ内のブラックボックスに隠されているべきではない。

### 現状の問題

#### 問題 1：Windows ユーザがダウンロードしても動かない

現在の Release には `yaoxiang.exe` しかアップロードされておらず、`libz3.dll`
がパッケージに含まれていない。Windows でダブルクリックするとエラーになる：

```
The code execution cannot proceed because libz3.dll was not found.
```

これは **ブロッキングバグ** — ユーザは最初のステップにも進めない。

#### 問題 2：Release 成果物が単一 exe のみで、標準ライブラリがユーザに不可視

現状は三重の断絶：

- Release 成果物は生バイナリのみで、標準ライブラリはディストリビューションに同梱されない
- LSP のインタフェースファイル検索チェーンはほぼ壊れている：`find_std_interface_file`
  を呼ぶ際にプロジェクトディレクトリを渡さず（グローバルな `~/.yaoxiang/std/`
  のみ調べ、それを埋めるフローがない）；`package init` が書き込むのは `.yaoxiang/std`
  で、検索チェーン上にない
- 標準ライブラリのソースコード（`.yx`
  層）とインタフェースビュー（native 層）はユーザにとって完全にブラックボックス

産業化されたやり方：ユーザは Python の `Lib/`
を直接開くように標準ライブラリディレクトリを読んでソースを直接読める —
**ディストリビューションが物理的に std ディレクトリを同梱することは本方案の必須要件**（既裁定）。

#### 問題 3：CI の手書きスクリプトの重複保守

現在、複数のビルドパイプラインを保守している：

| ファイル                  | 役割                         | 行数        |
| ------------------------- | ---------------------------- | ----------- |
| `_build-platforms.yml`    | クロスプラットフォームビルド | ~255 行     |
| `release.yml`             | バージョンリリース           | ~189 行     |
| `nightly.yml`             | デイリービルド               | ~173 行     |
| `scripts/build/setup.iss` | Inno Setup インストーラ      | ~250 行     |
| **合計**                  |                              | **~870 行** |

大部分は重複しており（Rust インストール → キャッシュ → ビルド → リネーム → アップロード）、各プラットフォームごとに書き直す必要がある。

#### 問題 4：Inno Setup のバージョン番号のハードコード

`setup.iss` 内の `MyAppVersion` が `0.7.0` とハードコードされており、ビルド時に `sed`
で置換している。迟早必ず失敗する。

#### 問題 5：RFC-014b との境界が曖昧

RFC-014b は「YaoXiang パッケージのビルドと配布機構」（すなわち `yaoxiang.toml` の `[build]` と
`[binaries]`
設定）を定義しているが、**「YaoXiang コンパイラ自体をどうリリースするか」をカバーしていない**。本 RFC はこの空白を埋める。

## 提案

### 中核設計

cargo-dist が担うのは**ビルドオーケストレーション層**のみ；パッケージ構造とインストーラはすべて自社。責務分割：

```
cargo-dist の責務（ビルドオーケストレーション層）:
  ├── クロスプラットフォームコンパイル（5 ターゲット）
  └── 圧縮パッケージとチェックサムの生成
  （ネイティブインストーラと npm wrapper は廃止 — フラットバイナリという仮定が bin/+lib/ 構造と衝突する）

build.rs の継続する責務:
  └── Z3 ダウンロード/リンク（全プラットフォーム動的 + rpath）

YaoXiang 自社スクリプト:
  ├── package-dist.sh — パッケージ構造の再構成（bin/ + lib/）、共有ライブラリの同梱、
  │   std ディレクトリの充填（リポジトリの事前生成インタフェースビュー + .yx 層ソース）、チェックサム再計算
  └── Inno Setup — Windows インストールウィザード（既存資産；完全なディレクトリ構造を敷設）

コマンドモデル（フロントドア/エンジン分離）:
  ├── yx — フロントドア（新規小型 crate）：バージョン解決 + ディスパッチ；toolchain/self 動詞のみ保持、他は透過
  └── yaoxiang-rs — エンジン（現 yaoxiang モノリスの改名）：コンパイル/実行/パッケージ管理/fmt/lsp サブコマンド

インストール方式（二重）:
  ├── 標準チャネル（Go/Zig 方式）: ディストリビューションがそのままプロダクト、展開 + PATH
  └── お手軽チャネル（Rust 方式）: ワンライナーインストール + バージョン管理（yx フロントドアに内蔵）
      ├── Linux: apt（自社 deb リポジトリ、システムレベル並列配置）/ curl … | sh（パッケージ一括インストール）
      ├── Windows: irm … | iex（パッケージ一括インストール）/ Inno Setup ウィザード（既存資産、システムレベル並列配置）
      └── macOS: curl … | sh（brew は homebrew-core コミュニティに委ねる）
```

### リリースディレクトリ構造（既裁定：物理的に標準ライブラリソースを同梱）

ユーザは Python の `Lib/`
を直接読むように標準ライブラリを読めるべきで、ディストリビューションが同棲の std ディレクトリを持つことは必須要件であり、パッケージングの詳細ではない。Z3 も同様：外部システムとして、ディレクトリ型の共有ライブラリ配布は自然な形態 —
`.so`
を exe に詰め込むのも外に置いて動的リンクするのも「どちらもディストリビューションに同梱する必要がある」点では同等で、後者の方が交換可能性を保つ。

各プラットフォームのディストリビューションは、`package-dist.sh`
が cargo-dist のビルド後に再構成する：

```
yaoxiang-{version}-{target}.tar.gz / .zip     （ポータブル：展開後 bin/ 内から直接実行可能）
├── bin/
│   ├── yx                            # フロントドア（または yx.exe）
│   ├── yaoxiang-rs                   # エンジン（または yaoxiang-rs.exe）
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # ユーザが直接読める（Python Lib/ 方式）
│           ├── io.yx                 # native モジュール：リポジトリで事前生成されたインタフェースビュー
│           ├── math.yx
│           ├── test.yx               # .yx 層：リポジトリ内の実ソースをそのままコピー
│           └── ...
├── README.md
└── LICENSE
```

インストール = 任意のディレクトリに展開 + `bin/` を PATH に追加（Go の `/usr/local/go/bin`
方式；`~/.yaoxiang/` への展開が一般的選択）。Windows では Inno
Setup ウィザードが同じことを行う（デフォルト Program Files）。ポータブル展開時、`yx` フロントドアは
`~/.yaoxiang` 状態を持たず、隣接の `yaoxiang-rs`
にフォールバック — マネージドインストールと同じ動作；エンジンの rpath と exe 相対 std 検索はフロントドアの存在によって変わらない。

### プラットフォームサポート

| プラットフォーム | target triple               | 説明                    |
| ---------------- | --------------------------- | ----------------------- |
| Linux x86_64     | `x86_64-unknown-linux-gnu`  | メインプラットフォーム  |
| Linux ARM64      | `aarch64-unknown-linux-gnu` | CI 上でクロスコンパイル |
| macOS x86_64     | `x86_64-apple-darwin`       | Intel Mac               |
| macOS ARM64      | `aarch64-apple-darwin`      | Apple Silicon           |
| Windows x86_64   | `x86_64-pc-windows-msvc`    | メインプラットフォーム  |

合計 5 ターゲット。Windows
ARM64 は当面サポートしない（Z3 公式のプレコンパイル ARM64 パッケージがない）。

### Z3 配布戦略

**全プラットフォーム動的リンク**（既確認維持）：

| プラットフォーム | 変更                   | 成果物        |
| ---------------- | ---------------------- | ------------- |
| Linux            | **静的→動的に変更**    | `libz3.so`    |
| macOS            | **静的→動的に変更**    | `libz3.dylib` |
| Windows          | 変更なし               | `libz3.dll`   |
| wasm32           | 変更なし（静的リンク） | 内蔵 `.a`     |

理由：

- **一貫性** — 3 プラットフォームの挙動が統一され、例外がなくなる
- **外部ライブラリは共有ライブラリで配布すべき**。Python（`python3.dll`+`DLLs/lib*.dll`）、Node（`node`+`lib/`）もそうしている
- **ユーザの Z3 アップグレードはコンパイラのバージョンに依存しない** — `.so`/`.dylib`/`.dll`
  を差し替えるだけ
- **バイナリサイズが小さい** — Z3 は小さくなく、静的リンクだと exe が数 MB 膨れる

動的リンクには**必要な付帯事項**がある：Linux/macOS の動的リンカはデフォルトでバイナリの所在ディレクトリを検索しないため、rpath を注入しなければ「展開してすぐ使う」が成立しない（Windows はデフォルトで exe ディレクトリを検索する、処理不要）。対応する
`build.rs` の変更：

```rust
// 統一動的リンク + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 配布パッケージのレイアウトが統一されていない、lib/bin 両ディレクトリを探索
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037：全プラットフォーム動的リンク。共有ライブラリは bin/ に同梱して配布、ユーザは Z3 を一括交換・アップグレード可能
    if target_os == "windows" {
        // MSVC import lib の名前は libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // 動的リンカはデフォルトでバイナリ所在ディレクトリを検索しない、rpath を注入しなければ「展開してすぐ使う」が成立しない
        // （ディストリビューション内の exe と libz3 は同じ bin/ にある；Windows はデフォルトで exe ディレクトリを検索する、処理不要）
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

**「全プラットフォーム静的リンク」は目標としない。**これは特殊ケースの除去ではなく、合理的なケースを誤った方法で除去するもの。共有ライブラリは外部ライブラリの正常な配布方式である。

### インストーラサポート

主要言語ツールチェーンの配布方式との対照（2026-09 調査）：

| 言語     | 公式ディストリビューション   | 公式インストール方法                                     | インストーラ保守主体               |
| -------- | ---------------------------- | -------------------------------------------------------- | ---------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | 公式ドキュメントがそのまま「ダウンロード → 展開 → PATH」 | なし（brew/apt はコミュニティ）    |
| Zig      | `zig/{bin,lib/std}` tarball  | 同上、公式インストールスクリプトなし                     | なし（homebrew-core コミュニティ） |
| Node     | `{bin,lib,include}` tarball  | tar + 公式 pkg/msi                                       | チームが自作                       |
| Rust     | 複数コンポーネント tarball   | rustup                                                   | チームが自作                       |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                              | チーム + brew コミュニティ         |
| Deno/Bun | 単一バイナリ zip             | 公式 curl スクリプト                                     | チームが自作（スクリプト極小）     |
| Gleam    | cargo-dist 単一バイナリ      | cargo-dist 生成スクリプト                                | cargo-dist                         |

3 つの法則：

- **複数ファイルのツールチェーンで、サードパーティ生成器を使ってインストーラを作る例はない** —
  cargo-dist のインストーラは単一バイナリシナリオにのみ対応する（Gleam で使えるのはまさに外部依存なしの単一 Rust バイナリだから）
- 最もシンプルなモデルは
  **Go/Zig の「ディストリビューションがそのままプロダクト」**：公式インストールガイドは展開 +
  PATH のみ、インストーラコードゼロ；ディストリビューションに可読 std ソースが同梱されるのは常態（Go の
  `src/`、Zig の `lib/std/`、Crystal の `src/`）
- curl ワンライナーを提供するのは（Deno/Bun/rustup）すべて**自作スクリプト**でほぼ進化しない；brew
  formula は一律コミュニティが homebrew-core で保守し、言語チームは自作 tap を作らない（Crystal チームが明確に「formula はコミュニティのもの」と述べている）

YaoXiang は二重モデルを採用：

| チャネル                                                | 層     | 状態 | 説明                                                                                                                                        |
| ------------------------------------------------------- | ------ | ---- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                            | 標準   | ✅   | 展開してすぐ使える（rpath + 同ディレクトリ共有ライブラリ）、extract + PATH が公式ガイド                                                     |
| `yx`（フロントドア、内蔵バージョン管理）                | お手軽 | ✅   | rustup/Go GOTOOLCHAIN 相当：複数バージョンインストール/切替/更新 + プロジェクト pin                                                         |
| `curl ... \| sh`（install.sh）                          | お手軽 | ✅   | Linux / macOS：再構成パッケージをダウンロードして `versions/` に展開、`bin/yx` をインストールルートに配置し、デフォルトバージョンを書き込む |
| `irm ... \| iex`（install.ps1）                         | お手軽 | ✅   | Windows：同ロジック                                                                                                                         |
| `apt install yaoxiang`                                  | お手軽 | ✅   | `.deb`（amd64/arm64）+ GitHub Pages 静的 apt リポジトリ；システムレベル並列配置、`apt upgrade` でバージョン追従                             |
| Inno Setup exe                                          | お手軽 | ✅   | Windows ウィザード（既存資産）、システムレベル並列配置、完全な bin/+lib/ 構造を敷設                                                         |
| winget / `.rpm` / homebrew-core / npm                   | —      | ⏸    | オプションフォローアップ：winget と brew-core はどちらもコミュニティ保守、rpm と deb は同型                                                 |
| MSI / cargo-dist ネイティブインストーラ / 自作 brew tap | —      | ❌   | 「代替案」を参照                                                                                                                            |

**お手軽チャネルは Rust を参照、バージョン管理をフロントドアに内蔵。**Rust の分解は「ブートストラップスクリプト（sh.rustup.rs）→
rustup
→ ツールチェーンパッケージ」、rustup は最初から複数バージョン、切替、更新を管理する；Python/Node の公式インストーラはこの層を作らず、生態系が事後的に pyenv/nvm/pdm をそれぞれ分立して生み出した。YaoXiang は**フロントドア/エンジン分離**を採用する（Go の
`go` フロントドア + GOTOOLCHAIN、rustup のプロキシディスパッチ、両者の同型形態）：常用コマンドは
`yx`、エンジンは `yaoxiang-rs`——

```
~/.yaoxiang/
├── bin/yx                  # フロントドア：小型バイナリ、バージョン解決 + ディスパッチ（プロジェクト pin > デフォルト > 隣接エンジン）
├── settings.toml           # デフォルトバージョン、ミラーソース
└── versions/               # バージョンが第一級の概念；<ver>/ がそのバージョンディストリビューションの展開ルート
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # エンジン：コンパイル/実行/パッケージ管理/fmt/lsp サブコマンド
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- インストールルート `~/.yaoxiang/` は業界慣例に合わせる（pyenv `~/.pyenv`、nvm `~/.nvm`、deno
  `~/.deno`、bun `~/.bun`、volta `~/.volta` いずれも単一ルート；rustup の `~/.cargo`+`~/.rustup`
  二重ルートは cargo が rustup より先に存在した歴史的経緯で、模倣しない）、さらに `~/.yaoxiang`
  は既にコードベースの既存ネームスペース（std グローバルフォールバックスロット）；環境変数
  `YAOXIANG_HOME` での上書きをサポート（先例 RUSTUP_HOME /
  DENO_INSTALL、CI とコンテナシナリオに対応）；Windows は `%USERPROFILE%\.yaoxiang`
- **バージョンディレクトリ = ディストリビューション展開ルート**：`versions/<ver>/`
  はポータブル展開、deb インストールツリーと完全に同型、1 バージョンのインストールは 1 ディストリビューションの展開そのもの —
  3 チャネル間の構造的分岐ゼロ
- コマンド面（rustup 相当）：`yx toolchain install / default / update / list / uninstall`、`yx self update`
  を含む；他の動詞はそのままエンジンに透過
- **バージョンロックは構造的保証**：fmt などのツールは構文と同期して進化する（古い fmt は新構文を認識しない）、バージョン解決はフロントドアで一度完了し、組全体が切り替わる — 「新エンジンに旧 fmt」の組み合わせ空間は存在しない；将来 fmt/LSP を独立バイナリに分割する場合も同じバージョンの
  `bin/` 内に配置される
- プロジェクトレベル pin：`yx-toolchain.toml`（先例 rust-toolchain.toml、コマンド名に準拠；`yaoxiang.toml`
  には入れない — パッケージマニフェストはツールチェーンバージョンをライブラリ利用者に強制すべきでない）
- ブートストラップエントリ（`curl | sh` /
  `irm | iex`）で最新の stable パッケージ一括インストール：`versions/` に展開、`bin/yx`
  をインストールルートに配置、`settings.toml`
  にデフォルトバージョンを書き込み（rustup 「ブートストラップでマネージャをインストール」分解の等価収束 — フロントドアとエンジンが同パッケージ、二段階不要）
- ミラーソース設定可能（settings.toml）、国内ユーザへの配慮を継続（Z3 ダウンロードと同じネットワーク問題）
- **バージョン管理は自己完結不変量を壊さない**：各バージョンは完全なディストリビューションツリー、rpath と exe 相対 std 検索はツリー内で自己完結し、フロントドアは構造を変更せずディスパッチのみ

`.deb` と Inno は**システムレベル並列配置**チャネル（root / Program
Files の単一バージョン、`apt upgrade`
/ コントロールパネルで更新）、サーバ、CI、純粋初心者向けシナリオ向け；マネージャとの共存は PATH 順序に依存（先例：apt の rustc と rustup の共存）。全チャネルが同一の成果物ツリーを共有する。

`.deb`
レイアウトは同じディレクトリツリーを再利用：`/usr/lib/yaoxiang/`（ディストリビューション全体：`bin/{yx,yaoxiang-rs,libz3.so}` +
`lib/yaoxiang/std/`）+ `/usr/bin/yx` のシンボリックリンクは `/usr/lib/yaoxiang/bin/yx` を指す —
`$ORIGIN` は**実パス**で計算されるため、リンク後も同じディレクトリの `libz3.so`
を解決し、展開パッケージ構造と同型。ブランドフルネームはパッケージ名と製品名に残し（`apt install yaoxiang`、Inno 製品名 YaoXiang）、コマンド面は統一
`yx` — Go と同じ：パッケージ名 `golang-go`、コマンド `go`。apt リポジトリは GitHub
Pages で静的ホスティング（Packages/Release/InRelease メタデータは GPG 署名、release
CI から発行）；将来的に Debian/Ubuntu 公式収録を申請可能（期間が長く、バージョン遅延、主経路ではない）。

### 標準ライブラリディレクトリ

`lib/yaoxiang/std/`
の内容はすべてリポジトリの静的ファイルから来ており、パッケージングは**純粋コピー**で、ランタイム生成エントリはない：

| 層                                | ソース                                                                    | 性質                                               |
| --------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------- |
| native モジュール（io/math/…）    | リポジトリ `src/std/interfaces/*.yx` で事前生成されたインタフェースビュー | インタフェースシグネチャビュー（実装はバイナリ内） |
| .yx 層モジュール（test/… 増加中） | リポジトリ `src/std/*.yx` をそのままコピー                                | 実ソース                                           |

事前生成ビューは `StdModule::exports()` から派生（`src/std/gen_interfaces.rs` の
`generate_all_interfaces()`）、**ランタイム生成サブコマンドは設けない**（2026-09-10 裁定：パッケージング完成後、サブコマンドは余分なインタフェース面）。同期は「生成物の入库 + テストゲート」パターンを採用（RFC-013 コード表と同じ）：`test_committed_interface_files_match_generation`
で事前生成ファイルと現在の生成結果をバイト単位で比較、ドリフトで赤；治癒は bless エントリ
`cargo test update_committed_interface_files -- --ignored`。生成ロジックは crate 内部の `StdModule`
実装に依存し、build.rs に下ろせないため、ゲートは構築時ではなくテスト時に置く。

**ランタイム検索チェーン** — `find_std_interface_file` に exe 相対検索を追加：

1. プロジェクト `.yaoxiang/vendor/std/<name>.yx`（プロジェクトオーバーライド、現状）
2. **exe の所在ディレクトリの
   `../lib/yaoxiang/std/<name>.yx`（新規）**：ポータブル展開、マネージドインストール（`versions/<ver>/`）、deb 並列配置で統一ヒット
3. `~/.yaoxiang/std/<name>.yx`（グローバルフォールバック、手動オーバーライド用として保持）

現状このチェーンはほぼ壊れている（LSP 呼び出しがプロジェクトディレクトリを渡さない、`package init`
が書き込む `.yaoxiang/std` がチェーンにない）、今回は付随的に修正し、`package init` の出力を
`.yaoxiang/vendor/std` に統一する（パッケージマネージャ vendor ディレクトリと一致）。

**コンパイル権威は変わらない**：`.yx` 層は引き続き `include_str!`
でインライン展開（RFC-036 の「std バージョンとバイナリの厳密バインド」不変量を維持）。ディストリビューションディレクトリの位置付けは**可読ビュー +
LSP 解決ソース**で、コンパイル入力ではない — ユーザがディストリビューションディレクトリ内の `.yx`
を手作業で変更してもコンパイラは採用しない（「`Lib/`
を変更すれば即時有効」の Python 風セマンティクスを开放するかは、開放問題を参照）。

### Wasm ビルド

**独立を維持、cargo-dist に統合しない。**

cargo-dist が管理するのは「コンパイラをユーザに配布する」こと、wasm は「オンライン playground をドキュメントサイトに埋め込む」 —
2 つは完全に異なるデリバラブル。

| 側面             | やり方                                      |
| ---------------- | ------------------------------------------- |
| ビルドツール     | `wasm-pack build` を維持                    |
| CI workflow      | `_build-wasm.yml` 独立ジョブを維持          |
| トリガタイミング | 同じ release タグプッシュで並列の独立ジョブ |
| 公開ターゲット   | `docs/public/wasm/` → GitHub Pages          |

### npm 公開

| パッケージ             | 内容                                       | 状態                                                                                                                                                                                                             |
| ---------------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@yaoxiang/cli`        | ディストリビューションダウンロード wrapper | 延期：cargo-dist の npm wrapper も同様にフラット成果物仮定に基づくため、インストーラとともに廃止；npm チャネルが必要なら、wrapper を自作（再構成パッケージをダウンロードして展開、展開インストールと同ロジック） |
| `@yaoxiang/playground` | wasm ライブラリ（JS + .wasm）              | オプション、現状 docs のみ公開                                                                                                                                                                                   |

2 つは競合せず、名前も競合しない。

### 既存リリースフローとの統合

現状の `release.yml`：main プッシュ → check-version（`v{version}` タグが存在しない場合のみ通過）→
build / build-wasm / security / test 4 系統 → release ジョブ（タグプッシュ +
`generate-commit-list.ts` で @mentions を含む body 生成 + 成果物アップロード）。

cargo-dist 生成のパイプラインはタグ駆動で announce/publish を内蔵し、fmt/clippy/test/audit ゲートを含まず、release
notes 形式も merge commit changelog を担えない。**全体を一括置換すると既存リリース儀式を壊す**（PR →
CI 全グリーン → バージョン上げ → merge commit が changelog）。

統合原則：**トリガとゲートは現状維持、ビルドは `cargo dist build` に委ねる、公開は現状維持。**

1. check-version / security / test とタグ作成を `dist-release.yml` の `gate` / `security` / `test` /
   `tag` ジョブに統合、トリガは main プッシュを維持（**タグ駆動は不可**：workflow は `GITHUB_TOKEN`
   でプッシュしたタグでは他の workflow がトリガされない；旧 `release.yml`
   はこのためフラットバイナリを自作する必要があり、`_build-platforms.yml`
   と共に削除済み、本ファイルがリリースの単一エントリ）
2. タグ生成後にビルド：`plan` ジョブで dist がランナー/システム依存マトリクスを計算 →
   `cargo dist build`（5 ターゲット）→ `package-dist.sh` でターゲットごとに再構成 → Inno
   Setup ジョブ（Windows 再構成パッケージを入力にウィザードをビルド、`/DMyAppVersion=`
   でバージョンを注入、二度目のコンパイル不要）→ `_build-wasm.yml`（並列ジョブ）
3. publish ジョブ：`generate-commit-list.ts`
   で body 生成（既存スクリプト再利用）→ 再構成パッケージ + `.sha256` + `.deb` + wasm + Setup
   exe をアップロード、`action-gh-release` で自作 Release；独立 `publish-apt` ジョブで GitHub Pages
   apt リポジトリメタデータを発行（`secrets.APT_GPG_KEY`
   未設定時は自動スキップ、他チャネルに影響なし）
4. 再公開/再ビルド：`workflow_dispatch` で既存タグを指定（`gate` はタグ作成ステップをスキップ）

### Nightly 公開

cargo-dist はネイティブ nightly サポートなし（[axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143)、依然 open
feature request）。

既存の cron + タグ上書き方式を維持、ビルド部分を `_build-platforms.yml` から `cargo dist build`
に変更 — 本質的に cargo コマンドなので、nightly.yml で直接呼び出せる。workflow 再利用は行わない（想定の
`uses: ./release.yml` は不可：再利用側は `workflow_call`
トリガが必要、かつ cargo-dist ワークフローはタグ駆動でビルドと公開が結合）：

```yaml
# nightly.yml（移行後）
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh（正式版と同じ仕組み）
  publish: # 現状維持：nightly タグを作成/移動 → GitHub Pre-release を上書き
```

### cargo-dist 設定（既実装 dist-workspace.toml）

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# dist バージョンを固定（Cargo.toml SemVer 構文）
cargo-dist-version = "0.32.0"
ci = "github"
# インストーラはすべて自社、cargo-dist はビルド + 圧縮パッケージ + チェックサムのみ
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# 生成されたワークフローは意図的に改変されている（package-dist.sh 再構成 + 自社公開段、RFC-037）、
# generate テンプレートからのドリフトを理由にビルドを拒否しない
allow-dirty = ["ci"]
```

上記はリポジトリ vendor の実設定（`cargo dist init`
で生成後、必要に応じて修正）；`cargo-dist-version`
は 0.32.0 に固定、生成されたワークフローは vendor してリポジトリに入れ review を受け、実行時に取得しない。ビルドプロファイルは init がルートの Cargo.toml の
`[profile.dist]` に注入（inherits release、lto=thin）、2 バイナリは `target/<triple>/dist/`
に配置されて再構成スクリプトが取得。

### package-dist.sh（既実装）

リポジトリの `scripts/release/package-dist.sh` を基準とする、要点：

- 2 バイナリは `cargo dist build` のビルド出力ディレクトリ `target/<triple>/dist/`
  から直接取得（profile=dist）—
  cargo-dist 自体のフラット単一バイナリアーカイブはデリバラブルではなく、同名再構成パッケージを
  `target/distrib/` で直接上書き
- Z3 共有ライブラリもビルド出力ディレクトリから取得 —
  build.rs リンク時に該当プラットフォームの共有ライブラリを選択してコピー（`copy_shared_lib`）、**単一ソース**：パッケージングスクリプトは Z3 バージョンとプラットフォームディレクトリ名を知る必要がないし、知る必要もない；Z3 ライセンステキストはライブラリとともに配置され、パッケージングで同梱（MIT 配布義務）、macOS 側ではパッケージング時に dylib の install_name を
  `@rpath` に統一し、バイナリを ad-hoc 再署名
- std ディレクトリは純粋コピー：`src/std/interfaces/*.yx`（事前生成インタフェースビュー）+
  `src/std/*.yx`（.yx 層の実ソース）
- README/LICENSE を同梱；再パッケージング（Windows zip / その他 tar.gz；Git
  Bash に zip がない場合は zip → System32 bsdtar → PowerShell の三段フォールバック）し、`.sha256`
  を再計算
- Linux かつ `dpkg-deb` 利用可能時は `build-deb.sh` を呼び `.deb` を生成（`/usr/lib/yaoxiang`
  並列配置ツリー + `/usr/bin/yx` シンボリックリンク）

### 廃止された手書き CI

移行完了後に調整されるファイル：

| ファイル                                 | 行数        | 処理                                                                     |
| ---------------------------------------- | ----------- | ------------------------------------------------------------------------ |
| `.github/workflows/_build-platforms.yml` | 254         | 削除（cargo-dist ビルドマトリクスで置換）                                |
| `.github/workflows/release.yml`          | 189         | 削除（ゲートとタグ作成を dist-release.yml に統合、リリース単一エントリ） |
| `.github/workflows/nightly.yml`          | 173         | ビルド段を `cargo dist build` に変更、公開ロジックは保持                 |
| `scripts/build/setup.iss`                | ~250        | **保持し正式採用**（Windows ウィザード）                                 |
| **合計削除**                             | **~600 行** |                                                                          |

保持される：

- `ci.yml`（日次 fmt + clippy + test + MSRV、公開フローに属さない）
- `_build-wasm.yml`（独立ビルドフロー、dist-release.yml の並列ジョブに紐付け）
- `_build-z3-wasm.yml`（wasm 専用 Z3）
- `docs-deploy.yml`（ドキュメントデプロイ）

### 受け入れ基準

「箱から出してすぐ使える」はテスト可能で、移行完了の判定は「新旧成果物が一致」ではなく、以下すべてが通過すること：

- クリーンなマシン（Rust / Z3 / `~/.yaoxiang`
  なし）でいずれかのプラットフォームの圧縮パッケージを展開し、直接 `bin/yaoxiang-rs --version`
  が成功する — `LD_LIBRARY_PATH` 設定不要（rpath 有効）
- 展開ディレクトリ内の `lib/yaoxiang/std/*.yx`
  すべてが読める：native モジュールはシグネチャインタフェースビュー、`.yx` 層は実ソース
- 展開ディレクトリ下のサンプルプロジェクトで LSP を起動し、std メンバー補完 / ジャンプ定義が使える（exe 相対検索有効）
- 公式ガイド通りに `/usr/local`（または
  `~/.yaoxiang`）に展開して PATH に追加した後、任意のディレクトリで `yx --version` が成功する
- `apt install yaoxiang`（自社リポジトリ）後に実行可能、`apt upgrade`
  でバージョン追従可能；`/usr/bin/yx` シンボリックリンク下のエンジンの `$ORIGIN`（実パスで解決）が
  `bin/libz3.so` を解決する
- クリーン環境で `curl ... | sh` と `irm ... | iex` 実行後に `yx` が実行可能、PATH が配置される
- `yx toolchain install <ver>` / `default` / `update`
  が有効：複数バージョン共存、`yx-toolchain.toml`
  プロジェクト pin がデフォルトバージョンより優先、フロントドアが正しいバージョンにディスパッチ（バージョンツリー内の rpath と std 検索が自己完結、「新エンジンに旧 fmt」組み合わせなし）
- ポータブル展開後 `yx` が隣接 `yaoxiang-rs` にフォールバック、マネージドインストールと同じ動作
- Inno Setup インストール後ディレクトリ構造が完全、PATH 有効、アンインストール可能
- Release 資産完備：5 プラットフォームの再構成パッケージ + `.sha256` が実際の内容と一致
- Release body は `generate-commit-list.ts` の出力（merge commit changelog が完全）
- nightly 成果物は Pre-release、最新の正式タグに影響しない

## トレードオフ

### 利点

- **箱から出してすぐ使える**
  — ポータブル展開ですぐ使える（rpath + 同ディレクトリ共有ライブラリ）、インストーラが完全なディレクトリを敷設
- **標準ライブラリが読める** — ユーザは Python `Lib/` を読むように直接 std を読める（必須要件達成）
- **保守コスト削減** — 手書きビルド YAML 約 600 行を cargo-dist + 自社スクリプト約 80 行に置換
- **クロスプラットフォーム一貫性**
  — 全プラットフォーム動的リンク + 同ディレクトリ共有ライブラリ、例外なし
- **二重インストール**
  — 標準チャネルは新規コードゼロ（Go/Zig 方式）；お手軽チャネルは Rust を参照、apt / curl / iex /
  exe 4 エントリが同じ成果物構造を共有
- **バージョン管理内蔵** — フロントドア `yx` が rustup/Go
  GOTOOLCHAIN 相当、Python/Node が事後的に pyenv/nvm/pdm で補講する生態系分裂を回避；ツールのバージョンロックは構造的保証であり規約ではない

### 欠点とリスク

- **お手軽チャネルの保守面** — install.sh / install.ps1（ワンラインスクリプト、ほぼ進化しない）+
  `.deb` と apt リポジトリメタデータ公開（release CI 自動化）+ `yx` フロントドア crate
- **エンジン改名の波及範囲** — `yaoxiang` → `yaoxiang-rs`
  は CI 成果物名、Inno、テスト、ドキュメントの一括移行が必要（フェーズ 5 内で完了）
- **学習コスト** — チームが cargo-dist 設定を学ぶ必要がある
- **cargo-dist 上流リスク** —
  2025 年中頃に Axo 停止に遭遇、同年 9 月に原作者が復活し継続リリース（0.29 →
  0.32+）；`dist-version` 固定 + 生成物 vendor してリポジトリで review で軽減
- **cargo-dist ネイティブ nightly なし** — nightly 公開部分は依然手書きが必要

### RFC-014b との関係

|              | RFC-014b                                  | RFC-037                                      |
| ------------ | ----------------------------------------- | -------------------------------------------- |
| **範囲**     | サードパーティパッケージのビルドと配布    | コンパイラ自体のパッケージングと配布         |
| **ツール**   | `yaoxiang build` / `yaoxiang publish`     | `cargo-dist` + 自社スクリプト                |
| **成果物**   | サードパーティパッケージの FFI ライブラリ | コンパイラ + 標準ライブラリ + ツールチェーン |
| **相互排他** | いいえ、補完                              | いいえ、補完                                 |

## 代替案

| 方案                                     | なぜ不採用                                                                                                                                                                                          |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **手書き CI を継続**                     | 既に約 870 行手書き、重複作業、DLL 漏れを起こしやすい                                                                                                                                               |
| **自社パッケージングツールを作成**       | 車輪の再発明はしない、cargo-dist は既に成熟                                                                                                                                                         |
| **tar.gz のみでインストーラなし**        | 唯一公式チャネルは展開 + PATH；Inno は Windows ウィザード慣行のためだけ（国内ユーザは既裁定で保持）                                                                                                 |
| **Docker 配布**                          | コンパイラと言語ツールチェーンはネイティブバイナリが必要、コンテナシナリオではない                                                                                                                  |
| **自社 Homebrew tap 作成**               | tap は一律コミュニティ保守（homebrew-core）、自作は時期尚早の要件；お手軽チャネルの macOS エントリは curl スクリプト                                                                                |
| **独立 `yaoxiangup` マネージャバイナリ** | rustup の先例通りで可能；しかし 2 つ目のユーザ動詞を生み、「パッケージマネージャ/fmt も独立すべきか」の対称性問題を引き起こす — フロントドア/エンジン分離で同時に解消（2026-09-09 討議で否決）      |
| **自己更新のみ、複数バージョンなし**     | 単一バージョン自己更新では複数プロジェクトの異なる pin バージョン需要を解決できない；Python/Node は公式バージョン管理が欠如、生態系が pyenv/nvm/pdm を強制的に生んだ — 既裁定で内蔵（2026-09-09）   |
| **Z3 全静的リンク**                      | 既裁定で否決 — 外部システムのディレクトリ型共有ライブラリ配布は自然な形態；exe に詰め込むのも外に置くのも「どちらもディストリビューションに同梱する必要がある」点では同等で、後者は交換可能性も失う |
| **Inno Setup 廃止**                      | 既裁定で否決 — Windows ウィザードとして保持（追加チャネル）                                                                                                                                         |
| **cargo-dist ネイティブインストーラ**    | フラットバイナリ仮定が bin/+lib/ 構造と衝突、インストール直後にライブラリ欠落                                                                                                                       |
| **自社保守 WiX/MSI**                     | cargo-dist ジェネレータを失うコストは Inno が既にカバーする価値を上回る                                                                                                                             |

## 実装戦略

### フェーズ 1：言語側の変更（P0）

1. `build.rs`：全プラットフォーム統一動的リンク + rpath link-arg；`copy_dll()` を
   `copy_shared_lib()` に拡張（so/dylib/dll）
2. リポジトリで native インタフェースビューを事前生成（`src/std/interfaces/`）、テストゲートで
   `StdModule::exports()` との同期を強制（gen-std サブコマンドは既裁定で取消）
3. `find_std_interface_file` に exe 相対検索分岐を追加；`package init` の出力パスを
   `.yaoxiang/vendor/std` に統一

### フェーズ 2：cargo-dist 統合（P0）

1. `cargo dist init` を実行して初期設定を生成（`installers = []`、dist-version 固定）
2. `package-dist.sh` を記述（再構成 + .yx ソースコピー + チェックサム再計算）
3. `dist-release.yml` がすべてを承载：main プッシュトリガ → バージョンドアゲート → ゲート（audit /
   fmt / clippy / test）→ タグ作成 → dist build → 再構成 → wasm 並列 → 自社 publish +
   apt；`release.yml` と `_build-platforms.yml` を削除
4. 新旧パイプラインを並走、受け入れ基準で逐次検証

### フェーズ 3：旧 CI 停止（P1）

1. `_build-platforms.yml` と `release.yml` を一括削除（リリース単一エントリ：`dist-release.yml`）
2. `nightly.yml` ビルド段を `cargo dist build` に変更
3. `setup.iss`
   を新成果物構造に統合（Inno を正式採用；バージョン番号は Cargo.toml から注入、sed 置換を撤廃）

### フェーズ 4：お手軽チャネル（P2）

1. `install.sh` / `install.ps1`（プラットフォーム検出 → 最新再構成パッケージダウンロード →
   `versions/` に展開 → `bin/yx` をインストールルートに配置 → `settings.toml`
   にデフォルトバージョン書き込み →
   PATH 通知/書き込み；フェーズ 5 と同じサイクルで着地、最終形へ一歩で到達）
2. `.deb` パッケージング（`package-dist.sh` と同じディレクトリツリー + `/usr/bin`
   シンボリックリンクを再利用）+ GitHub Pages 静的 apt リポジトリ（メタデータ GPG 署名、release
   CI から公開）

### フェーズ 5：フロントドア yx とエンジン改名（P2）

1. 現モノリスを `yaoxiang-rs`
   に改名（CI 成果物名、Inno、テスト、ドキュメントを全て一括スキャン、一回で移行完了）
2. フロントドア小型 crate `yx`
   を新設（workspace メンバー）：バージョン解決、release 成果物ダウンロード、tar/zip 解凍、settings.toml、ディスパッチ（toolchain/self 動詞のみ保持、他は透過、clap を使わず手書きディスパッチで引数逐語転送を保証）；バージョンインデックスは GitHub
   Releases latest API から取得（ミラーソースは ghproxy 風プレフィックス連結）；`yx`
   コマンド名は重複チェック済み（主流ディストリビューション/Homebrew に同名の常用コマンドなし、ある小規模ツールが yx を別名として使用）
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml`
   プロジェクト pin + ミラーソース + `yx self update`
4. ブートストラップスクリプト：`curl | sh` / `irm | iex`
   → 再構成パッケージをダウンロードしてフロントドアとデフォルト stable を一括インストール（フェーズ 4 と同じサイクルで最終形に着地）

### フェーズ 6：オプションフォローアップ（いずれもブロックしない）

1. winget 送信（Inno exe を指す、コミュニティ保守、対称 homebrew-core モデル）
2. `.rpm`（dnf ユーザ、`.deb` と同型）
3. Homebrew：知名度が homebrew-core 参入基準に達したらコミュニティが提出
4. npm `@yaoxiang/cli` 自作 wrapper（名称は現在未登録）

## 開放問題

### 未決

- **.yx 層に「ディストリビューションディレクトリを手で変更すればコンパイルで採用される」Python 式セマンティクスを提供するか？**
  デフォルトは否 — コンパイル権威は RFC-036 のインライン展開を維持（std バージョンとバイナリの厳密バインド）、ディストリビューションディレクトリは可読ビュー +
  LSP 解決ソースに位置付け。将来开放する場合、バージョンバインド不変量を再検討する必要あり。

### クローズ済み

以下の問題は設計討議で解決済み：

- ~~Windows での Z3 静的リンクの実現性？~~ →
  **静的リンクしない、全プラットフォーム動的**（2026-09-09 確認維持）
- ~~gen-std-interfaces サブコマンド命名？~~ →
  **サブコマンドを設けない**（2026-09-10 裁定：通常のパッケージング完成後、サブコマンド面は余分；native インタフェースビューはリポジトリ事前生成
  `src/std/interfaces/` + テストゲート同期に変更、パッケージングは純粋コピー）
- ~~Inno Setup を保持するか？~~ → **Windows ウィザードとして保持（追加チャネル）**
- ~~ディストリビューション構造は物理的に標準ライブラリソースを同梱するか？~~ →
  **必須**（ユーザ可読性は Python `Lib/` に合わせる、2026-09-09 裁定）
- ~~cargo-dist ネイティブインストーラ（shell/powershell/homebrew/msi/npm）？~~ →
  **すべて廃止**、インストーラは自社（フラット仮定が bin/+lib/ と衝突）
- ~~インストーラ戦略？~~ →
  **二重**（2026-09-09 終裁）：標準チャネル = ディストリビューションがそのままプロダクト（Go/Zig 方式、展開 +
  PATH）；お手軽チャネルは Rust を参照しワンライナーインストール — Linux
  `apt`（自社 deb リポジトリ）/ `curl | sh`、Windows `irm | iex` / Inno
  exe。不採用：MSI、cargo-dist ネイティブインストーラ、自社 brew tap
- ~~バージョン管理を含めるか？~~ →
  **必須、インストーラ体系に属する**（2026-09-09 裁定、同日早朝の「遠期独立 RFC」境界を推翻）：動機は Python/Node が公式バージョン管理を欠くため pyenv/nvm/pdm の補講という生態系分裂
- ~~バージョン管理形態：独立バイナリ or サブコマンド？~~ →
  **フロントドア/エンジン分離**（2026-09-09 終裁、三ラウンド収束 A→C→命名反転）：常用コマンド `yx`
  = フロントドア（小型バイナリ、toolchain/self 動詞内蔵）、エンジン `yaoxiang-rs` = 現 `yaoxiang`
  モノリスの改名；ディレクトリ構造 `versions/<ver>/`
  — バージョンが第一級概念、バージョンディレクトリはディストリビューション展開ルートそのもの、ツールはバージョン全体でロック（旧 fmt は新構文を認識しない；かつて想定した内層
  `toolchains/` は冗長で取消）。先例：Go `go` フロントドア +
  GOTOOLCHAIN、rustup プロキシディスパッチ
- ~~cargo-dist extra-artifacts 条件実行？~~ → **`package-dist.sh` スクリプトで処理、shell
  case 分岐で**
- ~~標準ライブラリインタフェースのバージョン互換性？~~ →
  **コンパイラバージョンと同じディストリビューションで発行、同一圧縮パッケージ内**

## 参考文献

- [cargo-dist 公式ドキュメント](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 ビルド設定 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

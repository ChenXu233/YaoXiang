---
title: 'RFC-037: 工業的配布方案 — cargo-dist ベースのコンパイラ/ツールチェーンパッケージング'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-09-10'
accepted: '2026-09-09'
issue: '#230'
status: '承認済み'
---

# RFC-037: 工業的配布方案 — cargo-dist ベースのコンパイラ/ツールチェーンパッケージング

> 本 RFC は [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
> と補完関係にある。RFC-014b は
> **YaoXiang パッケージマネージャ**がサードパーティ製パッケージをビルド・配布する方法を定義する。一方、本 RFC は
> **YaoXiang コンパイラ/ツールチェーン自体**のパッケージングと配布方法を定義する。

## 概要

クロスプラットフォームのビルドオーケストレーションには
`cargo-dist`（Rust エコシステムのバイナリ配布ツール）を用い、ディストリビューションパッケージの構造は独自のスクリプトが担う。中心となる約束は 2 つ：配布パッケージは**標準ライブラリのソースディレクトリを物理的に同梱**し（Python の
`Lib/`
を読むようにユーザーが直接読める）、全プラットフォームで動的リンクされる Z3 共有ライブラリをパッケージに同梱して配布する。コマンドモデルは**フロントドア/エンジン分離**とする：常用コマンドは
`yx`（小さなフロントドア、バージョン管理を内蔵——rustup/Go GOTOOLCHAIN 相当）、エンジンは
`yaoxiang-rs`（現 `yaoxiang`
モノリスを改名）。これにより、Python/Node のように事後的に nvm/pdm で補填する生態系の分裂を防ぐ。インストール方式は二重化：標準チャネルは Go/Zig に合わせ、**配布パッケージ＝プロダクト**（展開 +
PATH）、簡易チャネルはワンライナーインストール（Linux `apt` / `curl | sh`、Windows `irm | iex` /
Inno exe ウィザード）。`libz3.dll`
が見つからない問題、標準ライブラリがユーザーから不可視な問題、CI スクリプトの重複保守といった課題を解決する。

## 動機

### なぜこの機能が必要か

YaoXiang をダウンロードしたユーザーは**箱から出してすぐに使える**べきであり、標準ライブラリは**ユーザーから直接読める**べきで、バイナリの中のブラックボックスになっていてはならない。

### 現状の問題

#### 問題 1：Windows ユーザーがダウンロードしても動かない

現在の Release には `yaoxiang.exe` しかアップロードされておらず、`libz3.dll`
がパッケージに含まれていない。Windows ユーザーがダブルクリックで実行すると以下のエラーが出る：

```
The code execution cannot proceed because libz3.dll was not found.
```

これは**中断性バグ**である——ユーザーは最初の段階でさえ進められない。

#### 問題 2：Release 成果物が単一 exe のみで、標準ライブラリがユーザーから不可視

現状は三重の断絶がある：

- Release 成果物は生のバイナリのみで、標準ライブラリが配布物に同梱されていない
- LSP のインターフェースファイル検索チェーンがほぼ切断されている：`find_std_interface_file`
  の呼び出し時にプロジェクトディレクトリを伝播せず（グローバルの `~/.yaoxiang/std/`
  のみ参照し、それを埋めるフローが存在しない）；`package init` が書き込む先は `.yaoxiang/std`
  であり、検索チェーンに含まれていない
- 標準ライブラリのソースコード（`.yx`
  レイヤ）とインターフェースビュー（native レイヤ）がユーザーにとって完全にブラックボックス

工業的なアプローチ：ユーザーが Python の `Lib/`
を読むように直接標準ライブラリのディレクトリを開いてソースを読めるようにする——**配布パッケージが物理的に std ディレクトリを同梱することは本方案の必須要件**（裁可済み）。

#### 問題 3：CI の手書きスクリプトが重複保守されている

現在、複数のビルドパイプラインを保守している：

| ファイル                  | 責務                         | 行数        |
| ------------------------- | ---------------------------- | ----------- |
| `_build-platforms.yml`    | クロスプラットフォームビルド | ~255 行     |
| `release.yml`             | バージョンリリース           | ~189 行     |
| `nightly.yml`             | 日次ビルド                   | ~173 行     |
| `scripts/build/setup.iss` | Inno Setup インストーラ      | ~250 行     |
| **合計**                  |                              | **~870 行** |

その大部分は重複している（Rust インストール → キャッシュ → ビルド → リネーム → アップロード）で、プラットフォームごとに書き直す必要がある。

#### 問題 4：Inno Setup のバージョン番号がハードコード

`setup.iss` の `MyAppVersion` が `0.7.0` と直書きされており、ビルド時に `sed`
で置換している。迟早破綻する。

#### 問題 5：RFC-014b との境界が曖昧

RFC-014b は「YaoXiang パッケージのビルド・配布機構」（つまり `yaoxiang.toml` の `[build]` と
`[binaries]`
設定）を定義しているが、**「YaoXiang コンパイラ自体をどうリリースするか」をカバーしていない**。本 RFC はこの空白を埋める。

## 提案

### 中心設計

cargo-dist は**ビルドオーケストレーション層**のみを担い、パッケージ構造とインストーラはすべて自前とする。責務分担は以下の通り：

```
cargo-dist の責務（ビルドオーケストレーション層）:
  ├── クロスプラットフォームコンパイル（5 ターゲット）
  └── 圧縮パッケージとチェックサムの生成
  （ネイティブインストーラと npm wrapper は廃止——フラットなバイナリ前提が bin/+lib/ 構造と衝突するため）

build.rs は引き続き担当:
  └── Z3 ダウンロード/リンク（全プラットフォーム動的 + rpath）

YaoXiang 自前スクリプト:
  ├── package-dist.sh — パッケージ構造の再構成（bin/ + lib/）、共有ライブラリの同梱、
  │   std ディレクトリの充填（リポジトリで事前生成したインターフェースビュー + .yx レイヤソース）、チェックサム再計算
  └── Inno Setup — Windows インストールウィザード（既存資産；完全なディレクトリ構造を敷設）

コマンドモデル（フロントドア/エンジン分離）:
  ├── yx — フロントドア（新規小型 crate）：バージョン解決 + ディスパッチ；toolchain/self 動詞のみ保持、他は透過転送
  └── yaoxiang-rs — エンジン（現 yaoxiang モノリスを改名）：compile/run/package/fmt/lsp サブコマンド

インストール方式（二重化）:
  ├── 標準チャネル（Go/Zig モデル）: 配布パッケージ＝プロダクト、展開 + PATH
  └── 簡易チャネル（Rust モデル）: ワンライナーインストール + バージョン管理（フロントドア yx に内蔵）
      ├── Linux: apt（自前 deb リポジトリ、システムレベル平置き）/ curl … | sh（ワンクリックでパッケージ全体導入）
      ├── Windows: irm … | iex（ワンクリックでパッケージ全体導入）/ Inno Setup ウィザード（既存資産、システムレベル平置き）
      └── macOS: curl … | sh（brew は homebrew-core コミュニティに任せる）
```

### 配布ディレクトリ構造（裁可済み：標準ライブラリソースを物理同梱）

ユーザーは Python の `Lib/`
を読むように標準ライブラリを直接読めるべき——配布パッケージに std ディレクトリを同梱することは必須要件であり、パッケージングの詳細ではない。Z3 も同様：外部システムとして、ディレクトリ型の共有ライブラリ配布が自然な形態である——`.so`
を exe に埋め込もうが外に置いて動的リンクしようが「どちらもパッケージ同梱が必要」という意味では等価であり、後者には交換可能性が保たれる利点がある。

各プラットフォームの配布パッケージは、`package-dist.sh` が cargo-dist のビルド後に再構成する：

```
yaoxiang-{version}-{target}.tar.gz / .zip     （ポータブル即利用：展開後 bin/ から直接実行）
├── bin/
│   ├── yx                            # フロントドア（または yx.exe）
│   ├── yaoxiang-rs                   # エンジン（または yaoxiang-rs.exe）
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # ユーザーが直接読める（Python Lib/ モデル）
│           ├── io.yx                 # native モジュール：リポジトリで事前生成したインターフェースビュー
│           ├── math.yx
│           ├── test.yx               # .yx レイヤ：リポジトリの実ソースをそのままコピー
│           └── ...
├── README.md
└── LICENSE
```

インストール＝任意のディレクトリに展開 + `bin/` を PATH に追加（Go の `/usr/local/go/bin`
モデル；`~/.yaoxiang/` への展開が一般的選択）。Windows では Inno
Setup ウィザードが同じことを行う（デフォルトは Program Files）。ポータブル展開時、`yx`
フロントドアは `~/.yaoxiang` 状態を持たず、隣接の `yaoxiang-rs`
にフォールバックする——マネージドインストールと挙動は一致する。エンジンの rpath と exe 相対 std 検索はフロントドアの存在によって変わらない。

### プラットフォームサポート

| プラットフォーム | target triple               | 説明                    |
| ---------------- | --------------------------- | ----------------------- |
| Linux x86_64     | `x86_64-unknown-linux-gnu`  | メインプラットフォーム  |
| Linux ARM64      | `aarch64-unknown-linux-gnu` | CI 上でクロスコンパイル |
| macOS x86_64     | `x86_64-apple-darwin`       | Intel Mac               |
| macOS ARM64      | `aarch64-apple-darwin`      | Apple Silicon           |
| Windows x86_64   | `x86_64-pc-windows-msvc`    | メインプラットフォーム  |

合計 5 ターゲット。Windows
ARM64 は当面サポート対象外（Z3 公式に ARM64 プレコンパイルパッケージがないため）。

### Z3 配布戦略

**全プラットフォーム動的リンク**（再確認の上で維持）：

| プラットフォーム | 変更点                 | 成果物        |
| ---------------- | ---------------------- | ------------- |
| Linux            | **静的→動的へ変更**    | `libz3.so`    |
| macOS            | **静的→動的へ変更**    | `libz3.dylib` |
| Windows          | 変更なし               | `libz3.dll`   |
| wasm32           | 変更なし（静的リンク） | 内蔵 `.a`     |

理由：

- **一貫性** — 3 プラットフォームの挙動が統一され、特例がなくなる
- **これは外部ライブラリであり、共有ライブラリで配布すべき**。Python（`python3.dll`+`DLLs/lib*.dll`）、Node（`node`+`lib/`）も同様の方式
- **ユーザーが Z3 をアップグレードする際にコンパイラのバージョンを待つ必要がない** —
  `.so`/`.dylib`/`.dll` を差し替えるだけ
- **バイナリサイズが小さくなる** — Z3 は小さくなく、静的リンクだと exe が数 MB 膨れる

動的リンクには**必要な付帯対応**がある：Linux/macOS の動的リンカはデフォルトではバイナリの置かれたディレクトリを検索しないため、rpath を注入しなければ「展開即利用」が成立しない（Windows はデフォルトで exe ディレクトリを検索するため対応不要）。対応する
`build.rs` の修正：

```rust
// 動的リンク + rpath を統一
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 配布パッケージのレイアウトは統一されていないため、lib/bin 両ディレクトリを探索
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037：全プラットフォーム動的リンク。共有ライブラリは配布パッケージ bin/ に同梱し、ユーザーが Z3 をまとめて差し替えてアップグレード可能
    if target_os == "windows" {
        // MSVC の import lib は libz3.lib という名前
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // 動的リンカはデフォルトではバイナリの置かれたディレクトリを検索しないため、rpath を注入しなければ「展開即利用」が成立しない
        // （配布パッケージ内では exe と libz3 は同じ bin/ にある；Windows はデフォルトで exe ディレクトリを検索するため対応不要）
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

**「全プラットフォーム静的リンク」は目標としない。**これは特例の排除ではなく、合理的な状況の排除を誤った方法で行うものだ。共有ライブラリは外部ライブラリの通常の配布方式である。

### インストーラサポート

主要言語ツールチェーンの配布方式との比較（2026-09 時点の調査）：

| 言語     | 公式配布物                   | 公式インストール方式                       | インストーラ保守者                 |
| -------- | ---------------------------- | ------------------------------------------ | ---------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | 公式ドキュメントがそのまま「DL→展開→PATH」 | なし（brew/apt はコミュニティ）    |
| Zig      | `zig/{bin,lib/std}` tarball  | 同上、公式インストールスクリプトなし       | なし（homebrew-core コミュニティ） |
| Node     | `{bin,lib,include}` tarball  | tar + 公式 pkg/msi                         | チームが自作                       |
| Rust     | 複数コンポーネント tarball   | rustup                                     | チームが自作                       |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                | チーム + brew コミュニティ         |
| Deno/Bun | 単一バイナリ zip             | 公式 curl スクリプト                       | チームが自作（スクリプトは極小）   |
| Gleam    | cargo-dist 単一バイナリ      | cargo-dist 生成スクリプト                  | cargo-dist                         |

3 つの法則：

- **多ファイルツールチェーンでサードパーティ製ジェネレータをインストーラに使っているところはない**——cargo-dist のインストーラは単一バイナリシナリオにしか対応しない（Gleam が使えるのは外部依存なしの単一 Rust バイナリだからこそ）
- 最もシンプルなモデルは **Go/Zig の「配布パッケージ＝プロダクト」**：公式インストールガイドは展開 +
  PATH のみ、インストーラコードはゼロ；配布パッケージに可読な std ソースが同梱されているのは常態（Go の
  `src/`、Zig の `lib/std/`、Crystal の `src/`）
- curl ワンクリックインストールを提供しているところ（Deno/Bun/rustup）はすべて**自作スクリプト**でほぼ進化しない；brew フォーミュラは全て homebrew-core でコミュニティ保守され、言語チームは自前の tap を作らない（Crystal チームがフォーミュラはコミュニティのものと明言）

YaoXiang は二重モデルを採用する：

| チャネル                                                | レイヤ | 状態 | 説明                                                                                                                                            |
| ------------------------------------------------------- | ------ | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                            | 標準   | ✅   | 展開即利用（rpath + 同ディレクトリ共有ライブラリ）、extract + PATH が公式ガイド                                                                 |
| `yx`（フロントドア、バージョン管理内蔵）                | 簡易   | ✅   | rustup/Go GOTOOLCHAIN 相当：複数バージョンインストール/切替/更新 + プロジェクト pin                                                             |
| `curl ... \| sh`（install.sh）                          | 簡易   | ✅   | Linux / macOS：配布パッケージをダウンロード・再構成して `versions/` に展開、`bin/yx` をインストールルートに配置しデフォルトバージョンを書き込む |
| `irm ... \| iex`（install.ps1）                         | 簡易   | ✅   | Windows：同ロジック                                                                                                                             |
| `apt install yaoxiang`                                  | 簡易   | ✅   | `.deb`（amd64/arm64）+ GitHub Pages 静的 apt リポジトリ；システムレベル平置き、`apt upgrade` でバージョン追従                                   |
| Inno Setup exe                                          | 簡易   | ✅   | Windows ウィザード（既存資産）、システムレベル平置き、完全な bin/+lib/ 構造を敷設                                                               |
| winget / `.rpm` / homebrew-core / npm                   | —      | ⏸    | 任意でフォローアップ：winget と brew-core はどちらもコミュニティ保守、rpm は deb と同型                                                         |
| MSI / cargo-dist ネイティブインストーラ / 自作 brew tap | —      | ❌   | 「代替案」セクション参照                                                                                                                        |

**簡易チャネルは Rust を参照し、バージョン管理はフロントドアに内蔵する。**Rust の分解は「ブートストラップスクリプト（sh.rustup.rs）→
rustup
→ ツールチェーンパッケージ」であり、rustup は最初から複数バージョン・切替・更新を掌握している。Python/Node の公式インストーラにはこの層がなく、生態系は事後的に pyenv/nvm/pdm として分裂した。YaoXiang は**フロントドア/エンジン分離**を採用する（Go の
`go` フロントドア + GOTOOLCHAIN、rustup のプロキシディスパッチ、いずれも同型の形態）：常用コマンドは
`yx`、エンジンは `yaoxiang-rs`——

```
~/.yaoxiang/
├── bin/yx                  # フロントドア：小型バイナリ、バージョン解決 + ディスパッチ（プロジェクト pin > デフォルト > 隣接エンジン）
├── settings.toml           # デフォルトバージョン、ミラーリポジトリ
└── versions/               # バージョンは第一級の概念；<ver>/ がそのバージョンの配布パッケージの展開ルート
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # エンジン：compile/run/package/fmt/lsp サブコマンド
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- インストールルート `~/.yaoxiang/` は業界の慣習に合わせる（pyenv `~/.pyenv`、nvm `~/.nvm`、deno
  `~/.deno`、bun `~/.bun`、volta `~/.volta` いずれも単一ルート；rustup の `~/.cargo`+`~/.rustup`
  の二重ルートは cargo が rustup より先に存在した歴史的経緯による負債であり、追随しない）、かつ
  `~/.yaoxiang`
  は既にコードベースの既存ネームスペース（std のグローバルフォールバックスロット）；環境変数
  `YAOXIANG_HOME` でのオーバーライドをサポート（先例：RUSTUP_HOME /
  DENO_INSTALL、CI とコンテナシナリオに対応）；Windows では `%USERPROFILE%\.yaoxiang`
- **バージョンディレクトリ＝配布パッケージ展開ルート**：`versions/<ver>/`
  はポータブル展開、deb インストールツリーと完全に同型であり、バージョンのインストールは配布パッケージの展開そのもの——3 つのチャネル間で構造的分岐はゼロ
- コマンド面（rustup 相当）：`yx toolchain install / default / update / list / uninstall`、`yx self update`
  を含む；その他の動詞はそのままエンジンに透過転送
- **バージョンロックは構造的保証**：fmt などのツールは構文と同期して進化する（古い fmt は新しい構文を認識しない）、バージョン解決はフロントドアで一度に完了し全体として切り替わる——「新エンジンに旧 fmt」の組み合わせ空間は存在しない；将来 fmt/LSP を独立バイナリに分離する場合も同じバージョンの
  `bin/` 内に配置される
- プロジェクトレベル pin：`yx-toolchain.toml`（先例：rust-toolchain.toml、コマンド名に準拠；`yaoxiang.toml`
  には入れない——パッケージマニフェストがツールチェーンバージョンをライブラリの利用者に強制すべきではない）
- ブートストラップエントリポイント（`curl | sh` /
  `irm | iex`）は最新の stable パッケージ全体を一度に導入：展開して `versions/` に格納、`bin/yx`
  をインストールルートに配置、`settings.toml`
  にデフォルトバージョンを書き込む（rustup の「ブートストラップでマネージャ導入」分解の等価収斂——フロントドアとエンジンが同パッケージのため 2 ステップ不要）
- ミラーソースは設定可能（settings.toml）、中国国内ユーザーへの配慮を継続（Z3 ダウンロードと同じネットワーク問題）
- **バージョン管理は自己完結不変量を破壊しない**：各バージョンは完全な配布ツリーであり、rpath と exe 相対 std 検索はツリー内で自己完結し、フロントドアは構造を変更せずディスパッチのみ行う

`.deb` と Inno は**システムレベル平置き**チャネル（root / Program
Files に単一バージョン、`apt upgrade`
/ コントロールパネルで更新）、サーバー、CI、純粋初心者シナリオにサービスを提供；マネージャとの共存は PATH 順序で実現（先例：apt の rustc と rustup の並存）。全チャネルで同じ成果物ツリーを共有する。

`.deb`
レイアウトは同じディレクトリツリーを再利用：`/usr/lib/yaoxiang/`（配布ツリー全体：`bin/{yx,yaoxiang-rs,libz3.so}` +
`lib/yaoxiang/std/`）+ `/usr/bin/yx` のシンボリックリンクは `/usr/lib/yaoxiang/bin/yx`
を指す——`$ORIGIN` は解決後の**実パス**に基づき計算されるため、リンク後も同じディレクトリの
`libz3.so`
をヒットし、展開パッケージの構造と同一。ブランドフルネームはパッケージ名と製品名に残し（`apt install yaoxiang`、Inno 製品名 YaoXiang）、コマンド面は統一
`yx`——Go と同じ：パッケージ名 `golang-go`、コマンド `go`。apt リポジトリは GitHub
Pages で静的ホスティング（Packages/Release/InRelease メタデータは GPG 署名、release
CI から発行）；将来的には Debian/Ubuntu 公式収録を申請可能（サイクルが長くバージョンが遅延するため、主経路ではない）。

### 標準ライブラリディレクトリ

`lib/yaoxiang/std/`
の内容はすべてリポジトリの静的ファイルから来ており、パッケージングは**純粋なコピー**であり、ランタイム生成エントリポイントは設けない：

| レイヤ                                  | 出所                                                                      | 性質                                           |
| --------------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------- |
| native モジュール（io/math/...）        | リポジトリ `src/std/interfaces/*.yx` で事前生成したインターフェースビュー | インターフェース署名ビュー（実装はバイナリ内） |
| .yx レイヤモジュール（test/... 増加中） | リポジトリ `src/std/*.yx` をそのままコピー                                | 実ソース                                       |

事前生成ビューは `StdModule::exports()` から派生（`src/std/gen_interfaces.rs` の
`generate_all_interfaces()`）、**ランタイム生成サブコマンドは設けない**（2026-09-10 裁可：パッケージングが形になった後、サブコマンドは余分なインターフェース面）。同期は「生成物コミット + テストゲート」パターンを採用（RFC-013 コード表と同じ）：`test_committed_interface_files_match_generation`
が事前生成ファイルと現在の生成結果をバイト単位で比較し、差異があれば赤；修復は bless エントリ
`cargo test update_committed_interface_files -- --ignored` で行う。生成ロジックは crate 内部の
`StdModule` 実装に依存し、build.rs には下ろせないため、ゲートはビルド期ではなくテスト期。

**ランタイム検索チェーン**——`find_std_interface_file` に exe 相対検索を一段追加：

1. プロジェクト `.yaoxiang/vendor/std/<name>.yx`（プロジェクトオーバーライド、現状）
2. **exe の置かれたディレクトリ
   `../lib/yaoxiang/std/<name>.yx`（新規）**：ポータブル展開、マネージドインストール（`versions/<ver>/`）、deb 平置きで統一ヒット
3. `~/.yaoxiang/std/<name>.yx`（グローバルフォールバック、手動オーバーライド用として保持）

現状このチェーンはほぼ切断されている（LSP 呼び出しがプロジェクトディレクトリを伝播しない、`package init`
が書き込む `.yaoxiang/std` がチェーンに含まれない）、今回はこれもついでに繋ぎ、`package init`
の出力先を `.yaoxiang/vendor/std` に統一する（パッケージマネージャの vendor ディレクトリと一致）。

**コンパイル権威は変わらない**：`.yx` レイヤは引き続き `include_str!`
で内蔵（RFC-036 の「std バージョンとバイナリの厳密紐付け」不変量を維持）。配布ディレクトリの位置付けは**可読ビュー +
LSP 解析ソース**であり、コンパイル入力ではない——配布ディレクトリ内の `.yx`
をユーザーが手動で変更してもコンパイラには採用されない（Python 流の「`Lib/`
を変更すれば即時反映」セマンティクスを開放するかの議論はオープン問題を参照）。

### Wasm ビルド

**独立を維持し、cargo-dist には移行しない。**

cargo-dist が管理するのは「コンパイラをユーザーに届ける」こと、wasm は「オンライン playground をドキュメントサイトに埋め込む」こと——2 つのまったく異なるデリバリーである。

| 側面         | やり方                                     |
| ------------ | ------------------------------------------ |
| ビルドツール | `wasm-pack build` を維持                   |
| CI workflow  | `_build-wasm.yml` を独立 job として保持    |
| トリガー     | リリースと同じ tag push で、並列の独立 job |
| 公開先       | `docs/public/wasm/` → GitHub Pages         |

### npm 公開

| パッケージ             | 内容                                     | 状態                                                                                                                                                                                                         |
| ---------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `@yaoxiang/cli`        | 配布パッケージをダウンロードする wrapper | 保留：cargo-dist の npm wrapper も同様にフラット成果物前提のため、インストーラと一緒に廃止；npm チャネルが必要なら自作 wrapper（配布パッケージをダウンロード・再構成・展開、展開インストールと同じロジック） |
| `@yaoxiang/playground` | wasm ライブラリ（JS + .wasm）            | 任意、現状は docs のみ公開                                                                                                                                                                                   |

両者は競合せず、名前も競合しない。

### 既存リリースフローの統合

現状の `release.yml`：push main → check-version（`v{version}` tag が存在しなければ通過）→ build /
build-wasm / security / test の 4 系統 → release job（tag 打ち + プッシュ +
`generate-commit-list.mjs` で @mentions を含む body 生成 + 成果物アップロード）。

cargo-dist が生成するパイプラインは tag 駆動で announce/publish を内蔵し、fmt/clippy/test/audit ゲートを含まず、release
notes フォーマットも merge commit
changelog を運べない。**全体を一括置換すると既存リリース儀式を壊す**（PR → CI 全緑 → bump → merge
commit が changelog）。

統合方針：**トリガーとゲートは現状維持、ビルドは `cargo dist build` に委ね、公開は現状維持。**

1. check-version / security / test の 3 job は現状維持（push main トリガー、tag 打ち前ゲート）
2. すべて通過後、release job が `v{version}` tag を作成・プッシュ（現状変更なし）
3. tag push が新規 `dist-release.yml` をトリガー：plan
   job が dist で runner/システム依存マトリクスを計算 → `cargo dist build`（5 ターゲット）→
   `package-dist.sh` でターゲットごとに再構成 → Inno Setup
   job（Windows 再構成パッケージを食べてウィザードをビルド、`/DMyAppVersion=`
   でバージョン注入、二度コンパイルしない）→ `_build-wasm.yml`（並列 job）
4. publish job：`generate-commit-list.mjs` で body 生成（既存スクリプト再利用）→ 再構成パッケージ +
   `.sha256` + `.deb` + wasm + Setup exe のアップロードを追記；独立した `publish-apt` job が GitHub
   Pages apt リポジトリのメタデータを発行（`secrets.APT_GPG_KEY`
   未設定時は自動スキップ、他チャネルに影響なし）

### Nightly 公開

cargo-dist にはネイティブ nightly サポートがない（[axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143)、依然 open
feature request）。

既存の cron + tag 上書き方式を維持し、ビルド部分を `_build-platforms.yml` から `cargo dist build`
に置き換える——これは本質的に cargo コマンドであり、nightly.yml 内で直接呼び出せる。workflow 再利用はしない（想定していた
`uses: ./release.yml` は不可：被利用側に `workflow_call`
トリガーが必要、かつ cargo-dist ワークフローは tag 駆動でビルドと公開が結合している）：

```yaml
# nightly.yml（移行後）
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh（正式版と同じセット）
  publish: # 現状維持：nightly tag を打つ/移動 → GitHub Pre-release を上書き
```

### cargo-dist 設定（dist-workspace.toml として導入済み）

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# dist バージョンを固定（Cargo.toml SemVer シンタックス）
cargo-dist-version = "0.32.0"
ci = "github"
# インストーラはすべて自前、cargo-dist はビルド + 圧縮 + チェックサムのみ
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# 生成されたワークフローは意図的に改変されている（package-dist.sh 再構成 + 自前公開セクション、RFC-037）、
# generate テンプレートからのドリフトでビルド拒否しない
allow-dirty = ["ci"]
```

以上はリポジトリに vendor された実際の設定（`cargo dist init`
生成後に必要に応じて修正）；`cargo-dist-version`
は 0.32.0 に固定、生成されたワークフローはレビューを受ける形でリポジトリに vendor され、実行時にダウンロードしない。ビルドプロファイルは init がルート Cargo.toml の
`[profile.dist]` に注入（inherits release、lto=thin）、両バイナリは `target/<triple>/dist/`
に出力され再構成スクリプトがそこから取得する。

### package-dist.sh（導入済み）

リポジトリの `scripts/release/package-dist.sh` を基準とし、要点：

- 両バイナリは `cargo dist build` のビルド出力ディレクトリ
  `target/<triple>/dist/`（profile=dist）から直接取得——cargo-dist が自前で生成するフラット単一バイナリアーカイブはデリバリーではなく、同名の再構成パッケージを
  `target/distrib/` で直接上書きする
- Z3 共有ライブラリは `.z3/z3-<ver>-<tag>/`（lib → bin 両ディレクトリ探索）から `bin/`
  へコピー；`<tag>` マッピングは `build.rs::detect_target()`
  の配布パッケージ命名に合わせる（2 箇所保守、リスク参照）
- std ディレクトリは純粋コピー：`src/std/interfaces/*.yx`（事前生成インターフェースビュー）+
  `src/std/*.yx`（.yx レイヤ実ソース）
- README/LICENSE を同梱；再パッケージング（Windows zip / その他 tar.gz；Git
  Bash に zip がない場合 zip → System32 bsdtar → PowerShell の 3 段階フォールバック）し `.sha256`
  を再計算
- Linux かつ `dpkg-deb` が利用可能な場合、ついでに `build-deb.sh` を呼び出して `.deb`
  を生成（`/usr/lib/yaoxiang` 平置きツリー + `/usr/bin/yx` シンボリックリンク）

### 廃止する手書き CI

移行完了時に調整されるファイル：

| ファイル                                 | 行数        | 処遇                                                             |
| ---------------------------------------- | ----------- | ---------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254         | 削除（cargo-dist ビルドマトリックスで代替）                      |
| `.github/workflows/release.yml`          | 189         | ゲート + tag 打ちに縮小（ビルド/公開は dist-release.yml へ）     |
| `.github/workflows/nightly.yml`          | 173         | ビルドセクションを `cargo dist build` に置換、公開ロジックは保持 |
| `scripts/build/setup.iss`                | ~250        | **保持して正規化**（Windows ウィザード）                         |
| **合計削減**                             | **~600 行** |                                                                  |

保持：

- `ci.yml`（日常 fmt + clippy + test + MSRV、リリースフローではない）
- `_build-wasm.yml`（独立ビルドフロー、dist-release.yml の並列 job に組み込み）
- `_build-z3-wasm.yml`（wasm 専用 Z3）
- `docs-deploy.yml`（ドキュメントデプロイ）

### 受け入れ基準

「箱から出してすぐに使える」はテスト可能であり、移行完了の判定は「新旧成果物が一致」ではなく、以下すべてが通ることである：

- クリーン環境（Rust / Z3 / `~/.yaoxiang`
  なし）で任意プラットフォームの圧縮パッケージを展開し、`bin/yaoxiang-rs --version`
  を直接実行成功——`LD_LIBRARY_PATH` を設定しない（rpath 有効）
- 展開ディレクトリ内 `lib/yaoxiang/std/*.yx`
  がすべて読める：native モジュールは署名インターフェースビュー、`.yx` レイヤは実ソース
- 展開ディレクトリ下のサンプルプロジェクトで LSP を起動し、std メンバーの補完 / 定義ジャンプが使える（exe 相対検索が機能）
- 公式ガイド通り `/usr/local`（または
  `~/.yaoxiang`）に展開して PATH に追加した後、任意ディレクトリで `yx --version` 成功
- `apt install yaoxiang`（自前リポジトリ）後実行可能、`apt upgrade` でバージョン追従；`/usr/bin/yx`
  シンボリックリンク下でエンジンの `$ORIGIN`（実パス解決）が `bin/libz3.so` をヒット
- `curl ... | sh` と `irm ... | iex` をクリーン環境で実行後、`yx` が実行可能で PATH が通っている
- `yx toolchain install <ver>` / `default` / `update`
  が機能：複数バージョン共存、`yx-toolchain.toml`
  プロジェクト pin がデフォルトバージョンより優先、フロントドアが正しいバージョンにディスパッチ（バージョンツリー内の rpath と std 検索が自己完結、「新エンジンに旧 fmt」の組み合わせなし）
- ポータブル展開後 `yx` が隣接の `yaoxiang-rs` にフォールバック、挙動がマネージドインストールと一致
- Inno Setup インストール後ディレクトリ構造が完全、PATH が通り、アンインストール可能
- Release アセット完備：5 プラットフォームの再構成パッケージ + `.sha256` が実際の内容と一致
- Release body は `generate-commit-list.mjs` の出力（merge commit changelog 完備）
- nightly 成果物は Pre-release、最新の正式 tag に影響しない

## トレードオフ

### 利点

- **箱から出してすぐ使える**
  — ポータブル展開即利用（rpath + 同ディレクトリ共有ライブラリ）、インストーラが完全ディレクトリを敷設
- **標準ライブラリが読める** — ユーザーが Python `Lib/`
  を読むように std を直接読める（必須要件達成）
- **保守コスト削減** — ~600 行の手書きビルド YAML を cargo-dist + ~80 行の自前スクリプトに
- **クロスプラットフォーム一貫性**
  — 全プラットフォーム動的リンク + 同ディレクトリ共有ライブラリ、特例なし
- **インストールの二重化**
  — 標準チャネルは追加コードゼロ（Go/Zig モデル）；簡易チャネルは Rust を参照、apt / curl / iex /
  exe の 4 エントリが同じ成果物構造を共有
- **バージョン管理内蔵** — フロントドア `yx` が rustup/Go
  GOTOOLCHAIN 相当、Python/Node のように事後的に pyenv/nvm/pdm で補填する生態系分裂を回避；ツールのバージョンロックは構造保証であり取り決めではない

### 欠点とリスク

- **簡易チャネルの保守面** — install.sh / install.ps1（ワンクリックスクリプト、ほぼ進化しない）+
  `.deb` と apt リポジトリメタデータ発行（release CI で自動化）+ `yx` フロントドア crate
- **エンジン改名の波及範囲** — `yaoxiang` → `yaoxiang-rs`
  は CI アーティファクト名、Inno、テスト、ドキュメントを一度に一括移行が必要（フェーズ 5 内で完了）
- **学習コスト** — チームが cargo-dist 設定を学ぶ必要
- **cargo-dist 上流リスク** —
  2025 年中頃に Axo 停止に伴い停滞、同年 9 月に原作者が復活し継続的にリリース（0.29 →
  0.32+）；`dist-version` 固定 + 生成物 vendor でレビュー可能にすることで緩和
- **cargo-dist にはネイティブ nightly がない** — nightly 公開部分は手書き継続
- **Z3 ディレクトリ命名の二重保守** — `package-dist.sh` と `build.rs::detect_target()`
  の同期が必要（単一源に収束させるべき）

### RFC-014b との関係

|              | RFC-014b                                    | RFC-037                                      |
| ------------ | ------------------------------------------- | -------------------------------------------- |
| **スコープ** | サードパーティ製パッケージのビルド・配布    | コンパイラ自体のパッケージング・配布         |
| **ツール**   | `yaoxiang build` / `yaoxiang publish`       | `cargo-dist` + 自前スクリプト                |
| **成果物**   | サードパーティ製パッケージの FFI ライブラリ | コンパイラ + 標準ライブラリ + ツールチェーン |
| **相互排他** | いいえ、補完関係                            | いいえ、補完関係                             |

## 代替案

| 案                                       | 採用しない理由                                                                                                                                                                                                 |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **手書き CI を継続**                     | 既に ~870 行手書きしており、重複作業と DLL 漏れリスクがある                                                                                                                                                    |
| **自前でパッケージツールを書く**         | 車輪の再発明をしない、cargo-dist は既に成熟                                                                                                                                                                    |
| **tar.gz のみでインストーラなし**        | 唯一の公式チャネルが展開 + PATH；Inno は Windows ウィザード慣習のためだけにサービス（国内ユーザー向けとして保持裁可）                                                                                          |
| **Docker 配布**                          | コンパイラと言語ツールチェーンはコンテナではなくネイティブバイナリが必要                                                                                                                                       |
| **自前 Homebrew tap**                    | tap は一律コミュニティ保守（homebrew-core）、自前は時期尚早；簡易チャネルの macOS エントリは curl スクリプト                                                                                                   |
| **独立 `yaoxiangup` マネージャバイナリ** | rustup の先例そのまま、可行；しかし 2 つ目のユーザー動詞を生み出し、「パッケージマネージャ/fmt も独立すべきか」の対称性問題を引き起こす——フロントドア/エンジン分離で同時に解消（2026-09-09 議論で否决）        |
| **自己更新のみ、複数バージョンなし**     | 単一バージョンの自己更新では、複数プロジェクトが異なるバージョンを pin するニーズに応えられない；Python/Node は公式バージョン管理を欠くため、生態系は pyenv/nvm/pdm を生み出した——内蔵が裁可済み（2026-09-09） |
| **Z3 全プラットフォーム静的リンク**      | 裁可済み否决——外部システムのディレクトリ型共有ライブラリ配布が自然な形態；exe に埋め込もうが外に置こうが「どちらもパッケージ同梱が必要」という意味では等価で、後者には交換可能性が失われる                     |
| **Inno Setup 廃止**                      | 裁可済み否决——Windows ウィザードとして保持（追加チャネル）                                                                                                                                                     |
| **cargo-dist ネイティブインストーラ**    | フラットバイナリ前提が bin/+lib/ 構造と衝突、インストール直後にライブラリ欠如                                                                                                                                  |
| **自前 WiX/MSI 保守**                    | cargo-dist ジェネレータを手放すと、Inno が既にカバーする価値よりコストが高い                                                                                                                                   |

## 実装戦略

### フェーズ 1：言語側変更（P0）

1. `build.rs`：全プラットフォーム統一動的リンク + rpath link-arg；`copy_dll()` を
   `copy_shared_lib()` に拡張（so/dylib/dll）
2. リポジトリに native インターフェースビューを事前生成（`src/std/interfaces/`）、テストゲートで
   `StdModule::exports()` との同期を強制（gen-std サブコマンドは裁可済み取消）
3. `find_std_interface_file` に exe 相対検索分岐を追加；`package init` の出力パスを
   `.yaoxiang/vendor/std` に統一

### フェーズ 2：cargo-dist 統合（P0）

1. `cargo dist init` を実行して初期設定を生成（`installers = []`、dist-version 固定）
2. `package-dist.sh` を記述（再構成 + .yx ソースコピー + チェックサム再計算）
3. `dist-release.yml` を新規作成（tag 駆動：dist build → 再構成 →
   wasm 並列 → 自前 publish）；`release.yml` をゲート + tag 打ちに縮小
4. 新旧パイプラインを並行実行し、受け入れ基準で逐次検証

### フェーズ 3：旧 CI 廃止（P1）

1. 問題なければ `_build-platforms.yml` を削除
2. `nightly.yml` のビルドセクションを `cargo dist build` に置換
3. `setup.iss`
   を新成果物構造に接続（Inno を正規化；バージョン番号は Cargo.toml から注入、sed 置換を撲滅）

### フェーズ 4：簡易チャネル（P2）

1. `install.sh` / `install.ps1`（プラットフォーム検出 → 最新の再構成パッケージをダウンロード →
   `versions/` に展開 → `bin/yx` をインストールルートに配置 → `settings.toml`
   にデフォルトバージョンを書き込み →
   PATH 提示/書き込み；フェーズ 5 と同時に着地、最終状態へ一歩で到達）
2. `.deb` パッケージング（`package-dist.sh` と同じディレクトリツリー + `/usr/bin`
   シンボリックリンクを再利用）+ GitHub Pages 静的 apt リポジトリ（メタデータ GPG 署名、release
   CI から発行）

### フェーズ 5：フロントドア yx とエンジン改名（P2）

1. 現モノリスを `yaoxiang-rs`
   に改名（CI アーティファクト名、Inno、テスト、ドキュメントを全件通し、ワンショット移行）
2. フロントドア小型 crate `yx`
   を新規追加（workspace メンバー）：バージョン解決、Release アーティファクトのダウンロード、tar/zip 解凍、settings.toml、ディスパッチ（toolchain/self 動詞のみ保持、他は透過転送、clap を使わず手書きディスパッチで引数逐語転送を保証）；バージョンインデックスは GitHub
   Releases latest API から取得（ミラーソースは ghproxy 風プレフィックス連結）；`yx`
   コマンド名は衝突チェック済み（主要ディストリビューション/Homebrew に同名の常用コマンドなし、ごく一部のニッチツールが yx を別名として使用）
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml`
   プロジェクト pin + ミラーソース + `yx self update`
4. ブートストラップスクリプト：`curl | sh` / `irm | iex`
   → 再構成パッケージをダウンロードしフロントドアとデフォルト stable を一度にインストール（フェーズ 4 と同時に最終状態として着地）

### フェーズ 6：任意フォローアップ（すべて非ブロッキング）

1. winget 提出（Inno exe を指す、コミュニティ保守、homebrew-core モデルに対称）
2. `.rpm`（dnf ユーザー、`.deb` と同型）
3. Homebrew：homebrew-core 参入基準に達したらコミュニティが提出
4. npm `@yaoxiang/cli` 自作 wrapper（名称は現時点で未登録）

## オープン問題

### 未決

- **.yx レイヤで「配布ディレクトリを手動変更すればコンパイル採用される」Python 風セマンティクスを提供するか？**
  デフォルトは否——コンパイル権威は RFC-036 の内蔵を維持（std バージョンとバイナリの厳密紐付け）、配布ディレクトリの位置付けは可読ビュー +
  LSP 解析ソース。将来開放する場合、バージョン紐付け不変量を再検討する必要がある。

### クローズ済み

以下の問題は設計議論で解決済み：

- ~~Windows での Z3 静的リンク可行性？~~ →
  **静的リンクは行わず、全プラットフォーム動的**（2026-09-09 再確認の上で維持）
- ~~gen-std-interfaces サブコマンド命名？~~ →
  **サブコマンドは設けない**（2026-09-10 裁可：通常のパッケージングが形になった後、サブコマンド面は余分；native インターフェースビューはリポジトリ事前生成
  `src/std/interfaces/` + テストゲート同期に変更、パッケージングは純粋コピー）
- ~~Inno Setup を保持するか？~~ → **Windows ウィザードとして保持（追加チャネル）**
- ~~配布パッケージ構造は標準ライブラリソースを物理同梱するか？~~ → **必須**（ユーザー可読性が Python
  `Lib/` に揃う、2026-09-09 裁可）
- ~~cargo-dist ネイティブインストーラ（shell/powershell/homebrew/msi/npm）？~~ →
  **すべて廃止**、インストーラは自前（フラット前提が bin/+lib/ と衝突）
- ~~インストーラ戦略？~~ →
  **二重化**（2026-09-09 最終裁可）：標準チャネル＝配布パッケージ＝プロダクト（Go/Zig モデル、展開 +
  PATH）；簡易チャネルは Rust を参照しワンライナーインストール——Linux `apt`（自前 deb リポジトリ）/
  `curl | sh`、Windows `irm | iex` / Inno
  exe。行わない：MSI、cargo-dist ネイティブインストーラ、自前 brew tap
- ~~バージョン管理を組み込むか？~~ →
  **必須、インストーラ体系の一部**（2026-09-09 裁可、同日早些时候の「远期独立 RFC」境界を推翻）：動機は Python/Node が公式バージョン管理を欠くため pyenv/nvm/pdm で補填する生態系分裂
- ~~バージョン管理形態：独立バイナリかサブコマンドか？~~ →
  **フロントドア/エンジン分離**（2026-09-09 最終裁可、3 回の収斂 A→C→命名反転）：常用コマンド
  `yx`＝フロントドア（小型バイナリ、toolchain/self 動詞内蔵）、エンジン `yaoxiang-rs`＝現 `yaoxiang`
  モノリスの改名；ディレクトリ構造
  `versions/<ver>/`——バージョンは第一級の概念、バージョンディレクトリは配布パッケージの展開ルートそのもの、ツールはバージョンと一体でロック（旧 fmt は新構文を認識しない；かつて想定した内層
  `toolchains/` は冗長のため取消）。先例：Go `go` フロントドア +
  GOTOOLCHAIN、rustup プロキシディスパッチ
- ~~cargo-dist extra-artifacts 条件実行？~~ → **`package-dist.sh` スクリプトで処理、shell
  case 分岐で**
- ~~標準ライブラリインターフェースバージョン互換性？~~ →
  **コンパイラのバージョンと同じパッケージで配布**

## 参考文献

- [cargo-dist 公式ドキュメント](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: ビルドシステムとバイナリ配布](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 ビルド設定 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)
